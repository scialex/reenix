/*
 *  FILE: vnode.c
 *  AUTH: mcc | mahrens | kma | afenn
 *  DESC: vnode management
 *  $Id: vnode.c,v 1.6.2.2 2006/06/04 01:02:32 afenn Exp $
 */

#include "kernel.h"
#include "util/init.h"
#include "util/string.h"
#include "util/printf.h"
#include "errno.h"
#include "fs/stat.h"
#include "fs/vfs.h"
#include "fs/vnode.h"
#include "mm/slab.h"
#include "proc/sched.h"
#include "util/debug.h"
#include "vm/vmmap.h"
#include "globals.h"

static slab_allocator_t *vnode_allocator;

static list_t vnode_inuse_list;

/* Related to vnodes representing special files: */
static void init_special_vnode(vnode_t *vn);
static int special_file_read(vnode_t *file, off_t offset, void *buf, size_t count);
static int special_file_write(vnode_t *file, off_t offset, const void *buf, size_t count);
static int special_file_mmap(vnode_t *file, vmarea_t *vma, mmobj_t **ret);
static int special_file_stat(vnode_t *vnode, struct stat *ss);
static int special_file_fillpage(vnode_t *file, off_t offset, void *pagebuf);
static int special_file_dirtypage(vnode_t *file, off_t offset);
static int special_file_cleanpage(vnode_t *file, off_t offset, void *pagebuf);
/* mmobj_t entry points: */
static void vo_vref(mmobj_t *o);
static void vo_vput(mmobj_t *o);

static int  vlookuppage(mmobj_t *o, uint32_t pagenum, int forwrite, pframe_t **pf);
static int  vreadpage(mmobj_t *o, pframe_t *pf);
static int  vdirtypage(mmobj_t *o, pframe_t *pf);
static int  vcleanpage(mmobj_t *o, pframe_t *pf);

static mmobj_ops_t vnode_mmobj_ops = {
        .ref = vo_vref,
        .put = vo_vput,
        .lookuppage = vlookuppage,
        .fillpage = vreadpage,
        .dirtypage = vdirtypage,
        .cleanpage = vcleanpage
};

/* vnode operations tables for special files: */
static vnode_ops_t bytedev_spec_vops = {
        .read = special_file_read,
        .write = special_file_write,
        .mmap = special_file_mmap,
        .create = NULL,
        .mknod = NULL,
        .lookup = NULL,
        .link = NULL,
        .unlink = NULL,
        .mkdir = NULL,
        .rmdir = NULL,
        .readdir = NULL,
        .stat = special_file_stat,
        .fillpage = special_file_fillpage,
        .dirtypage = special_file_dirtypage,
        .cleanpage = special_file_cleanpage
};

static vnode_ops_t blockdev_spec_vops = {
        .read = NULL,
        .write = NULL,
        .mmap = NULL,
        .create = NULL,
        .mknod = NULL,
        .lookup = NULL,
        .link = NULL,
        .unlink = NULL,
        .mkdir = NULL,
        .rmdir = NULL,
        .readdir = NULL,
        .stat = special_file_stat,
        .fillpage = NULL,
        .dirtypage = NULL,
        .cleanpage = NULL
};

/*
 * Initialization:
 */
static __attribute__((unused)) void
vnode_init(void)
{
        list_init(&vnode_inuse_list);
        vnode_allocator = slab_allocator_create("vnode", sizeof(vnode_t));
}
init_func(vnode_init);

/*
 * Core vnode management routines:
 */
void
vref(vnode_t *vn)
{
        KASSERT(vn);
        KASSERT(0 < vn->vn_refcount);
        vn->vn_refcount++;
        dbg(DBG_VNREF, "vref: 0x%p, 0x%p ino %ld up to %d, nrespages=%d\n",
            vn, vn->vn_fs, (long)vn->vn_vno, vn->vn_refcount, vn->vn_nrespages);
}

