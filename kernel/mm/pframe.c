#include "globals.h"
#include "config.h"
#include "errno.h"

#include "proc/proc.h"

#include "util/debug.h"
#include "util/string.h"

#include "mm/mmobj.h"
#include "mm/page.h"
#include "mm/slab.h"
#include "mm/kmalloc.h"
#include "mm/pframe.h"
#include "mm/tlb.h"
#include "mm/pagetable.h"

#include "vm/vmmap.h"

/*
 * In this file, physical pages (as represented by pframes) will be
 * referred to as "pages"
 * A particular mmobj and a page number within the mmobj constitute
 * the "identity" of a page, and uniquely identify a page.
 *
 * A page is always in one of three categories:
 *     - (1) free
 *     - (2) allocated
 *     - (3) pinned
 *
 * (1) Free pages do not contain identifiable data and are readily
 *     available for use. They are not pre-zeroed (at the moment, at least
 *     (but often times, OSs will have their equivalent of Weenix's
 *     idleproc (or another thread/process dedicated to this purpose) zero
 *     unzeroed pages on the free list when the system is otherwise idle)).
 *
 * (2) Allocated pages contain identifiable data.
 *
 * (3) Pinned pages contain identifiable data but differ from allocated
 *     pages in that the data they contain is "pinned" to the page frame in
 *     which they reside; pinned pages cannot be reclaimed nor can they be
 *     cleaned. Every pinned page is, of course, allocated also.
 *
 * For example, file system cache pages are typically allocated, but not pinned,
 * because if we needed to claim the page frame they're using, we could write
 * the data out to disk and use that page frame.
 *
 * By contrast, pages used by anonymous mappings are pinned because they can't
 * be paged out - there's no other copy of the data they contain.
 *
 *
 * When a page is allocated or pinned:
 *     - pf_link links the page into allocated_list or pinned_list,
 *       respectively
 *     - pf_hlink links the page into the appropriate hash chain of the
 *       resident page hashtable
 *     - pf_olink links the page into the appropriate mmobj's list of
 *       resident pages
 *
 * When a page is free:
 *     - pf_link links the page into free_list
 *     - pf_hlink does not link the page into any list
 *     - pf_olink does not link the page into any list
 */

/* Page management structures:
 *   Paging lists/queues:
 *     The PINNED list:
 *       Pages on this list have been "pinned" to the page frames in which
 *       they currently reside-- these page frames cannot be reclaimed nor
 *       can the pages be cleaned until they are completely unpinned.
 *       There is no need to keep this list in any order.
 */
static int npinned;
static list_t pinned_list;

/*     The ALLOCATED list: */
/*       Pages on this list contain useful/actual/real data. This list is
 *       maintained in least-recently-requested (via pframe_get or
 *       pframe_get_resident) (and thus, *roughly/approximately* LRU) order.
 */
static int nallocated;
static list_t alloc_list;

static slab_allocator_t *pframe_allocator;

/* Used to quickly look up pframes. ALL pages "owned by" some
 * mmobj should be in this hash
 * (object, pagenum) --> list of pframes */
#define hash_page(obj, pagenum)  ((((uint32_t)(obj)) + (pagenum)) \
                                  % PF_HASH_SIZE)
static list_t pframe_hash[PF_HASH_SIZE];

/* Related to the Pageout daemon: */

static uint32_t nfreepages_min = 0;
static uint32_t nfreepages_target = 0;

/*   pageoutd sleeps on this queue */
static proc_t *pageoutd = NULL;
static kthread_t *pageoutd_thr = NULL;
static ktqueue_t pageoutd_waitq;

/* threads waiting for pageoutd to run sleep on this queue */
static ktqueue_t alloc_waitq;

/* Pageout daemon functions */
static void *pageoutd_run(int arg1, void *arg2);
static void pageoutd_exit(void);
#define pageoutd_wakeup()        (sched_broadcast_on(&pageoutd_waitq))
#define pageoutd_needed()        \
	((page_free_count() <= nfreepages_min) && (!list_empty(&alloc_list)))
#define pageoutd_target_met()    (page_free_count() >= nfreepages_target)


/*
 * Initialize the pinned and allocated counts and lists. Then, make a pframe
 * slab allocator. You should also list_init all the lists that make
 * up the pframe_hash. Finally, you need to set things up for pageoutd to
 * run by setting nfreepages_min and nfreepages_target.
 */
