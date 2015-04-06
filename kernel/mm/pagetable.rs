// TODO Copyright Header

use page;
use libc::{uintptr_t,c_int};
use base::errno;
use base::errno::KResult;
use core::u32;
use core::prelude::*;

// TODO Make this bitflags.
pub const PRESENT        : usize = 0x001;
pub const WRITE          : usize = 0x002;
pub const USER           : usize = 0x004;
pub const WRITE_THROUGH  : usize = 0x008;
pub const CACHE_DISABLED : usize = 0x010;
pub const ACCESSED       : usize = 0x020;
pub const DIRTY          : usize = 0x040;
pub const SIZE           : usize = 0x080;
pub const GLOBAL         : usize = 0x100;

pub const ENTRY_COUNT : usize = page::SIZE / (u32::BYTES as usize);
pub const VADDR_SIZE  : usize = page::SIZE * ENTRY_COUNT;

type pte = usize;
type pde = usize;

#[repr(C, packed)]
#[unsafe_no_drop_flag]
struct KPageDir {
    pd_physical : [pde; ENTRY_COUNT],
    pd_virtual  : [*mut pte; ENTRY_COUNT],
}

pub struct PageDir(*const KPageDir);

impl PageDir {
    pub fn new() -> PageDir {
        dbg!(debug::MM, "making pagedir");
        unsafe { PageDir(pt_create_pagedir()) }
    }

    pub unsafe fn set_active(&self) {
        pt_set(self.0);
    }

    pub unsafe fn map(&mut self, vaddr: usize, paddr: usize, pdflags: u32, ptflags: u32) -> KResult<()> {
        if pt_map(self.0, vaddr as uintptr_t, paddr as uintptr_t, pdflags, ptflags) == 0 {
            Ok(())
        } else {
            Err(errno::ENOMEM)
        }
    }

    pub unsafe fn unmap(&mut self, vaddr: usize) {
        pt_unmap(self.0, vaddr as uintptr_t)
    }

    pub unsafe fn unmap_range(&mut self, low: usize, high: usize) {
        pt_unmap_range(self.0, low as uintptr_t, high as uintptr_t)
    }

    pub fn virt_to_phys(&self, vaddr: usize) -> usize {
        // TODO Rewrite this in rust.
        unsafe { base_virt_to_phys(vaddr as u32) as usize }
    }
}

#[inline] pub fn vaddr_to_pdindex(vaddr: usize) -> usize { ((vaddr) >> page::SHIFT) / ENTRY_COUNT }
#[inline] pub fn vaddr_to_ptindex(vaddr: usize) -> usize { ((vaddr) >> page::SHIFT) % ENTRY_COUNT }
#[inline] pub fn vaddr_to_offset (vaddr: usize) -> usize { vaddr & page::MASK }

impl Drop for PageDir {
    fn drop(&mut self) {
        unsafe { pt_destroy_pagedir(self.0) }
    }
}

/// A super unsafe function needed to create the initial bootstrap pagedir.
/// TODO Find a better way to do this.
pub unsafe fn get_temp_init_pagedir() -> PageDir {
    PageDir(current)
}

// TODO Maybe make these rust.
#[allow(improper_ctypes)]
extern "C" {
    //static template_pagedir : *const PageDir;
    #[link_name = "current_pagedir"]
    static current : *const KPageDir;

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
    fn pt_set(pd: *const KPageDir);
    fn pt_create_pagedir() -> *const KPageDir;
    fn pt_destroy_pagedir(p: *const KPageDir);
    fn pt_map(p: *const KPageDir, v: uintptr_t, p: uintptr_t, f: u32, f2: u32) -> c_int;
    fn pt_unmap(p: *const KPageDir, v: uintptr_t);
    fn pt_unmap_range(p: *const KPageDir, l: uintptr_t, h: uintptr_t);

}

pub fn init_stage1() { unsafe { pt_init(); } }
pub fn init_stage2() {}
pub fn template_init() { unsafe { pt_template_init(); } }