vnode_t *
vget(struct fs *fs, ino_t vno)
{
        vnode_t *vn = NULL;

        KASSERT(fs);

        /* look for inuse vnode */
find:
        list_iterate_begin(&vnode_inuse_list, vn, vnode_t, vn_link) {
                if ((vn->vn_fs == fs) && (vn->vn_vno == vno)) {
                        /* found it... */
                        if (VN_BUSY & vn->vn_flags) {
                                /* it's either being brought in or it's on
                                 * its way out. Let's not race whomever is
                                 * doing this. */

                                dbg(DBG_VNREF, "vget: wow, found vnode busy (0x%p, 0x%p ino %ld refcount %d)\n",
                                    vn, vn->vn_fs, (long)vn->vn_vno, vn->vn_refcount);

                                sched_sleep_on(&vn->vn_waitq);
                                goto find;
                        }

#ifndef __MOUNTING__
                        /* If we are implementing mountpoint support
                           then we should get the mounted vnode,
                           not the requested one (if none is
                           mounted then vn->vn_mount should
                           point back to vn) */
                        vref(vn);
                        return vn;
#else
                        vref(vn->vn_mount);
                        return vn->vn_mount;
#endif
                }
        } list_iterate_end();

        /* if we got here, we didn't find the vnode. */
        /*   alloc a new vnode: */
        vn = slab_obj_alloc(vnode_allocator);
        if (!vn) {
                dbg(DBG_VNREF, "vget: kmem has been exhausted. "
                    "will then re-attempt to vget vnode later %d of fs %p\n", vno, fs);
                sched_make_runnable(curthr);
                sched_switch();
                goto find;
        }
        memset(vn, 0, sizeof(vnode_t));
        /*   initialize its contents: */
        /*     members that can be initialized here: */
        vn->vn_fs = fs;
        vn->vn_vno = vno;
        kmutex_init(&vn->vn_mutex);
        mmobj_init(&vn->vn_mmobj, &vnode_mmobj_ops);
        sched_queue_init(&vn->vn_waitq);

#ifdef __MOUNTING__
        vn->vn_mount = vn;
#endif

        /*     use 'read_vnode' to ask underlying fs for initial values of
         *     vn_mode, vn_len, vn_i, and vn_devid (if
         *     appropriate)): */

        /*       mark it busy and place it on vnode_inuse_list (so it can
         *       be found while we are possibly blocking): (also, seems
         *       appropriate not to ref it yet since no references from
         *       outside this context (vnode.c) will exist until we are
         *       done bringing the vnode in)
         */
        vn->vn_flags |= VN_BUSY;
        list_insert_head(&vnode_inuse_list, &vn->vn_link);

        KASSERT(vn->vn_fs->fs_op && vn->vn_fs->fs_op->read_vnode);
        /*       this is where we might block (depending on the underlying
         *       fs): */
        vn->vn_fs->fs_op->read_vnode(vn);

        vn->vn_flags &= ~VN_BUSY;

        /*     for special files: */
        if (S_ISCHR(vn->vn_mode) || S_ISBLK(vn->vn_mode))
                init_special_vnode(vn);

        vn->vn_refcount = 1;

        return vn;
}

/*
 * - decrement vn->vn_refcount
 * - if it is zero
 *     - (vn->vn_nrespages should also be zero)
 *     - free the vnode
 *
 * - (otherwise it is > zero)
 *
 * - if it is now equal to vn->vn_nrespages and query_vnode returns zero
 *     - (the vnode is not linked in the underlying fs, and no active
 *       references to it exist --> all of its pages can be uncached, etc.)
 *     - add an artificial refcount so we don't spuriously hit this case
 *       in the upcoming steps
 *     - for each page in vn->vn_respages
 *         - pframe_free(page)
 *             - (note: this will decrement nrespages before dropping a
 *               reference to the object containing nrespages, so weknow
 *               that nrespages will be decremented before refcount (or,
 *               equivalently, before vput is called on vn))
 *     - (assert: refcount should be 1, nrespages should be zero, linkcount
 *       should be zero)
 *     - free the vnode
 */