void
pframe_init(void)
{
        /* initialize page lists: */
        npinned = 0;
        list_init(&pinned_list);
        nallocated = 0;
        list_init(&alloc_list);

        pframe_allocator = slab_allocator_create("pframe", sizeof(pframe_t));
        KASSERT(NULL != pframe_allocator);

        /* initialize pframe_hash: */
        int i;
        for (i = 0; i < PF_HASH_SIZE; ++i)
                list_init(&pframe_hash[i]);

        /* initialize pageout parameters: */
        nfreepages_target = page_free_count() >> 1;
        nfreepages_min = 0;

		/* initialize alloc_waitq */
		sched_queue_init(&alloc_waitq);
}

void
pframe_shutdown()
{
        KASSERT(PID_IDLE == curproc->p_pid); /* Should call from idleproc */

        /* Stop pageoutd and wait for it */
        pageoutd_exit();

        int pid = pageoutd->p_pid;
        int child = do_waitpid(-1, 0, NULL);
        KASSERT(pid == child && "waited on process other than pageoutd");
        KASSERT(0 == npinned && "WARNING: FOUND PINNED "
                "PAGES!!!!!!!!!! SOMETHING IS BROKEN!!\n");

        /* Clean all pages (sync with secondary storage) */
        pframe_clean_all();

        /* Free all pages */
        pframe_t *pf;
        list_iterate_begin(&alloc_list, pf, pframe_t, pf_link) {
                KASSERT(!pframe_is_dirty(pf));
                KASSERT(!pframe_is_busy(pf));
                KASSERT(!pframe_is_pinned(pf));
                pframe_free(pf);
        } list_iterate_end();
}

/*
 * Obtain the (unique) page identified by 'o' and 'pagenum' only if this page is
 * already resident; if this page is not already resident, NULL is
 * returned. This routine will not block.
 *
 * Note that this function may return a busy page. The caller must check this
 * case and deal with it appropriately. When a page is busy, it is being
 * sync'ed, filled, or reclaimed. A page may be sync'ed by pageoutd or an
 * arbitrary thread (which might free the page afterward).
 *
 * @param o the mmobj the page is in
 * @param pagenum the page number identifying this page within the object
 *
 * @return the page requested, or NULL if it is not resident.
 */
pframe_t *
pframe_get_resident(struct mmobj *o, uint32_t pagenum)
{
        list_t *hashchain;
        pframe_t *pf;

        hashchain = &pframe_hash[hash_page(o, pagenum)];
        list_iterate_begin(hashchain, pf, pframe_t, pf_hlink) {
                if ((o == pf->pf_obj) && (pagenum == pf->pf_pagenum)) {
                        /* found a page with the specified identity. It is
                         * up to the caller to recognize/care if the page
                         * is busy. */
                        if (!pframe_is_pinned(pf)) {
                                /* send to back of alloc_list */
                                list_remove(&pf->pf_link);
                                list_insert_tail(&alloc_list, &pf->pf_link);
                        }
                        return pf;
                }
        } list_iterate_end();

        return NULL;
}

/*
 * Allocate a pframe to hold the page identified by the object and page number.
 * The given page should not already be resident.
 *
 * We allocate a page from the free list. We then initialize the newly allocated
 * page's object, pagenum, and flags, pin count, and links. We also update the
 * object's nrespages.
 *
 * @param o the mmobj identifying this page
 * @param pagenum the page number of this page in the object
 *
 * @return a new pframe
 */
static pframe_t *
pframe_alloc(mmobj_t *o, uint32_t pagenum)
{
        pframe_t *pf;
        if (NULL == (pf = slab_obj_alloc(pframe_allocator))) {
                dbg(DBG_PFRAME, "WARNING: not enough kernel memory\n");
                return NULL;
        }
        if (NULL == (pf->pf_addr = page_alloc())) {
                dbg(DBG_PFRAME, "WARNING: not enough kernel memory\n");
                slab_obj_free(pframe_allocator, pf);
                return NULL;
        }

        nallocated++;
        list_insert_tail(&alloc_list, &pf->pf_link);

        pf->pf_obj = o;
        pf->pf_pagenum = pagenum;
        pf->pf_flags = 0;
        sched_queue_init(&pf->pf_waitq);
        pf->pf_pincount = 0;

        list_insert_head(&pframe_hash[hash_page(o, pagenum)], &pf->pf_hlink);

        o->mmo_ops->ref(o);
        o->mmo_nrespages++;
        list_insert_head(&o->mmo_respages, &pf->pf_olink);

        return pf;
}

