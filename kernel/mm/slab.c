/*
 * slab_alloc.c - Kernel memory allocator
 * Jason Lango <jal@cs.brown.edu>
 *
 * This implementation is based on the description of slab allocation
 * (used in Solaris and Linux) from UNIX Internals: The New Frontiers,
 * by Uresh Vahalia.
 *
 * Note that there is no need for locking in allocation and deallocation because
 * it never blocks nor is used by an interrupt handler. Hurray for non preemptible
 * kernels!
 */

#include "types.h"

#include "mm/mm.h"
#include "mm/slab.h"
#include "mm/page.h"

#include "util/gdb.h"
#include "util/string.h"
#include "util/debug.h"

#ifdef SLAB_REDZONE
#define front_rz(obj)           (*(uintptr_t*)(obj))
#define rear_rz(cache,obj)      (*(uintptr_t*)(((uintptr_t)(obj))               \
                                              + (cache)->sa_objsize             \
                                              - sizeof (uintptr_t)))
#define VERIFY_REDZONES(cache,obj)                                              \
        do {                                                                    \
                if (front_rz(obj) != SLAB_REDZONE)                              \
                        panic("alloc: red-zone check failed: *(0x%p)=0x%.8x\n", \
                              &front_rz(obj), front_rz(obj));                   \
                if (rear_rz(cache,obj) != SLAB_REDZONE)                         \
                        panic("alloc: red-zone check failed: *(0x%p)=0x%.8x\n", \
                              &rear_rz(cache,obj), rear_rz(cache,obj));         \
        } while (0);
#endif

struct slab {
        struct slab             *s_next;       /* link on list of slabs */
        int                      s_inuse;      /* number of allocated objs */
        void                    *s_free;       /* head of obj free list */
        void                    *s_addr;       /* start address */
};

struct slab_allocator {
        struct slab_allocator   *sa_next;       /* link on list of slab allocators */
        const char              *sa_name;       /* user-provided name */
        size_t                   sa_objsize;    /* object size */
        struct slab             *sa_slabs;      /* head of slab list */
        int                      sa_order;      /* npages = (1 << order) */
        int                      sa_slab_nobjs; /* number of objs per slab */
};

struct slab_bufctl {
        union {
                void                *sb_next;   /* next free object */
                struct slab         *sb_slab;   /* containing slab */
        } u;
#ifdef SLAB_CHECK_FREE
        uint8_t                  sb_free;       /* true if is object is free */
#endif
};
#define sb_next                 u.sb_next
#define sb_slab                 u.sb_slab

#define obj_bufctl(allocator, obj) \
        ( (struct slab_bufctl*)(((uintptr_t)(obj)) + (allocator)->sa_objsize) )
#define bufctl_obj(allocator, buf) \
        ( (void*)(((uintptr_t)(buf)) - (allocator)->sa_objsize) )
#define next_obj(allocator, obj) \
        ( (void*) (((uintptr_t)(obj)) + (allocator)->sa_objsize \
                   + sizeof(struct slab_bufctl)) )

GDB_DEFINE_HOOK(slab_obj_alloc, void *addr, struct slab_allocator *allocator)
GDB_DEFINE_HOOK(slab_obj_free, void *addr, struct slab_allocator *allocator)

/* Head of global list of slab allocators. */
static struct slab_allocator *slab_allocators = NULL;

/* Special case - allocator for allocation of slab_allocator objects. */
static struct slab_allocator slab_allocator_allocator;

/*
 * This constant defines how many orders of magnitude (in page block
 * sizes) we'll search for an optimal slab size (past the smallest
 * possible slab size).
 */
#define SLAB_MAX_ORDER                  5

static size_t
_slab_size(size_t objsize, size_t nobjs)
{
        return (nobjs * (objsize + sizeof(struct slab_bufctl))
                + sizeof(struct slab));
}

static int
_slab_nobjs(size_t objsize, size_t order)
{
        return (((PAGE_SIZE << order) - sizeof(struct slab))
                / (objsize + sizeof(struct slab_bufctl)));
}

