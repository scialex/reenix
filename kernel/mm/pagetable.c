#include "types.h"
#include "kernel.h"
#include "config.h"
#include "errno.h"
#include "limits.h"
#include "globals.h"

#include "main/interrupt.h"

#include "mm/mm.h"
#include "mm/page.h"
#include "mm/pagetable.h"
#include "mm/phys.h"
#include "mm/tlb.h"
#include "mm/pframe.h"

#include "util/debug.h"
#include "util/string.h"
#include "util/printf.h"

#include "vm/pagefault.h"

#include "boot/config.h"

#define PT_ENTRY_COUNT    (PAGE_SIZE / sizeof (uint32_t))
#define PT_VADDR_SIZE     (PAGE_SIZE * PT_ENTRY_COUNT)

struct pagedir {
        pde_t      pd_physical[PT_ENTRY_COUNT];
        uintptr_t *pd_virtual[PT_ENTRY_COUNT];
};

/* for a given virtual memory address these macros will
 * calculate the index into the page directory and page
 * tables for that memory location as well as the offset
 * into the page of physical memory */
#define vaddr_to_pdindex(vaddr) \
        ((((uint32_t)(vaddr)) >> PAGE_SHIFT) / PT_ENTRY_COUNT)
#define vaddr_to_ptindex(vaddr) \
        ((((uint32_t)(vaddr)) >> PAGE_SHIFT) % PT_ENTRY_COUNT)
#define vaddr_to_offset(vaddr) \
        (((uint32_t)(vaddr)) & (~PAGE_MASK))

/* the virtual address of the page directory in cr3 */
static pagedir_t *current_pagedir = NULL;
static pagedir_t *template_pagedir = NULL;

static uint32_t phys_map_count = 1;
static pte_t *final_page;

uintptr_t
pt_phys_tmp_map(uintptr_t paddr)
{
        KASSERT(PAGE_ALIGNED(paddr));
        final_page[PT_ENTRY_COUNT - 1] = paddr | PT_PRESENT | PT_WRITE;

        uintptr_t vaddr = UPTR_MAX - PAGE_SIZE + 1;
        tlb_flush(vaddr);
        return vaddr;
}

uintptr_t
pt_phys_perm_map(uintptr_t paddr, uint32_t count)
{
        KASSERT(PAGE_ALIGNED(paddr));

        phys_map_count += count;
        KASSERT(phys_map_count < PT_ENTRY_COUNT);

        uint32_t i;
        for (i = 0; i < count; ++i) {
                final_page[PT_ENTRY_COUNT - phys_map_count + i] =
                        (paddr + PAGE_SIZE * i) | PT_PRESENT | PT_WRITE;
        }

        uintptr_t vaddr = UPTR_MAX - (PAGE_SIZE * phys_map_count) + 1;
        tlb_flush(vaddr);
        return vaddr;
}

uintptr_t
pt_virt_to_phys(uintptr_t vaddr)
{
        uint32_t table = vaddr_to_pdindex(vaddr);
        uint32_t entry = vaddr_to_ptindex(vaddr);
        uint32_t offset = vaddr_to_offset(vaddr);

        pte_t *pagetable = (pte_t *)pt_phys_tmp_map(current_pagedir->pd_physical[table] & PAGE_MASK);
        uintptr_t page = pagetable[entry] & PAGE_MASK;
        return page + offset;
}

void
pt_set(pagedir_t *pd)
{
        uintptr_t pdir = pt_virt_to_phys((uintptr_t)pd->pd_physical);
        current_pagedir = pd;
        __asm__ volatile("movl %0, %%cr3" :: "r"(pdir) : "memory");
}

pagedir_t *
pt_get(void)
{
        return current_pagedir;
}