/*
 * Fills the contents of the page (using the mmobj's fillpage op).
 * Make sure to mark the page busy while it's being filled.
 * @param pf the page to fill
 */
static int
pframe_fill(pframe_t *pf)
{
        int ret;

        pframe_set_busy(pf);
        ret = pf->pf_obj->mmo_ops->fillpage(pf->pf_obj, pf);
        pframe_clear_busy(pf);

        sched_broadcast_on(&pf->pf_waitq);

        return ret;
}

/*
 * Find and return the pframe representing the page identified by the object
 * and page number. If the page is already resident in memory, then we return
 * the existing page. Otherwise, we allocate a new page and fill it (in which
 * case this routine may block). After allocating the new pframe, we check to
 * see if we need to call pageoutd and wake it up if necessary.
 *
 * If the page is found (resident) but busy, then we will wait for it to become
 * unbusy and then try again (since it may have been freed after that). Thus,
 * as long as this routine returns successfully, the returned page will be a
 * non-busy page that will be guaranteed to remain resident until the calling
 * context blocks without first pinning the page.
 *
 * This routine may block at the mmobj operation level.
 *
 * @param o the parent object of the page
 * @param pagenum the page number of this page in the object
 * @param result used to return the pframe (NULL if there's an error)
 * @return 0 on success, < 0 on failure.
 */
int
pframe_get(struct mmobj *o, uint32_t pagenum, pframe_t **result)
{
        NOT_YET_IMPLEMENTED("S5FS: pframe_get");
        return 0;
}

int
pframe_lookup(struct mmobj *o, uint32_t pagenum, int forwrite, pframe_t **result)
{
        KASSERT(NULL != o);
        KASSERT(NULL != result);

        return o->mmo_ops->lookuppage(o, pagenum, forwrite, result);
}

/*
 * Migrate a page frame up the tree. The destination must be on the same
 * branch as the pframe's current object. pf must not be busy. If dest
 * already has a page with the same number as pf clean pf.
 *
 * @param pf page to be migrated
 * @param dest destination vm object
 */
void
pframe_migrate(pframe_t *pf, mmobj_t *dest)
{
        KASSERT(!pframe_is_busy(pf));
        if (NULL != pframe_get_resident(dest, pf->pf_pagenum)) {
                /* dest already has a newer version of the page, clean this page */
                pframe_unpin(pf);
                pframe_clean(pf);
                pframe_free(pf);
        } else {
                mmobj_t *src = pf->pf_obj;
                pf->pf_obj = dest;
                list_remove(&pf->pf_hlink);
                list_remove(&pf->pf_olink);
                src->mmo_nrespages--;
                src->mmo_ops->put(src);
                list_insert_head(&pframe_hash[hash_page(dest, pf->pf_pagenum)], &pf->pf_hlink);
                list_insert_head(&dest->mmo_respages, &pf->pf_olink);
                dest->mmo_nrespages++;
                dest->mmo_ops->ref(dest);
        }
}

/*
 * Increases the pin count on this page. Pages with a pin count > 0 will not be
 * paged out by pageoutd, so this ensures that the page will remain resident
 * until the pin count is decreased.
 *
 * If the pframe has not yet been pinned, remove this pframe's list link from
 * the allocated list and add it to the pinned list.  Be sure to decrement
 * nallocated and increment npinned.
 *
 * In either case, increment the pf_pincount.
 *
 * @param pf the page to pin
 */
void
pframe_pin(pframe_t *pf)
{
        NOT_YET_IMPLEMENTED("S5FS: pframe_pin");
}

/*
 * Decreases the pin count on a page. If the pin count reaches zero, then the
 * page could be paged out any time after the calling context blocks.
 *
 * If the pin count reaches zero, move the pframe's list link from the pinned
 * list to the allocated list.  Be sure to correctly update npinned and
 * nallocated
 *
 * @param pf a pinned page (a page with a positive pin count)
 */
void
pframe_unpin(pframe_t *pf)
{
        NOT_YET_IMPLEMENTED("S5FS: pframe_unpin");
}

/*
 * Indicates that a page is about to be modified. This should be called on a
 * page before any attempt to modify its contents. This marks the page dirty
 * (so that pageoutd knows to clean it before reclaiming the page frame)
 * and calls the dirtypage mmobj entry point.
 * The given page must not be busy.
 *
 * This routine can block at the mmobj operation level.
 *
 * @param pf the page to dirty
 * @return 0 on success, -errno on failure
 */
