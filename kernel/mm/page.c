#include "types.h"
#include "kernel.h"

#include "mm/mm.h"
#include "mm/page.h"
#include "mm/slab.h"

#include "util/gdb.h"
#include "util/bits.h"
#include "util/list.h"
#include "util/debug.h"
#include "util/string.h"

#include "vm/shadowd.h"

#include "proc/sched.h"

GDB_DEFINE_HOOK(page_alloc, void *addr, int npages)
GDB_DEFINE_HOOK(page_free, void *addr, int npages)

static list_t pagegroup_list;
static uintptr_t page_freecount;

struct pagegroup {
        list_t       pg_freelist[PAGE_NSIZES];
        void        *pg_map[PAGE_NSIZES];
        uintptr_t    pg_baseaddr;
        uintptr_t    pg_endaddr;
        list_link_t  pg_link;
};

struct freepage {
        list_link_t fp_link;
};

static struct pagegroup *
_pagegroup_create(uintptr_t start, uintptr_t end)
{
        KASSERT(PAGE_NSIZES > 0);
        KASSERT(sizeof(struct pagegroup) <= PAGE_SIZE);

        uintptr_t npages = (end - start) >> PAGE_SHIFT;
        struct pagegroup *group;

        end -= sizeof(*group);
        group = (struct pagegroup *)end;

        group->pg_baseaddr = start;
        group->pg_map[0] = NULL;

        /* allocate some of the space for the buddy bit maps,
         * we allocate enough bits to track all pages even
         * though some pages will be unavailable since they
         * are being used as bitmaps */
        int order;
        for (order = 1; order < PAGE_NSIZES; ++order) {
                uintptr_t count = npages >> order;
                count = ((count - 1) & ~((uintptr_t)0x7)) + 8;
                count = count >> 3;
                end -= count;
                group->pg_map[order] = (void *)end;
                memset(group->pg_map[order], 0, count);
        }

        /* discard the remainder of the page being used for
         * mappings and read just npages */
        end = (uintptr_t)PAGE_ALIGN_DOWN(end);
        npages = (end - start) >> PAGE_SHIFT;
        group->pg_endaddr = end;

        /* put pages which do not fit nicely into the largest
         * order and add them to smaller buckets */
        for (order = 0; order < PAGE_NSIZES - 1; ++order) {
                list_init(&group->pg_freelist[order]);
                if (npages & (1 << order)) {
                        end -= (1 << order) << PAGE_SHIFT;
                        list_insert_head(&group->pg_freelist[order], &((struct freepage *)end)->fp_link);
                }
        }

        /* put the remaining pages into the largest bucket */
        KASSERT(0 == (end - start) % (1 << order));
        list_init(&group->pg_freelist[order]);
        uintptr_t current = start;
        while (current < end) {
                list_insert_head(&group->pg_freelist[order], &((struct freepage *)current)->fp_link);
                current += (1 << order) << PAGE_SHIFT;
        }

        return group;
}

static struct pagegroup *
_pagegroup_from_address(uintptr_t addr)
{
        struct pagegroup *group;
        list_iterate_begin(&pagegroup_list, group, struct pagegroup, pg_link) {
                if (addr >= group->pg_baseaddr && addr < group->pg_endaddr)
                        return group;
        } list_iterate_end();
        return NULL;
}

void
page_init()
{
        list_init(&pagegroup_list);
        page_freecount = 0;
}

void
page_add_range(uintptr_t start, uintptr_t end)
{
        dbgq(DBG_MM, "Page System adding range: 0x%08x to 0x%08x\n", start, end);

        /* page align the start and end */
        start = (uintptr_t) PAGE_ALIGN_DOWN(start);
        end = (uintptr_t) PAGE_ALIGN_DOWN(end);

        struct pagegroup *group = _pagegroup_create(start, end);
        if (group->pg_baseaddr < group->pg_endaddr) {
                list_insert_tail(&pagegroup_list, &group->pg_link);
                page_freecount += ADDR_TO_PN(group->pg_endaddr - group->pg_baseaddr);
        }
}

/**
 * Calculates the address's index in to the buddy bitmap for the
 * specified order. The address must be within the range of addresses
 * managed by the given group and should be either the exact address
 * of one of the pages of the given order or the address of one of
 * the pages resulting from splitting a page of the given order
 * exactly once.
 *
 * @param group the page group the address falls in
 * @param order the order within the page group which we are interested in
 * @param addr the address whose index is being calculated
 * @return the index of the given address
 */