void
vput(struct vnode *vn)
{
        KASSERT(vn);

        KASSERT(0 <= vn->vn_nrespages);
        KASSERT(vn->vn_nrespages < vn->vn_refcount);

        KASSERT(!(VN_BUSY & vn->vn_flags));


        dbg(DBG_VNREF, "vput: 0x%p, 0x%p ino %ld, down to %d, nrespages = %d\n",
            vn, vn->vn_fs, (long)vn->vn_vno, vn->vn_refcount - 1, vn->vn_nrespages);

        if ((vn->vn_nrespages == (vn->vn_refcount - 1))
            && !vn->vn_fs->fs_op->query_vnode(vn)) {
                pframe_t *vp;
                /* vn is becoming passively-referenced, and the linkcount
                 * is zero, so there is no way for it to become
                 * actively-referenced ever again, and thus there is no
                 * point in keeping it or any cached pages of it around.
                 */
                list_iterate_begin(&vn->vn_mmobj.mmo_respages, vp, pframe_t,
                                   pf_olink) {
                        /*  (dbounov):
                         * Wait for the page to become not busy.
                         * At this point the only people who can be accessing the
                         * page are pframe_sync and the pageoutd (the shadowd does
                         * not touch non-anonymous objects). Both of them should
                         * definately free the page, if they have it busy.
                         */
                        while (pframe_is_busy(vp))
                                sched_sleep_on(&(vp->pf_waitq));
                        pframe_free(vp);
                } list_iterate_end();

                /* at this point, no matter what: */
                KASSERT(0 == vn->vn_nrespages);
                KASSERT(1 == vn->vn_refcount);

                /* now, it's actively referenced with no res pages */
        }


        if (0 < --vn->vn_refcount)
                return;

#ifdef __MOUNTING__
        KASSERT(vn->vn_mount == vn);
#endif

        /* no res pages and no more active references; free the vnode */
        KASSERT(0 == vn->vn_refcount);
        KASSERT(0 == vn->vn_nrespages);

        vn->vn_flags |= VN_BUSY;
        if (vn->vn_fs->fs_op->delete_vnode) {
                vn->vn_fs->fs_op->delete_vnode(vn);
        }
        /* (really no need to clear VN_BUSY): */

#ifndef NDEBUG
        if (!sched_queue_empty(&vn->vn_waitq)) {
                dbg(DBG_VNREF, "vput: wow, found thread(s) trying to vget "
                    "(%p, %p ino %ld) after returning from delete_vnode.\n",
                    vn, vn->vn_fs, (long)vn->vn_vno);
        }
#endif

        /* wake up anyone who might have attempted to vget this vnode while
         * we were taking it away: */
        sched_broadcast_on(&vn->vn_waitq);

        list_remove(&vn->vn_link); /* remove from vn_inuse_list */
        slab_obj_free(vnode_allocator, vn);
}

