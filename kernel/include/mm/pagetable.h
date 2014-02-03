#pragma once

#define PD_PRESENT        0x001
#define PD_WRITE          0x002
#define PD_USER           0x004
#define PD_WRITE_THROUGH  0x008
#define PD_CACHE_DISABLED 0x010
#define PD_ACCESSED       0x020

#define PT_PRESENT        0x001
#define PT_WRITE          0x002
#define PT_USER           0x004
#define PT_WRITE_THROUGH  0x008
#define PT_CACHE_DISABLED 0x010
#define PT_ACCESSED       0x020
#define PT_DIRTY          0x040
#define PT_SIZE           0x080
#define PT_GLOBAL         0x100

typedef uint32_t pte_t;
typedef uint32_t pde_t;

typedef struct pagedir pagedir_t;

/* Temporarily maps one page at the given physical address in at a
 * virtual address and returns that virtual address. Note that repeated
 * calls to this function will return the same virtual address, thereby
 * invalidating the previous mapping. */
uintptr_t pt_phys_tmp_map(uintptr_t paddr);

/* Permenantly maps the given number of physical pages, starting at the
 * given physical address to a virtual address and returns that virtual
 * address. Each call will return a different virtual address and the
 * memory will stay mapped forever. Note that there is an implementation
 * defined limit to the number of pages available and using too many
 * will cause the kernel to panic. */
uintptr_t pt_phys_perm_map(uintptr_t paddr, uint32_t count);

/* Looks up the given virtual address (vaddr) in the current page
 * directory, in order to find the matching physical memory address it
 * points to. vaddr MUST have a mapping in the current page directory,
 * otherwise this function's behavior is undefined */
uintptr_t pt_virt_to_phys(uintptr_t vaddr);

/* Maps the given physical page in at the given virtual page in the
 * given page directory. Creates a new page table if necessary and
 * places an entry in it in the page directory. vaddr must be in the
 * user address space. Both vaddr and paddr must be page aligned.
 * Note that the TLB is not flushed by this function. */
int pt_map(pagedir_t *pd, uintptr_t vaddr, uintptr_t paddr, uint32_t pdflags, uint32_t ptflags);

/* Unmaps the page for the given virtual page from the given page
 * directory. vaddr must be in the user address space. vaddr must
 * be page aligned. Note that the TLB is not flushed by this function. */
void pt_unmap(pagedir_t *pd, uintptr_t vaddr);

/* Unmaps the given range of addresses [low, high). As with pt_unmap,
 * the addresses must be page aligned in the user address space */
void pt_unmap_range(pagedir_t *pd, uintptr_t vlow, uintptr_t vhigh);

/* Creates a new page directory which is initialized to contain
 * mappings for all kernel memory. If there is not enough memory
 * to allocate the directory NULL is returned. Note that destroying
 * a page diretory does not affect the TLB, it is assumed that the
 * page directory being destroyed is not currently in use. Destroying
 * a page directory frees all page tables for user memory referenced
 * by that page directory. */
pagedir_t *pt_create_pagedir();
void pt_destroy_pagedir(pagedir_t *pdir);

/* Sets the page table in cr3 and performs other updates required by
 * the page table subsystem. The address should be a virtual address,
 * it will be translated by the current page table before being
 * placed in cr3. */
void pt_set(pagedir_t *pd);

/* Retreives the virtual address of the page directory currently in cr3. */
pagedir_t *pt_get();
