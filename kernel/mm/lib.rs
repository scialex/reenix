// TODO Copyright Header

//! # The Reenix base allocation library.
//!
//! This is currently just a shim around C code. It might become rust later

#![crate_name="mm"]
#![crate_type="rlib"]
#![allow(non_camel_case_types)]
#![allow(missing_doc)]
#![feature(phase, globs, struct_variant, macro_rules)]
#![no_std]

#[phase(link, plugin)] extern crate core;
#[phase(link, plugin)] extern crate base;
extern crate libc;

use libc::{c_void, size_t};

extern "C" {
    #[link_name = "kmalloc"]
    pub fn malloc(size: size_t) -> *mut c_void;
    #[link_name = "kfree"]
    pub fn free(addr: *mut c_void);
}

mod utils;
pub mod alloc;

pub mod poison {
    pub static ENABLED : bool = true;
    pub static ALLOC   : u8   = 0xBB;
}

#[cfg(kernel, target_arch="x86")]
pub mod user {
    pub static MEM_LOW  : uint = 0x00400000;
    pub static MEM_HIGH : uint = 0xc0000000;
}

pub mod pointer {
    use core::uint;
    pub static SIZE : uint = uint::BYTES;
    pub static MASK : uint = uint::BYTES - 1;
}

pub mod memman {
    /// Mapping protection
    pub mod prot {
        pub static NONE  : int = 0x0;
        pub static READ  : int = 0x1;
        pub static WRITE : int = 0x2;
        pub static EXEC  : int = 0x4;
        pub static MASK  : int = 0x7;
    }
    pub mod map {
        /// Mapping type
        pub static SHARED  : int = 0x1;
        pub static PRIVATE : int = 0x2;
        pub static MASK    : int = 0x3;
        /// Mapping flags
        pub static FIXED : int = 0x4;
        pub static ANON  : int = 0x8;
        pub static FAILED : uint = !0;
    }
}

pub mod page {
    use core::intrinsics::transmute;
    use libc::{uintptr_t, c_void};
    extern "C" {
        #[link_name = "page_add_range"]
        pub fn add_range(start: uintptr_t, end: uintptr_t);
        #[link_name = "page_alloc"]
        pub fn alloc() -> *mut c_void;
        #[link_name = "page_free"]
        pub fn free(page: *mut c_void);
        #[link_name = "page_alloc_n"]
        pub fn alloc_n(num: u32) -> *mut c_void;
        #[link_name = "page_free_n"]
        pub fn free_n(pages: *mut c_void, num: u32);
        #[link_name = "page_freecount"]
        pub fn free_count() -> u32;
    }

    pub static SHIFT  : uint = 12;
    pub static SIZE   : uint = 1 << SHIFT;
    pub static MASK   : uint = (!0) << SHIFT;
    pub static NSIZES : uint = 8;

    #[inline]
    pub unsafe fn const_align_down<T>(x: *const T) -> *const T {
        transmute::<uint, *const T>(
            transmute::<*const T, uint>(x) & MASK)
    }

    #[inline]
    pub unsafe fn align_down<T>(x: *mut T) -> *mut T {
        transmute::<uint, *mut T>(
            transmute::<*mut T, uint>(x) & MASK)
    }

    #[inline]
    pub unsafe fn align_up<T>(x: *mut T) -> *mut T {
        transmute::<uint, *mut T>(
            ((transmute::<*mut T, uint>(x) - 1) & MASK) + SIZE)
    }

    #[inline]
    pub unsafe fn const_align_up<T>(x: *const T) -> *const T {
        transmute::<uint, *const T>(
            ((transmute::<*const T, uint>(x) - 1) & MASK) + SIZE)
    }

    #[inline]
    pub fn offset<T>(x: *const T) -> uint {
        unsafe { transmute::<*const T, uint>(x) } & (!MASK)
    }

    #[inline]
    pub unsafe fn num_to_addr<T>(x: uint) -> *mut T {
        transmute::<uint, *mut T>(x << SHIFT)
    }

    #[inline]
    pub fn addr_to_num<T>(x: *const T) -> uint {
        unsafe { transmute::<*const T, uint>(x) } >> SHIFT
    }

    #[inline]
    pub fn aligned<T>(x: *const T) -> bool {
        0 == (unsafe { transmute::<*const T, uint>(x) } % SIZE)
    }

    #[inline]
    pub fn same<T>(x: *const T, y: *const T) -> bool {
        unsafe { const_align_down(x) == const_align_down(y) }
    }
}

pub mod pagetable {
    use super::page;
    use libc::uintptr_t;
    use base::errno;
    use core::u32;
    use core::prelude::*;
    use core::intrinsics::transmute;

    // TODO Make this bitflags.
    pub static PRESENT        : uint = 0x001;
    pub static WRITE          : uint = 0x002;
    pub static USER           : uint = 0x004;
    pub static WRITE_THROUGH  : uint = 0x008;
    pub static CACHE_DISABLED : uint = 0x010;
    pub static ACCESSED       : uint = 0x020;
    pub static DIRTY          : uint = 0x040;
    pub static SIZE           : uint = 0x080;
    pub static GLOBAL         : uint = 0x100;

    pub static ENTRY_COUNT : uint = page::SIZE / u32::BYTES;
    pub static VADDR_SIZE  : uint = page::SIZE * ENTRY_COUNT;

    pub type pte_t = u32;
    pub type pde_t = u32;

    #[repr(C)]
    pub struct PageDir {
        pd_physical : [pde_t, .. ENTRY_COUNT],
        pd_virtual  : [*mut uintptr_t, .. ENTRY_COUNT],
    }

    impl Drop for PageDir {
        fn drop(&mut self) {
            unsafe {
                imp::destroy_pagedir(transmute(self))
            }
        }
    }

    // TODO Maybe make these rust.
    extern "C" {

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
        pub fn virt_to_phys(vaddr: uintptr_t) -> uintptr_t;

        #[link_name = "pt_unmap"]
        pub fn unmap(pd: *mut PageDir, vaddr: uintptr_t);

        #[link_name = "pt_unmap_range"]
        pub fn unmap_range(pd: *mut PageDir, vlow: uintptr_t, vhigh: uintptr_t);

    }

    // TODO Rest of include/mm/
    #[inline]
    pub unsafe fn map(pd: *mut PageDir, vaddr: uintptr_t, paddr: uintptr_t, pdflags: u32, ptflags: u32) -> Result<(),errno::Errno> {
        use core::num;
        let a = imp::map(pd, vaddr, paddr, pdflags, ptflags);
        match num::from_i32::<errno::Errno>(a) {
            Some(errno::EOK) => Ok(()),
            Some(a) => Err(a),
            None => Err(errno::EUNKNOWN),
        }
    }

    mod imp {
        use libc::{c_int, uintptr_t};
        use super::PageDir;
        extern "C" {
            #[link_name = "pt_map"]
            pub fn map(pd: *mut PageDir, vaddr: uintptr_t, paddr: uintptr_t, pdflags: u32, ptflags: u32) -> c_int;

            #[link_name = "pt_create_pagedir"]
            pub fn create_pagedir() -> *mut PageDir;
            #[link_name = "pt_destroy_pagedir"]
            pub fn destroy_pagedir(pd: *mut PageDir);
        }
    }
}

mod std {
    pub use core::cmp;
    pub use core::fmt;
    pub use core::option;
    pub use core::num;
}
