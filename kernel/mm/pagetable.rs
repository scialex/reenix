// TODO Copyright Header

use user;
use page;
use libc::{uintptr_t,c_void};
use base::errno;
use base::errno::KResult;
use core::u32;
use core::prelude::*;
use core::intrinsics::copy_nonoverlapping_memory;
use core::ptr::{zero_memory,null,null_mut};
use core::mem::uninitialized;

// TODO Make this bitflags.
pub const PRESENT        : uint = 0x001;
pub const WRITE          : uint = 0x002;
pub const USER           : uint = 0x004;
pub const WRITE_THROUGH  : uint = 0x008;
pub const CACHE_DISABLED : uint = 0x010;
pub const ACCESSED       : uint = 0x020;
pub const DIRTY          : uint = 0x040;
pub const SIZE           : uint = 0x080;
pub const GLOBAL         : uint = 0x100;

pub const ENTRY_COUNT : uint = page::SIZE / u32::BYTES;
pub const VADDR_SIZE  : uint = page::SIZE * ENTRY_COUNT;

type pte = uint;
type pde = uint;

#[repr(C, packed)]
pub struct PageDir {
    pd_physical : [pde, .. ENTRY_COUNT],
    pd_virtual  : [*mut pte, .. ENTRY_COUNT],
}

impl PageDir {
    pub fn new() -> PageDir {
        assert!(template_pagedir != null());
        unsafe {
            let mut ret = uninitialized();
            let r : *mut PageDir = &mut ret;
            copy_nonoverlapping_memory(r, template_pagedir, 1);
            ret
        }
    }

    fn get_pagetable(&self, i: uint) -> Option<*mut uint> {
        if PRESENT & self.pd_physical[i] != 0 {
            let res = self.pd_virtual[i];
            assert!(res != null_mut());
            Some(res)
        } else {
            None
        }
    }

    pub unsafe fn set_active(&self) {
        pt_set(self as *const PageDir);
    }

    pub unsafe fn map(&mut self, vaddr: uint, paddr: uint, pdflags: uint, ptflags: uint) -> KResult<()> {
        assert!(page::aligned(vaddr as *const c_void));
        assert!(user::MEM_LOW <= vaddr && vaddr <= user::MEM_HIGH);
        assert!((pdflags & !page::MASK) == pdflags);
        let index = vaddr_to_pdindex(vaddr);
        let pt = match self.get_pagetable(index) {
            None => {
                let paget = try!(page::alloc().or_else(|_| { Err(errno::ENOMEM) }));
                zero_memory(paget, ENTRY_COUNT);
                self.pd_physical[index] = self.virt_to_phys(paget as uint) | pdflags;
                self.pd_virtual[index] = paget;
                paget
            },
            Some(_) => {
                self.pd_physical[index] |= pdflags;
                self.pd_virtual[index]
            }
        };

        let ptindex = vaddr_to_ptindex(vaddr);
        *pt.offset(ptindex as int) = paddr | ptflags;
        return Ok(());
    }

    pub unsafe fn unmap(&mut self, vaddr: uint) {
        assert!(page::aligned(vaddr as *const c_void), "request to unmap not page-aligned value");
        assert!(user::MEM_LOW <= vaddr && vaddr <= user::MEM_HIGH, "Request to unmap memory outside of allowable range");
        if let Some(x) = self.get_pagetable(vaddr_to_pdindex(vaddr)) {
            *x.offset(vaddr_to_ptindex(vaddr) as int) = 0;
        }
    }

    pub unsafe fn unmap_range(&mut self, low: uint, high: uint) {
        use core::ptr::zero_memory;
        let mut vhigh = high;
        let mut vlow = low;
        assert!(vlow < vhigh);
        assert!(page::aligned(vlow as *const c_void) && page::aligned(vhigh as *const c_void));
        assert!(user::MEM_LOW <= vlow && user::MEM_HIGH >= vhigh);

        if let Some(pt) = self.get_pagetable(vaddr_to_pdindex(vlow)) {
            let index = vaddr_to_ptindex(vlow);
            if index != 0 {
                let cnt = ENTRY_COUNT - index;
                zero_memory(pt.offset(index as int), cnt);
                vlow += page::SIZE * ((ENTRY_COUNT - index) % ENTRY_COUNT);
            }
        }

        if let Some(pt) = self.get_pagetable(vaddr_to_pdindex(vhigh)) {
            let index = vaddr_to_ptindex(vhigh);
            if index != 0 {
                zero_memory(pt, index);
                vhigh -= page::SIZE * index;
            }
        }

        assert!(0 == vaddr_to_ptindex(vlow));
        assert!(0 == vaddr_to_ptindex(vhigh));

        for i in range(vaddr_to_pdindex(vlow), vaddr_to_pdindex(vhigh)) {
            if let Some(x) = self.get_pagetable(i) {
                page::free(x as *mut c_void);
                self.delete_page(i);
            }
        }
    }