static int
_slab_waste(size_t objsize, int order)
{
        /* Waste is defined as the amount of unused space in the page
         * block, that is the number of bytes in the page block minus
         * the optimal slab size for that particular block size.
         */
        return ((PAGE_SIZE << order)
                - _slab_size(objsize, _slab_nobjs(objsize, order)));
}

static void
_calc_slab_size(struct slab_allocator *allocator)
{
        int best_order;
        int best_waste;
        int order;
        int minorder;
        int minsize;
        int waste;

        /* Find the minimum page block size that this slab requires. */
        minsize = _slab_size(allocator->sa_objsize, 1);
        for (minorder = 0; minorder < PAGE_NSIZES; minorder++)
                if ((int)(PAGE_SIZE << minorder) >= minsize)
                        break;
        if (minorder == PAGE_NSIZES)
                panic("unable to find minorder\n");

        /* Start the search with the minimum block size for this slab. */
        best_order = minorder;
        best_waste = _slab_waste(allocator->sa_objsize, minorder);

        dbg(DBG_MM, "calc_slab_size: minorder %d, waste %d\n", minorder, best_waste);

        /* Find the optimal number of objects per slab and slab size,
         * up to a predefined (somewhat arbitrary) limit on the number
         * of pages per slab.
         */
        for (order = minorder + 1; order < SLAB_MAX_ORDER; order++) {
                if ((waste = _slab_waste(allocator->sa_objsize, order)) < best_waste) {
                        best_waste = waste;
                        best_order = order;
                        dbg(DBG_MM, "calc_slab_size: replacing with order %d, waste %d\n",
                            best_order, best_waste);
                }
        }

        /* Finally, the best page block size wins.
        */
        allocator->sa_order = best_order;
        allocator->sa_slab_nobjs = _slab_nobjs(allocator->sa_objsize, best_order);
}

static void
_allocator_init(struct slab_allocator *allocator, const char *name, size_t size)
{
#ifdef SLAB_REDZONE
        /*
         * Add space for the front and rear red-zones.
         */
        size += 2 * sizeof(SLAB_REDZONE);
#endif

        if (!name)
                name = "<unnamed>";

        allocator->sa_name = name;
        allocator->sa_objsize = size;
        allocator->sa_slabs = NULL;
        _calc_slab_size(allocator);

        /* Add cache to global cache list. */
        allocator->sa_next = slab_allocators;
        slab_allocators = allocator;

        dbg(DBG_MM, "Initialized new slab allocator:\n");
        dbgq(DBG_MM, "  Name:          \"%s\" (0x%p)\n", allocator->sa_name, allocator);
        dbgq(DBG_MM, "  Object Size:   %d\n", allocator->sa_objsize);
        dbgq(DBG_MM, "  Order:         %d\n", allocator->sa_order);
        dbgq(DBG_MM, "  Slab Capacity: %d\n", allocator->sa_slab_nobjs);
}

struct slab_allocator *
slab_allocator_create(const char *name, size_t size) {
        struct slab_allocator *allocator;

        allocator = (struct slab_allocator *) slab_obj_alloc(&slab_allocator_allocator);
        if (!allocator)
                return NULL;

        _allocator_init(allocator, name, size);
        return allocator;
}


static int
_slab_allocator_grow(struct slab_allocator *allocator)
{
        void *addr;
        void *obj;
        int ii, npages;
        struct slab *slab;

        npages = 1 << allocator->sa_order;
        addr = page_alloc_n(npages);
        if (!addr)
                return 0;

        /* Initialize each bufctl to be free and point to the next object. */
        obj = addr;
        for (ii = 0; ii < (allocator->sa_slab_nobjs - 1); ii++) {
#ifdef SLAB_CHECK_FREE
                obj_bufctl(allocator, obj)->sb_free = 1;
#endif
                obj = obj_bufctl(allocator, obj)->sb_next = next_obj(allocator, obj);
        }

        /* The last bufctl is the tail of the list. */
#ifdef SLAB_CHECK_FREE
        obj_bufctl(allocator, obj)->sb_free = 1;
#endif
        obj_bufctl(allocator, obj)->sb_next = NULL;

        /* After the last object comes the slab structure itself. */
        slab = (struct slab *) next_obj(allocator, obj);

        /*
         * The first object in the slab will be the head of the free
         * list and the start address of the slab.
         */
        slab->s_free = addr;
        slab->s_addr = addr;
        slab->s_inuse = 0;

        /* Initialize objects. */
        obj = addr;
        for (ii = 0; ii < allocator->sa_slab_nobjs; ii++) {
#ifdef SLAB_REDZONE
                front_rz(obj) = SLAB_REDZONE;
                rear_rz(allocator, obj) = SLAB_REDZONE;
#endif
                obj = next_obj(allocator, obj);
        }

        dbg(DBG_MM, "Growing cache \"%s\" (0x%p), new slab 0x%p "
            "(%d pages)\n", allocator->sa_name, allocator, slab,
            1 << allocator->sa_order);

        /* Place this slab into the cache. */
        slab->s_next = allocator->sa_slabs;
        allocator->sa_slabs = slab;

        return 1;
}

