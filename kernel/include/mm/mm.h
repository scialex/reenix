#pragma once

#define MM_POISON             1
#define MM_POISON_ALLOC       0xBB
#define MM_POISON_FREE        0xDD

#define USER_MEM_LOW          0x00400000 /* inclusive */
#define USER_MEM_HIGH         0xc0000000 /* exclusive */

#define PTR_SIZE (sizeof(void *))
#define PTR_MASK (PTR_SIZE - 1)

/* Performs all initialization necessary for the
 * page allocation system. This should be called
 * only once at boot time before any other functions
 * in this header are called. */
void page_init();

/* This is the first step of initializing the page table system. It
 * replaces the temporary page table set up by the boot loader with
 * the page directory and first 2 page tables of the permenant page
 * table mappings for the kernel. One page table identity maps the
 * first 1mb of physical memory. The other maps the 4mb of physical
 * memory starting with the kernel text at 0xc0000000. */
void pt_init();

/* Called from the bootstrap context in order to set up the template
 * page directory used by all future threads. This cannot be set up
 * prior to this because the stack is mapped in to the first 4mb
 * of memory. Page faults are not handled until this function is
 * called. */
void pt_template_init();

/* Initializes the slab allocator subsystem. This should be done
 * only after the page subsystem has been initialized. Slab allocators
 * and kmalloc will not work until this funciton has been called. */
void slab_init();