int
pt_map(pagedir_t *pd, uintptr_t vaddr, uintptr_t paddr, uint32_t pdflags, uint32_t ptflags)
{
        KASSERT(PAGE_ALIGNED(vaddr) && PAGE_ALIGNED(paddr));
        KASSERT(USER_MEM_LOW <= vaddr && USER_MEM_HIGH > vaddr);

        int index = vaddr_to_pdindex(vaddr);

        pte_t *pt;
        if (!(PT_PRESENT & pd->pd_physical[index])) {
                if (NULL == (pt = page_alloc())) {
                        return -ENOMEM;
                } else {
                        KASSERT((pdflags & ~PAGE_MASK) == pdflags);
                        memset(pt, 0, PAGE_SIZE);
                        pd->pd_physical[index] = pt_virt_to_phys((uintptr_t)pt) | pdflags;
                        pd->pd_virtual[index] = pt;
                }
        } else {
                /* Be sure to add additional pagedir flags if necessary */
                pd->pd_physical[index] = pd->pd_physical[index] | pdflags;
                pt = (pte_t *)pd->pd_virtual[index];
        }

        index = vaddr_to_ptindex(vaddr);

        KASSERT((ptflags & ~PAGE_MASK) == ptflags);
        pt[index] = paddr | ptflags;

        return 0;
}

void
pt_unmap(pagedir_t *pd, uintptr_t vaddr)
{
        KASSERT(PAGE_ALIGNED(vaddr));
        KASSERT(USER_MEM_LOW <= vaddr && USER_MEM_HIGH > vaddr);

        int index = vaddr_to_pdindex(vaddr);

        if (PT_PRESENT & pd->pd_physical[index]) {
                pte_t *pt = (pte_t *)pd->pd_virtual[index];

                index = vaddr_to_ptindex(vaddr);
                pt[index] = 0;
        }
}

void
pt_unmap_range(pagedir_t *pd, uintptr_t vlow, uintptr_t vhigh)
{
        uint32_t index;

        KASSERT(vlow < vhigh);
        KASSERT(PAGE_ALIGNED(vlow) && PAGE_ALIGNED(vhigh));
        KASSERT(USER_MEM_LOW <= vlow && USER_MEM_HIGH >= vhigh);

        index = vaddr_to_ptindex(vlow);
        if (PT_PRESENT & pd->pd_physical[vaddr_to_pdindex(vlow)] && index != 0) {
                pte_t *pt = (pte_t *)pd->pd_virtual[vaddr_to_pdindex(vlow)];
                size_t size = (PT_ENTRY_COUNT - index) * sizeof(*pt);
                memset(&pt[index], 0, size);
        }
        vlow += PAGE_SIZE * ((PT_ENTRY_COUNT - index) % PT_ENTRY_COUNT);

        index = vaddr_to_ptindex(vhigh);
        if (PT_PRESENT & pd->pd_physical[vaddr_to_pdindex(vhigh)] && index != 0) {
                pte_t *pt = (pte_t *)pd->pd_virtual[vaddr_to_pdindex(vhigh)];
                size_t size = index * sizeof(*pt);
                memset(&pt[0], 0, size);
        }
        vhigh -= PAGE_SIZE * index;

        uint32_t i;
        for (i = vaddr_to_pdindex(vlow); i < vaddr_to_pdindex(vhigh); ++i) {
                if (PT_PRESENT & pd->pd_physical[i]) {
                        page_free(pd->pd_virtual[i]);
                        pd->pd_virtual[i] = NULL;
                        pd->pd_physical[i] = 0;
                }
        }
}


pagedir_t *
pt_create_pagedir()
{
        KASSERT(sizeof(pagedir_t) == PAGE_SIZE * 2);

        pagedir_t *pdir;
        if (NULL == (pdir = page_alloc_n(2))) {
                return NULL;
        }

        memcpy(pdir, template_pagedir, sizeof(*pdir));
        return pdir;
}

