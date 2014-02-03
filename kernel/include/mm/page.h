#pragma once

/* This header file contains the functions for allocating
 * and freeing page-aligned chunks of data which are a
 * multiple of a page in size. These are the lowest level
 * memory allocation functions. In general code should
 * use the slab allocator functions in mm/alloc.h unless
 * they require page-aligned buffers. */

#define PAGE_SHIFT         12
#define PAGE_SIZE          ((uint32_t)(1UL<<PAGE_SHIFT))
#define PAGE_MASK          (0xffffffff<<PAGE_SHIFT)

#define PAGE_ALIGN_DOWN(x) ((void*)(((uintptr_t)(x))&PAGE_MASK))
#define PAGE_ALIGN_UP(x)   ((void*)(((((uintptr_t)(x))-1)&PAGE_MASK)+PAGE_SIZE))
#define PAGE_OFFSET(x)     ((uintptr_t)(x)&~PAGE_MASK)

#define PN_TO_ADDR(x) ((void *)(((uint32_t)(x)) << PAGE_SHIFT))
#define ADDR_TO_PN(x) (((uint32_t)(x)) >> PAGE_SHIFT)

#define PAGE_ALIGNED(x) (0 == ((uintptr_t)(x)) % PAGE_SIZE)

#define PAGE_NSIZES  8

#define PAGE_SAME(addr1, addr2) (PAGE_ALIGN_DOWN(addr1) == PAGE_ALIGN_DOWN(addr2))

/* Adds the virtual pages [start,end) to those that
 * may be allocated by the page allocator, this should
 * only be called once for any given page (no overlaps). */
void page_add_range(uintptr_t start, uintptr_t end);

/* These functions allocate and free one page-aligned,
 * page-sized block of memory. Values passed to
 * page_free MUST have been returned by page_alloc
 * at some previous point. There should be only one
 * call to page_free for each value returned by
 * page_alloc. If the system is out of memory page_alloc
 * will return NULL. */
void *page_alloc(void);
void  page_free(void *addr);

/* These functions allocate and free a page-aligned
 * block of memory which are npages pages in length.
 * A call to page_alloc_n will allocate a block, to free
 * that block a call should be made to page_free_n with
 * npages set to the same as it was when the block was
 * allocated */
void *page_alloc_n(uint32_t npages);
void  page_free_n(void *start, uint32_t npages);

/* Returns the number of free pages remaining in the
 * system. Note that calls to page_alloc_n(npages) may
 * fail even if page_free_count() >= npages. */
uint32_t page_free_count();