void *
slab_obj_alloc(struct slab_allocator *allocator)
{
        struct slab *slab;
        void *obj;

        /* Find a slab with a free object. */
        for (;;) {
                slab = allocator->sa_slabs;
                while (slab && (slab->s_inuse == allocator->sa_slab_nobjs))
                        slab = slab->s_next;
                if (slab && (slab->s_inuse < allocator->sa_slab_nobjs))
                        break;
                if (!_slab_allocator_grow(allocator))
                        return NULL;
        }

        /*
         * Remove an object from the slab's free list.  We'll use the
         * free list pointer to store a pointer back to the containing
         * slab.
         */
        obj = slab->s_free;
        slab->s_free = obj_bufctl(allocator, obj)->sb_next;
        obj_bufctl(allocator, obj)->sb_slab = slab;
#ifdef SLAB_CHECK_FREE
        obj_bufctl(allocator, obj)->sb_free = 0;
#endif

        slab->s_inuse++;

        dbg(DBG_MM, "Allocated object 0x%p from \"%s\" (0x%p), "
            "slab 0x%p, inuse %d\n", obj, allocator->sa_name,
            allocator, allocator, slab->s_inuse);

#ifdef SLAB_REDZONE
        VERIFY_REDZONES(allocator, obj);

        /*
         * Make object pointer point past the first red-zone.
         */
        obj = (void *)((uintptr_t)obj + sizeof(SLAB_REDZONE));
#endif

        GDB_CALL_HOOK(slab_obj_alloc, obj, allocator);
        return obj;
}

void
slab_obj_free(struct slab_allocator *allocator, void *obj)
{
        struct slab *slab;
        GDB_CALL_HOOK(slab_obj_free, obj, allocator);

#ifdef SLAB_REDZONE
        /* Move pointer back.  See the end of kmem_cache_alloc. */
        obj = (void *)((uintptr_t)obj - sizeof(SLAB_REDZONE));

        VERIFY_REDZONES(allocator, obj);
#endif

#ifdef SLAB_CHECK_FREE
        KASSERT(!obj_bufctl(allocator, obj)->sb_free && "INVALID FREE!");
        obj_bufctl(allocator, obj)->sb_free = 1;
#endif

        slab = obj_bufctl(allocator, obj)->sb_slab;

        /* Place this object back on the slab's free list. */
        obj_bufctl(allocator, obj)->sb_next = slab->s_free;
        slab->s_free = obj;

        slab->s_inuse--;

        dbg(DBG_MM, "Freed object 0x%p from \"%s\" (0x%p), slab 0x%p, inuse %d\n",
            obj, allocator->sa_name, allocator, slab, slab->s_inuse);
}

/*
 * Reclaims as much memory (up to a target) from
 * unused slabs as possible
 * @param target - target number of pages to reclaim. If negative,
 * try to reclaim as many pages as possible
 * @return number of pages freed
 */