static inline uintptr_t
_pagegroup_calculate_index(struct pagegroup *group, uint32_t order, uintptr_t addr)
{
        KASSERT(PAGE_ALIGNED(addr));
        KASSERT(PAGE_NSIZES > order);
        KASSERT(addr >= group->pg_baseaddr && addr < group->pg_endaddr);

        uintptr_t offset = addr - group->pg_baseaddr;
        KASSERT(0 == (offset & ((1 << order) - 1)));
        return (offset >> order) >> PAGE_SHIFT;
}

static void
__page_split(struct pagegroup *group, uint32_t order)
{
        KASSERT(0 < order);
        KASSERT(PAGE_NSIZES > order);
        KASSERT(!list_empty(&group->pg_freelist[order]));
        KASSERT(PAGE_SIZE >= sizeof(uintptr_t));

        uintptr_t target = (uintptr_t)list_head(&group->pg_freelist[order], struct freepage, fp_link);
        list_remove_head(&group->pg_freelist[order]);

        /* splitting the page requires marking it as allocated */
        if (likely(order < PAGE_NSIZES - 1)) {
                uintptr_t index = _pagegroup_calculate_index(group, order + 1, target);
                bit_flip(group->pg_map[order + 1], index);
        }

        KASSERT(!bit_check(group->pg_map[order], _pagegroup_calculate_index(group, order, target)));

        uintptr_t buddy = (target + ((1 << (order - 1)) << PAGE_SHIFT));
        list_insert_head(&group->pg_freelist[order - 1], &((struct freepage *)target)->fp_link);
        list_insert_head(&group->pg_freelist[order - 1], &((struct freepage *)buddy)->fp_link);
        dbg(DBG_PAGEALLOC, "split 0x%.8x (%u) into 0x%.8x and 0x%.8x\n", target, order, target, buddy);
}

/**
 * Finds a block of pages strictly bigger than a block of the given order and
 * splits it into blocks of the given order. Used, for example, when the user
 * requests a 4k block and there are no free 4k blocks, but there is an 8k or
 * 16k block.
 *
 * @param order the order of the block to split into.
 * @return the group where the split took place on success, NULL otherwise
 */
static struct pagegroup *
_page_split(int order)
{
#ifdef __SHADOWD__
        uint32_t num_retrys = 2;
#else
        uint32_t num_retrys = 0;
#endif
        int norder;

        do {
                /* Find the first free block of greater size than requested. */
                for (norder = order + 1; norder < PAGE_NSIZES; norder++) {
                        struct pagegroup *group;
                        list_iterate_begin(&pagegroup_list, group, struct pagegroup, pg_link) {
                                if (!list_empty(&group->pg_freelist[norder])) {
                                        while (norder > order) {
                                                __page_split(group, norder);
                                                --norder;
                                        }
                                        KASSERT(!list_empty(&group->pg_freelist[order]));
                                        return group;
                                }
                        } list_iterate_end();
                }

                dbg(DBG_PAGEALLOC, "WARNING, cannot allocate order=%u\n", order);
                /* We have run out of kernel memory. Lets try and collapse some
                   shadow trees, and then retry */
#ifdef __SHADOWD__
                dbg(DBG_PAGEALLOC, "waking up shadowd\n");
                shadowd_wakeup();
                shadowd_alloc_sleep();
#endif
                int num_freed = slab_allocators_reclaim(0);
                dbg(DBG_MM, "reclaimed %d pages from slab allocator.\n", num_freed);
        } while (num_retrys-- > 0);

        /* We are out of memory, and not even the shadow deamon could free some */
        return NULL;
}

/**
 * Allocate a block of at least 2^order pages. Fills the block with the
 * MM_POISON_ALLOC pattern.
 *
 * @param order the order of the block size desired
 * @return the address of the free memory or null if no memory could be allocated
 */
static void *
_page_alloc_order(uint32_t order)
{
        uintptr_t addr;
        struct pagegroup *group;

        list_iterate_begin(&pagegroup_list, group, struct pagegroup, pg_link) {
                if (!list_empty(&group->pg_freelist[order]))
                        goto found;
        } list_iterate_end();

        if (NULL != (group = _page_split(order))) {
                KASSERT(!list_empty(&group->pg_freelist[order]));
                goto found;
        }
        return NULL;

found:
        addr = (uintptr_t)list_head(&group->pg_freelist[order], struct freepage, fp_link);
        list_remove_head(&group->pg_freelist[order]);
        if (PAGE_NSIZES - 1 > order)
                bit_flip(group->pg_map[order + 1], _pagegroup_calculate_index(group, order + 1, addr));

        dbg(DBG_MM, "allocating %d pages (addr 0x%x)\n", (1 << order), addr);

#ifdef MM_POISON
        /*
         * Wipe the pages with a special bit-pattern, so that
         * uninitialized memory accesses will be obvious.
         */
        memset((void *)addr, MM_POISON_ALLOC, (1 << order) << PAGE_SHIFT);
#endif /* MM_POISON */

        page_freecount -= (1 << order);
        return (void *) addr;
}