int
pframe_dirty(pframe_t *pf)
{
        int ret;

        KASSERT(!pframe_is_busy(pf));

        pframe_set_busy(pf);

        if (!(ret = pf->pf_obj->mmo_ops->dirtypage(pf->pf_obj, pf))) {
                pframe_set_dirty(pf);
        }
        pframe_clear_busy(pf);
        sched_broadcast_on(&pf->pf_waitq);

        return ret;
}

/*
 * Clean a dirty page by writing it back to disk. Removes the dirty
 * bit of the page and updates the MMU entry.
 * The page must be dirty but unpinned.
 *
 * This routine can block at the mmobj operation level.
 * @param pf the page to clean
 * @return 0 on success, -errno on failure
 */
int
pframe_clean(pframe_t *pf)
{
        int ret;

        KASSERT(pframe_is_dirty(pf) && "Cleaning page that isn't dirty!");
        KASSERT(pf->pf_pincount == 0 && "Cleaning a pinned page!");

        dbg(DBG_PFRAME, "cleaning page %d of obj %p\n", pf->pf_pagenum, pf->pf_obj);

        /*
         * Clear the dirty bit *before* we potentially (depending on this
         * particular object type's 'dirtypage' implementation) block so
         * that if the page is dirtied again while we're writing it out,
         * we won't (incorrectly) think the page has been fully cleaned.
         */
        pframe_clear_dirty(pf);

        /* Make sure a future write to the page will fault (and hence dirty it) */
        tlb_flush((uintptr_t) pf->pf_addr);
        pframe_remove_from_pts(pf);

        pframe_set_busy(pf);
        if ((ret = pf->pf_obj->mmo_ops->cleanpage(pf->pf_obj, pf)) < 0) {
                pframe_set_dirty(pf);
        }
        pframe_clear_busy(pf);
        sched_broadcast_on(&pf->pf_waitq);

        return ret;
}

/*
 * Deallocates a pframe (reclaims the page frame for use by something else).
 * The page should not be pinned, free, or busy. Note that if the page is dirty
 * it will not be cleaned. This removes the page's reference to its mmobj.
 *
 * This routine may block in the mmobj put operation.
 * @param pf the page to free
 */
void
pframe_free(pframe_t *pf)
{
        KASSERT(!pframe_is_pinned(pf));
        KASSERT(!pframe_is_free(pf));
        KASSERT(!pframe_is_busy(pf));

        dbg(DBG_PFRAME, "uncaching page %d of obj %p\n", pf->pf_pagenum, pf->pf_obj);

        mmobj_t *o = pf->pf_obj;


        /* Flush the TLB */
        tlb_flush((uintptr_t) pf->pf_addr);
        /* Remove from all pagetables that map it */
        pframe_remove_from_pts(pf);

        list_remove(&pf->pf_hlink);

        pf->pf_obj = NULL;
        nallocated--;
        list_remove(&pf->pf_link);

        page_free(pf->pf_addr);
        slab_obj_free(pframe_allocator, pf);

        o->mmo_nrespages--;
        list_remove(&pf->pf_olink);

        /* Now that pf has effectively been freed, dereference the corresponding
         * object. We don't do this earlier as we are modifying the object's counts
         * and also because this op can block */
        o->mmo_ops->put(o);
}

/*
 * Clean all allocated pages (that is, all pages that are not pinned and
 * not free). This is called by sync(2).
 */
void
pframe_clean_all()
{
        pframe_t *pf;
        dbg(DBG_PFRAME, "pframe_clean_all: starting (this may take a while)\n");

        /*
         * Iterate from head of alloc_list to tail; This is a rough attempt to
         * sync from least active to most active. Note that every time we block we
         * need to start the loop over as the "current element" pf may have been
         * moved or removed in the meantime (our list has no multithreaded
         * integrity)
         */
list_start:
        list_iterate_begin(&alloc_list, pf, pframe_t, pf_link) {
                KASSERT(!pframe_is_pinned(pf));
                KASSERT(!pframe_is_free(pf));
                if (pframe_is_busy(pf)) {
                        sched_sleep_on(&pf->pf_waitq);
                        goto list_start;
                }
                if (pframe_is_dirty(pf)) {
                        pframe_clean(pf);
                        goto list_start;
                }
        } list_iterate_end();

        /* In theory, this function might never terminate (if new pages are
         * constantly being added at the same time). That's why the user shouldn't
         * call sync(2) very much... */
        dbg(DBG_PFRAME, "pframe_clean_all: completed!\n");
}