int
vfs_is_in_use(fs_t *fs)
{
        /* - for each vnode vn that is
         *     - if vn does not belong to this fs
         *         - continue
         *
         *     - if vn is not the root vnode and (vn->vn_refcount -
         *       vn->vn_nrespages)
         *         - vn is in use => return -EBUSY
         *
         *     - if vn is the root vnode
         *         - assert: (1 <= (vn->vn_refcount - vn->vn_nrespages))
         *         - if (1 < (vn->vn_refcount - vn->vn_nrespages))
         *             - return -EBUSY
         *
         */
        list_t *list = &vnode_inuse_list;
        list_link_t *link;
        int ret = 0;
        for (link = list->l_next; link != list; link = link->l_next) {
                vnode_t *vn = list_item(link, vnode_t, vn_link);
                int refs;

                KASSERT(vn->vn_refcount >= vn->vn_nrespages);
                KASSERT(vn->vn_nrespages >= 0);

                if (fs != vn->vn_fs)
                        continue;

                /* otherwise, vn belongs to the given fs */

                /* if it is the root vnode and it has more than one
                 * reference
                 *
                 * if it isn't the root vnode and it has at least one
                 * reference
                 */
                refs = (vn->vn_refcount - vn->vn_nrespages);
                KASSERT(0 <= refs);
                KASSERT(((vn->vn_fs->fs_root == vn) && (1 <= refs))
                        || ((vn->vn_fs->fs_root != vn) && (0 <= refs)));

                if (((vn->vn_fs->fs_root == vn) && (1 < refs))
                    || ((vn->vn_fs->fs_root != vn) && refs)) {
                        dbg(DBG_ALL, "Vnode %ld mode %x device %x flags %x is still in use with refcount=%d and %d res pages\n",
                            (long)vn->vn_vno, vn->vn_mode, vn->vn_devid, vn->vn_flags, vn->vn_refcount, vn->vn_nrespages);
                        ret = -EBUSY;
                }
        }

        return ret;
}


void
vnode_flush_all(struct fs *fs)
{
        vnode_t *v;
        pframe_t *p;
        int err;

clean:
        list_iterate_begin(&vnode_inuse_list, v, vnode_t, vn_link) {
                list_iterate_begin(&v->vn_mmobj.mmo_respages,
                                   p, pframe_t, pf_olink) {
                        if (pframe_is_dirty(p)) {
                                if (0 > (err = pframe_clean(p))) {
                                        dbg(DBG_VFS, "vnode_flush_all: WARNING: failed to clean page %d of "
                                            "vnode %ld of fs %p of type %s\n", p->pf_pagenum,
                                            (long)v->vn_vno, v->vn_fs, v->vn_fs->fs_type);
                                }
                                KASSERT((!err)
                                        && "as things presently stand, "
                                        "this shouldn't happen");
                                /* This may have blocked. */
                                goto clean;
                        }
                } list_iterate_end();
        } list_iterate_end();

        /* all pages of all vnodes belonging to this fs have been cleaned.
         * Now, uncache all of them: */
        list_iterate_begin(&vnode_inuse_list, v, vnode_t, vn_link) {
                list_iterate_begin(&v->vn_mmobj.mmo_respages,
                                   p, pframe_t, pf_olink) {
                        KASSERT(!pframe_is_dirty(p));
                        pframe_free(p);
                } list_iterate_end();
        } list_iterate_end();
}


/*
 * Return the number of vnodes from the given filesystem which are in use.
 */
int
vnode_inuse(struct fs *fs)
{
        vnode_t *vn;
        int n = 0;

        list_iterate_begin(&vnode_inuse_list, vn, vnode_t, vn_link) {
                if (vn->vn_fs == fs)
                        n++;
        } list_iterate_end();
        return n;
}

static void
init_special_vnode(vnode_t *vn)
{
        if (S_ISCHR(vn->vn_mode)) {
                vn->vn_ops = &bytedev_spec_vops;
                vn->vn_cdev = bytedev_lookup(vn->vn_devid);
        } else {
                KASSERT(S_ISBLK(vn->vn_mode));
                vn->vn_ops = &blockdev_spec_vops;
                vn->vn_bdev = blockdev_lookup(vn->vn_devid);
        }
}


/*
 * If the file is a byte device then find the file's
 * bytedev_t, and call read on it. Return what read returns.
 *
 * If the file is a block device then return -ENOTSUP
 */
static int
special_file_read(vnode_t *file, off_t offset, void *buf, size_t count)
{
        NOT_YET_IMPLEMENTED("VFS: special_file_read");
        return 0;
}

/*
 * If the file is a byte device find the file's
 * bytedev_t, and call its write. Return what write returns.
 *
 * If the file is a block device then return -ENOTSUP.
 */