void
pt_destroy_pagedir(pagedir_t *pdir)
{
        KASSERT(PAGE_ALIGNED(pdir));

        uint32_t begin = USER_MEM_LOW / PT_VADDR_SIZE;
        uint32_t end = (USER_MEM_HIGH - 1) / PT_VADDR_SIZE;
        KASSERT(begin < end && begin > 0);

        uint32_t i;
        for (i = begin; i <= end; ++i) {
                if (PT_PRESENT & pdir->pd_physical[i]) {
                        page_free(pdir->pd_virtual[i]);
                }
        }
        page_free_n(pdir, 2);
}

static void
_pt_fault_handler(regs_t *regs)
{
        uintptr_t vaddr;
        /* Get the address where the fault occurred */
        __asm__ volatile("movl %%cr2, %0" : "=r"(vaddr));
        uint32_t cause = regs->r_err;

        /* Check if pagefault was in user space (otherwise, BAD!) */
        if (cause & FAULT_USER) {
                handle_pagefault(vaddr, cause);
        } else {
                panic("\nPage faulted while accessing 0x%08x\n", vaddr);
        }
}

static void
_pt_fill_page(pagedir_t *pd, pte_t *pt, pde_t pdflags, pte_t ptflags,
              uintptr_t vstart, uintptr_t pstart)
{
        KASSERT(NULL != pd && NULL != pt);
        KASSERT(0 == vstart % PT_VADDR_SIZE);

        uint32_t i;
        memset(pt, 0, PAGE_SIZE);
        for (i = 0; i < PT_ENTRY_COUNT; ++i) {
                pt[i] = (i * PAGE_SIZE + pstart) & PAGE_MASK;
                pt[i] = pt[i] | (ptflags & ~(PAGE_MASK));
        }
        uint32_t base = vaddr_to_pdindex(vstart);

        uint32_t table = vaddr_to_pdindex((uintptr_t)pt);
        uint32_t entry = vaddr_to_ptindex((uintptr_t)pt);

        pde_t *temppdir;
        __asm__ volatile("movl %%cr3, %0" : "=r"(temppdir));
        pte_t *pagetable = (pte_t *)pt_phys_tmp_map(temppdir[table] & PAGE_MASK);
        uintptr_t page = pagetable[entry] & PAGE_MASK;

        pd->pd_physical[base] = page | (pdflags & ~(PAGE_MASK));
        pd->pd_virtual[base] = pt;
}

void
pt_init(void)
{
        /* save the "current" page table (the temporary page
         * table created by the boot loader), note, the value is
         * only valid because the bootloader placed the page table
         * in the first 1mb of memory which is identity mapped,
         * normally current_pagedir holds a virtual address while
         * cr3 holds a physical address */
        pde_t *temppdir;
        __asm__ volatile("movl %%cr3, %0" : "=r"(temppdir));

        pagedir_t *pagedir = (pagedir_t *)&kernel_end;
        /* The kernel ending address should be page aligned by the linker script */
        KASSERT(PAGE_ALIGNED(pagedir));
        memset(pagedir, 0, sizeof(*pagedir));

        /* set up the necessary stuff for temporary mappings */
        final_page = (pde_t *)((char *)pagedir + sizeof(*pagedir));
        KASSERT(PAGE_ALIGNED(final_page));
        memset(final_page, 0, PAGE_SIZE);
        temppdir[PT_ENTRY_COUNT - 1] = ((uintptr_t)final_page
                                        - (uintptr_t)&kernel_start + KERNEL_PHYS_BASE) | PT_PRESENT | PT_WRITE;
        pagedir->pd_physical[PT_ENTRY_COUNT - 1] = temppdir[PT_ENTRY_COUNT - 1];
        pagedir->pd_virtual[PT_ENTRY_COUNT - 1] = final_page;

        /* identity map the first 4mb (one page table) of physical memory */
        pte_t *pagetable = final_page + PT_ENTRY_COUNT;
        _pt_fill_page(pagedir, pagetable, PD_PRESENT | PD_WRITE, PT_PRESENT | PT_WRITE, 0, 0);

        /* map in 4mb (one page table) where the kernel is
         * this will make our new page table identical to the temporary
         * page table the boot loader created. */
        pagetable += PT_ENTRY_COUNT;
        _pt_fill_page(pagedir, pagetable, PD_PRESENT | PD_WRITE, PT_PRESENT | PT_WRITE,
                      (uintptr_t)&kernel_start, KERNEL_PHYS_BASE);

        current_pagedir = pagedir;
        /* swap the temporary page table with our identical, but more
         * permanant page table */
        pt_set(pagedir);

        uintptr_t physmax = phys_detect_highmem();
        dbgq(DBG_MM, "Highest usable physical memory: 0x%08x\n", physmax);
        dbgq(DBG_MM, "Available memory: 0x%08x\n", physmax - KERNEL_PHYS_BASE);

        uintptr_t vaddr = ((uintptr_t)&kernel_start);
        uintptr_t paddr = KERNEL_PHYS_BASE;
        do {
                pagetable += PT_ENTRY_COUNT;
                vaddr += PT_VADDR_SIZE;
                paddr += PT_VADDR_SIZE;
                _pt_fill_page(pagedir, pagetable, PD_PRESENT | PD_WRITE, PT_PRESENT | PT_WRITE, vaddr, paddr);
        } while (paddr < physmax);

        page_add_range((uintptr_t) pagetable + PT_ENTRY_COUNT, physmax + ((uintptr_t)&kernel_start) - KERNEL_PHYS_BASE);
}