/* Remove a page frame from the page tables of all processes that map it
 * To do that, traverse all processes that map the given page frame into
 * their address space, and zero the corresponding address entry.
 */
void
pframe_remove_from_pts(pframe_t *pf)
{
        vmarea_t *vma;
        list_iterate_begin(mmobj_bottom_vmas(pf->pf_obj), vma, vmarea_t, vma_olink) {
                /* Get the virtual address in the area corresponding to this pf */
                if ((pf->pf_pagenum >= vma->vma_off)
                    && (pf->pf_pagenum < vma->vma_off + (vma->vma_end - vma->vma_start))) {
                        uintptr_t vaddr = (uintptr_t) PN_TO_ADDR(vma->vma_start + pf->pf_pagenum - vma->vma_off);
                        /* And unmap it from that area's proc */
                        if (NULL != vma->vma_vmmap->vmm_proc) {
                                pt_unmap(vma->vma_vmmap->vmm_proc->p_pagedir, vaddr);
                        }
                }

        } list_iterate_end();
}

/* ------------------------------------------------------------------ */
/* ------------------------- PAGEOUT DAEMON ------------------------- */
/* ------------------------------------------------------------------ */

/*
 * Initialize the pageout daemon process. This function is called
 * by pframe_init and simply starts up a new thread with the
 * appropriate function as its first call.
 */
static __attribute__((unused)) void
pageoutd_init(void)
{
        /* initialize pageoutd_waitq: */
        sched_queue_init(&pageoutd_waitq);

        /* create and schedule pageoutd: */
        KASSERT(curproc && (PID_IDLE == curproc->p_pid)
                && "should be calling this from idleproc");
        pageoutd = proc_create("pageoutd");
        KASSERT(NULL != pageoutd);
        pageoutd_thr = kthread_create(pageoutd, pageoutd_run, 0, NULL);
        KASSERT(NULL != pageoutd_thr);

        sched_make_runnable(pageoutd_thr);
}
init_func(pageoutd_init);
init_depends(sched_init);

/*
 * Just cancel pageoutd
 */
static void
pageoutd_exit()
{
        KASSERT(NULL != pageoutd_thr);
        kthread_cancel(pageoutd_thr, (void *) 0);
        pageoutd_thr = NULL;
}

/*
 * The pageout daemon, when run, gets the least-recently-requested page from the
 * list of pages which are available to be paged out. Make sure to check if the
 * page is busy before yanking it. If the page you select is dirty, make sure
 * to clean it before yanking it. Finally, go back to sleep after having paged
 * out the appropriate page.
 * Both arguments unused.
 */
static void *
pageoutd_run(int arg1, void *arg2)
{
        while (1) {
                KASSERT(nallocated >= 0);
                while ((!pageoutd_target_met()) && (!list_empty(&alloc_list))) {
                        pframe_t *pf;

                        /* obtain least-recently-requested page: */
                        pf = list_head(&alloc_list, pframe_t, pf_link);

                        if (pframe_is_busy(pf)) {
                                sched_sleep_on(&pf->pf_waitq);
                        } else if (pframe_is_dirty(pf)) {
                                pframe_clean(pf);
                        } else {
                                /* it's not busy, it's clean, and it's
                                 * least-recently-requested; reclaim it: */
                                pframe_free(pf);
                        }
                }

                /*   release the thundering herd... */
                sched_broadcast_on(&alloc_waitq);

                dbg(DBG_PFRAME, "PAGEOUT DEMAON: Falling asleep\n");
                dbg(DBG_PFRAME, "PAGEOUT DEMAON: "
                    "nfreepages_target=|%d| "
					"nfreepages_min=|%d| "
					"page_free_count=|%d|\n", nfreepages_target, nfreepages_min, page_free_count());
                if (sched_cancellable_sleep_on(&pageoutd_waitq))
                        kthread_exit((void *)0);
                dbg(DBG_PFRAME, "PAGEOUT DEMAON: Waking up\n");
                dbg(DBG_PFRAME, "PAGEOUT DEMAON: "
                    "nfreepages_target=|%d| "
					"nfreepages_min=|%d| "
					"page_free_count=|%d|\n", nfreepages_target, nfreepages_min, page_free_count());
        }
        return NULL;
}