static int
special_file_write(vnode_t *file, off_t offset, const void *buf, size_t count)
{
        NOT_YET_IMPLEMENTED("VFS: special_file_write");
        return 0;
}

/* Memory map the special file represented by <file>. All of the
 * work for this function is device-specific, so look up the
 * file's bytedev_t and pass the arguments through to its mmap
 * function. Return what that function returns.
 *
 * Do not worry about this until VM.
 */
static int
special_file_mmap(vnode_t *file, vmarea_t *vma, mmobj_t **ret)
{
        NOT_YET_IMPLEMENTED("VM: special_file_mmap");
        return 0;
}

/* Stat is currently the only filesystem specific routine that we have to worry
 * about for special files.  Here we just call the stat routine for the root
 * directory of the filesystem.
 */
static int
special_file_stat(vnode_t *vnode, struct stat *ss)
{
        KASSERT(vnode->vn_fs->fs_root->vn_ops->stat != NULL);

        /* call the containing file system's stat routine */
        return vnode->vn_fs->fs_root->vn_ops->stat(vnode, ss);
}

/* Just as with mmap above, pass the call through to the
 * device-specific fillpage function.
 *
 * Do not worry about this until VM.
 */
static int
special_file_fillpage(vnode_t *file, off_t offset, void *pagebuf)
{
        NOT_YET_IMPLEMENTED("VM: special_file_fillpage");
        return 0;
}

/* Just as with mmap above, pass the call through to the
 * device-specific dirtypage function.
 *
 * Do not worry about this until VM.
 */
static int
special_file_dirtypage(vnode_t *file, off_t offset)
{
        NOT_YET_IMPLEMENTED("VM: special_file_dirtypage");
        return 0;
}

/* Just as with mmap above, pass the call through to the
 * device-specific cleanpage function.
 *
 * Do not worry about this until VM.
 */
static int
special_file_cleanpage(vnode_t *file, off_t offset, void *pagebuf)
{
        NOT_YET_IMPLEMENTED("VM: special_file_cleanpage");
        return 0;
}

/*
 * Related to implementation of vnode vm_object entry points:
 */

#define mmobj_to_vnode(o) \
        (CONTAINER_OF((o), vnode_t, vn_mmobj))

static void
vo_vref(mmobj_t *o)
{
        KASSERT(o);
        vref(mmobj_to_vnode(o));
}

static void
vo_vput(mmobj_t *o)
{
        KASSERT(o);
        vput(mmobj_to_vnode(o));
}

static int
vlookuppage(mmobj_t *o, uint32_t pagenum, int forwrite, pframe_t **pf)
{
        KASSERT(NULL != pf);
        KASSERT(NULL != o);

        if ((uint32_t) mmobj_to_vnode(o)->vn_len <= pagenum * PAGE_SIZE) {
                return -EINVAL;
        }

        return pframe_get(o, pagenum, pf);
}

static int
vreadpage(mmobj_t *o, pframe_t *pf)
{
        KASSERT(NULL != pf);
        KASSERT(NULL != o);

        vnode_t *v = mmobj_to_vnode(o);
        return v->vn_ops->fillpage(v, (int)PN_TO_ADDR(pf->pf_pagenum), pf->pf_addr);
}

static int
vdirtypage(mmobj_t *o, pframe_t *pf)
{
        KASSERT(NULL != pf);
        KASSERT(NULL != o);

        vnode_t *v = mmobj_to_vnode(o);
        if (!pframe_is_dirty(pf)) {
                return v->vn_ops->dirtypage(v, (int) PN_TO_ADDR(pf->pf_pagenum));
        } else {
                return 0;
        }
}

static int
vcleanpage(mmobj_t *o, pframe_t *pf)
{
        KASSERT(NULL != pf);
        KASSERT(NULL != o);

        vnode_t *v = mmobj_to_vnode(o);
        return v->vn_ops->cleanpage(v, (int) PN_TO_ADDR(pf->pf_pagenum), pf->pf_addr);
}