int
slab_allocators_reclaim(int target)
{
        int npages_freed = 0, npages;

        struct slab_allocator *a;
        struct slab *s, **prev;

        /* Go through all caches */
        for (a = slab_allocators; NULL != a; a = a->sa_next) {
                prev = &(a->sa_slabs);
                s = a->sa_slabs;
                while (NULL != s) {
                        struct slab *next = s->s_next;
                        if (0 == s->s_inuse) {
                                /* Free Slab */
                                (*prev) = next;
                                npages = 1 << a->sa_order;

                                page_free_n(s->s_addr, npages);
                                npages_freed += npages;
                        } else {
                                prev = &(s->s_next);
                        }
                        /* Check if target was met */
                        if ((target > 0) && (npages_freed >= target)) {
                                return npages_freed;
                        }
                        s = next;
                }
        }
        return npages_freed;
}

#define KMALLOC_SIZE_MIN_ORDER  (6)
#define KMALLOC_SIZE_MAX_ORDER  (18)

static struct slab_allocator *kmalloc_allocators[KMALLOC_SIZE_MAX_ORDER - KMALLOC_SIZE_MIN_ORDER + 1];

/* Note that kmem_cache_names should be modified to remain consistent
 * with KMALLOC_SIZE_MIN_ORDER ... KMALLOC_SIZE_MAX_ORDER.
 */
static const char *kmalloc_allocator_names[] = {
        "size-64",
        "size-128",
        "size-256",
        "size-512",
        "size-1024",
        "size-2048",
        "size-4096",
        "size-8192",
        "size-16384",
        "size-32768",
        "size-65536",
        "size-131072",
        "size-262144"
};

void *
kmalloc(size_t size)
{
        int order;
        struct slab_allocator **cs;
        void *addr;

        size += sizeof(struct slab_allocator *);

        /*
         * Find the first power of two bucket bigger than the
         * requested size, and allocate from it.
         */
        cs = kmalloc_allocators;
        for (order = KMALLOC_SIZE_MIN_ORDER; order <= KMALLOC_SIZE_MAX_ORDER; order++, cs++) {
                if ((size_t)(1 << order) >= size) {
                        addr = slab_obj_alloc(*cs);
                        if (!addr) {
                                dbg(DBG_MM, "WARNING: kmalloc out of memory\n");
                                return NULL;
                        }
#ifdef MM_POISON
                        memset(addr, MM_POISON_ALLOC, size);
#endif /* MM_POISON */
                        *((struct slab_allocator **)addr) = *cs;
                        return (void *)(((struct slab_allocator **)addr) + 1);
                }
        }

        panic("size bigger than maxorder %ld\n", (unsigned long) size);
        return NULL;
}

__attribute__((used)) static void *
malloc(size_t size)
{
        /* This function is used by gdb to allocate memory
         * within the kernel, no code in the kernel should
         * call it. */
        return kmalloc(size);
}

void
kfree(void *addr)
{
        addr = (void *)(((struct slab_allocator **)addr) - 1);
        struct slab_allocator *sa = *(struct slab_allocator **)addr;

#ifdef MM_POISON
        /* If poisoning is enabled, wipe the memory given in
         * this object, as specified by the cache object size
         * (minus red-zone overhead, if any).
         */
        int objsize = sa->sa_objsize;
#ifdef SLAB_REDZONE
        objsize -= sizeof(SLAB_REDZONE) * 2;
#endif /* SLAB_REDZONE */
        memset(addr, MM_POISON_FREE, objsize);
#endif /* MM_POISON */

        slab_obj_free(sa, addr);
}

__attribute__((used)) static void
free(void *addr)
{
        /* This function is used by gdb to free memory allocated
         * by malloc, no code in the kernel should call it. */
        kfree(addr);
}

void
slab_init()
{
        int order;
        struct slab_allocator **cs;

        /* Special case initialization of the kmem_cache_t cache. */
        _allocator_init(&slab_allocator_allocator, "slab_allocators", sizeof(struct slab_allocator));

        /*
         * Allocate the power of two buckets for generic
         * kmalloc/kfree.
         */
        cs = kmalloc_allocators;
        for (order = KMALLOC_SIZE_MIN_ORDER; order <= KMALLOC_SIZE_MAX_ORDER; order++, cs++) {
                if (NULL == (*cs = slab_allocator_create(kmalloc_allocator_names[order - KMALLOC_SIZE_MIN_ORDER], (1 << order)))) {
                        panic("Couldn't create kmalloc allocators!\n");
                }
        }
}