static void
__page_join(struct pagegroup *group, int order, uintptr_t addr)
{
        uintptr_t index;
        while (PAGE_NSIZES - 1 > order && !bit_check(group->pg_map[order + 1],
                        index = _pagegroup_calculate_index(group, order + 1, (uintptr_t)addr))) {
                uintptr_t offset = addr - group->pg_baseaddr;
                uintptr_t buddy = addr + ((1 << order) << PAGE_SHIFT) * ((((offset >> PAGE_SHIFT) >> order) & 0x1) ? -1 : 1);

                KASSERT(0 == (offset & ((1 << order) - 1)));

                dbg(DBG_PAGEALLOC, "joining 0x%.8x and 0x%.8x (%u) into 0x%.8x\n", addr, buddy, order, MIN(offset, buddy));

                list_remove(&((struct freepage *)addr)->fp_link);
                list_remove(&((struct freepage *)buddy)->fp_link);
                addr = MIN(addr, buddy);
                ++order;
                list_insert_head(&group->pg_freelist[order], &((struct freepage *)addr)->fp_link);

                if (PAGE_NSIZES - 1 > order)
                        bit_flip(group->pg_map[order + 1], _pagegroup_calculate_index(group, order + 1, (uintptr_t)addr));
        }
}

/**
 * Free a block of 2^order pages. Fills the memory with a special
 * MM_POISON_FREE pattern.
 *
 * @param addr the start of the block being freed
 * @param order the order of the block size being freed
 */
static void
_page_free_order(void *addr, int order)
{
#ifdef MM_POISON
        /*
         * Wipe the pages with a special bit-pattern, so that invalid
         * references (to free pages) will be obvious.
         */
        memset(addr, MM_POISON_FREE, (1 << order) << PAGE_SHIFT);
#endif /* MM_POISON */

        struct pagegroup *group = _pagegroup_from_address((uintptr_t)addr);
        if (NULL == group)
                return;

        list_insert_head(&group->pg_freelist[order], &((struct freepage *)addr)->fp_link);
        page_freecount += (1 << order);

        if (PAGE_NSIZES - 1 > order) {
                uintptr_t index = _pagegroup_calculate_index(group, order + 1, (uintptr_t)addr);
                bit_flip(group->pg_map[order + 1], index);
                __page_join(group, order, (uintptr_t)addr);
        }

        dbg(DBG_MM, "page_free: freed %d pages (addr 0x%p); %u pages currently free\n",
            (1 << order), addr, page_freecount);
}

/*
 * Allocate one page of memory (which is, of course page-aligned).
 * @return the address of the page
 */
void *
page_alloc(void)
{
        void *addr =  _page_alloc_order(0);
        GDB_CALL_HOOK(page_alloc, addr, 1);
        return addr;
}

/*
 * Free one page of memory (which was allocated with page_alloc())
 * @param addr the address of the page to be freed
 */
void
page_free(void *addr)
{
        GDB_CALL_HOOK(page_free, addr, 1);
        _page_free_order(addr, 0);
}

/*
 * Allocates a block of at least npages pages.
 * @param npages the number of pages to allocate
 * @return the address of the block
 */
void *
page_alloc_n(uint32_t npages)
{
        int order;

        for (order = 0; order < PAGE_NSIZES; order++)
                if ((1 << order) >= (int)npages)
                        break;
        if (order == PAGE_NSIZES)
                panic("Implementation does not permit allocating %u pages!\n", npages);

        void *addr = _page_alloc_order(order);
        GDB_CALL_HOOK(page_alloc, addr, npages);
        return addr;
}

/*
 * Frees a block of npages pages allocated with page_alloc_n().
 * @param npages the size of the block (as given to page_alloc_n)
 */
void
page_free_n(void *start, uint32_t npages)
{
        int order;

        for (order = 0; order < PAGE_NSIZES; order++)
                if ((1 << order) >= (int)npages)
                        break;
        if (order == PAGE_NSIZES)
                panic("Implementation does not permit allocating %u pages!\n", npages);

        GDB_CALL_HOOK(page_free, start, npages);
        _page_free_order(start, order);
}

/*
 * @return the number of free pages in the kmem system
 */
uint32_t
page_free_count()
{
        return page_freecount;
}