void
pt_template_init()
{
        /* the current page directory should be the same one set up by
         * the pt_init function above, it needs to be slighly modified
         * to remove the mapping of the first 4mb and then saved in a
         * seperate page as the template */
        memset(current_pagedir->pd_virtual[0], 0, PAGE_SIZE);
        tlb_flush_all();

        template_pagedir = page_alloc_n(2);
        KASSERT(NULL != template_pagedir);
        memcpy(template_pagedir, current_pagedir, sizeof(*template_pagedir));

        intr_register(INTR_PAGE_FAULT, _pt_fault_handler);
}

/* Debugging information to print human-readable information about
 * a struct pagedir. */
size_t
pt_mapping_info(const void *pt, char *buf, size_t osize)
{
        size_t size = osize;

        KASSERT(NULL != pt);
        KASSERT(NULL != buf);

        const struct pagedir *pagedir = pt;
        uintptr_t vstart, pstart;
        uintptr_t pexpect;
        uint32_t pdi = 0;
        uint32_t pti = 0;
        int started = 0;

        while (PT_ENTRY_COUNT > pdi) {
                pte_t *entry = NULL;
                if (PD_PRESENT & pagedir->pd_physical[pdi]) {
                        if (PT_PRESENT & pagedir->pd_virtual[pdi][pti]) {
                                entry = &pagedir->pd_virtual[pdi][pti];
                        }
                } else {
                        ++pdi;
                        pti = 0;
                }

                int present = (NULL != entry);
                pexpect += PAGE_SIZE;
                if (present && !started) {
                        started = 1;
                        vstart = (pdi * PT_ENTRY_COUNT + pti) * PAGE_SIZE;
                        pstart = *entry & PAGE_MASK;
                        pexpect = pstart;
                } else if ((started && !present)
                           || (started && present && ((*entry & PAGE_MASK) != pexpect))) {
                        uintptr_t vend = (pdi * PT_ENTRY_COUNT + pti) * PAGE_SIZE;
                        uintptr_t pend = pstart + (vend - vstart);

                        started = 0;
                        iprintf(&buf, &size, "%#.8x-%#.8x => %#.8x-%#.8x\n",
                                vstart, vend, pstart, pend);
                }

                if (++pti == PT_ENTRY_COUNT) {
                        ++pdi;
                        pti = 0;
                }
        }

        return osize - size;
}