    pub fn delete_page(&mut self, index: uint) {
        self.pd_physical[index] = 0;
        self.pd_virtual[index] = 0 as *mut uint;
    }

    pub fn virt_to_phys(&self, vaddr: uint) -> uint {
        // TODO Rewrite this in rust.
        unsafe { base_virt_to_phys(vaddr as u32) as uint }
        /*
        // TODO I am not sure if this is right.
        let table = vaddr_to_pdindex(vaddr);
        let entry = vaddr_to_ptindex(vaddr);
        let offset = vaddr_to_offset(vaddr);

        let res = if let Some(pt) = self.get_pagetable(table) {
            let page = unsafe { *(pt.offset(entry as int)) & page::MASK };
            if page != 0 {
                page + offset
            } else {
                kpanic!("Illegal virtual address 0x{:8X} given which isn't mapped", vaddr)
            }
        } else {
            kpanic!("Illegal virtual address 0x{:8X} given which isn't mapped", vaddr)
        };
        let real =  unsafe {base_virt_to_phys(vaddr as u32)};
        assert!(res as uintptr_t == real, "we calculated paddr 0x{:x} but actually is 0x{:x}", res, real);
        res
        */
    }
}

#[inline] pub fn vaddr_to_pdindex(vaddr: uint) -> uint { ((vaddr) >> page::SHIFT) / ENTRY_COUNT }
#[inline] pub fn vaddr_to_ptindex(vaddr: uint) -> uint { ((vaddr) >> page::SHIFT) % ENTRY_COUNT }
#[inline] pub fn vaddr_to_offset (vaddr: uint) -> uint { vaddr & page::MASK }

impl Drop for PageDir {
    fn drop(&mut self) {
        let begin = user::MEM_LOW / VADDR_SIZE;
        let end = (user::MEM_HIGH - 1) / VADDR_SIZE;
        assert!(begin < end && begin > 0);

        dbg!(debug::MM, "Freeing pagedir");
        for i in range(begin, end) {
            if let Some(x) = self.get_pagetable(i) {
                unsafe { page::free(x as *mut c_void) }
            }
        }
    }
}

// TODO Maybe make these rust.
#[allow(improper_ctypes)]
extern "C" {
    static template_pagedir : *const PageDir;
    #[link_name = "current_pagedir"]
    pub static current : *const PageDir;

    /// Temporarily maps one page at the given physical address in at a
    /// virtual address and returns that virtual address. Note that repeated
    /// calls to this function will return the same virtual address, thereby
    /// invalidating the previous mapping.
    #[link_name = "pt_phys_tmp_map"]
    pub fn phys_tmp_map(paddr: uintptr_t) -> uintptr_t;

    /// Permenantly maps the given number of physical pages, starting at the
    /// given physical address to a virtual address and returns that virtual
    /// address. Each call will return a different virtual address and the
    /// memory will stay mapped forever. Note that there is an implementation
    /// defined limit to the number of pages available and using too many
    /// will cause the kernel to panic.
    #[link_name = "pt_phys_perm_map"]
    pub fn phys_perm_map(paddr: uintptr_t, count: u32) -> uintptr_t;

    /// Looks up the given virtual address (vaddr) in the current page
    /// directory, in order to find the matching physical memory address it
    /// points to. vaddr MUST have a mapping in the current page directory,
    /// otherwise this function's behavior is undefined */
    #[link_name = "pt_virt_to_phys"]
    pub fn base_virt_to_phys(vaddr: uintptr_t) -> uintptr_t;

    #[deny(dead_code)]
    fn pt_template_init();

    #[deny(dead_code)]
    fn pt_init();

    #[link_name = "pt_set"]
    fn pt_set(pd: *const PageDir);
}

pub fn init_stage1() { unsafe { pt_init(); } }
pub fn init_stage2() {}
pub fn template_init() { unsafe { pt_template_init(); } }

