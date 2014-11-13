// TODO Copyright Header

//! # The Reenix base allocation library.
//!
//! This is currently just a shim around C code. It might become rust later

#![crate_name="mm"]
#![crate_type="rlib"]
#![allow(non_camel_case_types)]
#![doc(html_logo_url = "https://avatars.io/gravatar/d0ad9c6f37bb5aceac2d7ac95ba82607?size=large",
       html_favicon_url="https://avatars.io/gravatar/d0ad9c6f37bb5aceac2d7ac95ba82607?size=small")]
#![feature(phase, globs, struct_variant, macro_rules, asm, if_let, tuple_indexing)]
#![no_std]

#[phase(link, plugin)] extern crate core;
#[phase(link, plugin)] extern crate base;
extern crate libc;

use libc::{c_void, size_t};
pub use alloc::{AllocError, Allocation};

/// Initialize this crate. This must be called exactly once during startup.
#[deny(dead_code)]
pub fn init_stage1() {
    tlb::init_stage1();
    page::init_stage1();
    pagetable::init_stage1();
    alloc::init_stage1();
}

pub fn init_stage2() {
    tlb::init_stage2();
    page::init_stage2();
    pagetable::init_stage2();
    alloc::init_stage2();
}

extern "C" {
    pub fn malloc(size: size_t) -> *mut c_void;
    pub fn free(addr: *mut c_void);
    pub fn realloc(addr: *mut c_void, size: size_t) -> *mut c_void;
}

mod mm {
    pub use super::alloc;
}

pub mod pagetable;
pub mod utils;
pub mod alloc;
mod slabmap;
mod macros;
mod backup;

pub mod poison {
    pub const ENABLED : bool = true;
    pub const ALLOC   : u8   = 0xBB;
}

#[cfg(all(kernel, target_arch="x86"))]
pub mod user {
    pub const MEM_LOW  : uint = 0x00400000;
    pub const MEM_HIGH : uint = 0xc0000000;
}

pub mod pointer {
    use core::uint;
    pub const SIZE : uint = uint::BYTES;
    pub const MASK : uint = uint::BYTES - 1;
}

pub mod memman {
    /// Mapping protection
    pub mod prot {
        pub const NONE  : int = 0x0;
        pub const READ  : int = 0x1;
        pub const WRITE : int = 0x2;
        pub const EXEC  : int = 0x4;
        pub const MASK  : int = 0x7;
    }
    pub mod map {
        /// Mapping type
        pub const SHARED  : int = 0x1;
        pub const PRIVATE : int = 0x2;
        pub const MASK    : int = 0x3;
        /// Mapping flags
        pub const FIXED : int = 0x4;
        pub const ANON  : int = 0x8;
        pub const FAILED : uint = !0;
    }
}

pub mod tlb {
    use libc::c_void;
    use core::iter::range;
    pub fn init_stage1() {}
    pub fn init_stage2() {}

    pub unsafe fn flush(vaddr : *mut c_void) {
        asm!("invlpg ($0)" : : "r"(vaddr) : "memory" : "volatile")
    }

    #[allow(unused_variables)]
    pub unsafe fn flush_range(vaddr: *mut c_void, pages: uint) {
        use super::page;
        let mut uv = vaddr as uint;
        for i in range(0, pages) {
            flush(uv as *mut c_void);
            uv += page::SIZE;
        }
    }

    pub unsafe fn flush_all() {
        let pdir : uint;
        asm!("movl %cr3, $0" : "=r"(pdir) :           :          : "volatile");
        asm!("movl $0, %cr3" :            : "r"(pdir) : "memory" : "volatile");
    }
}

pub mod page {
    use core::intrinsics::transmute;
    use libc::{uintptr_t, c_void};
    use core::result::*;
    use core::ptr::*;
    extern "C" {
        #[link_name = "page_add_range"]
        pub fn c_add_range(start: uintptr_t, end: uintptr_t);
        #[link_name = "page_alloc"]
        pub fn c_alloc() -> *mut c_void;
        #[link_name = "page_free"]
        pub fn free(page: *mut c_void);
        #[link_name = "page_alloc_n"]
        pub fn c_alloc_n(num: u32) -> *mut c_void;
        #[link_name = "page_free_n"]
        pub fn free_n(pages: *mut c_void, num: u32);
        #[link_name = "page_free_count"]
        pub fn free_count() -> u32;

        #[deny(dead_code)]
        pub fn page_init();
    }
    pub fn init_stage1() { unsafe { page_init(); } }
    pub fn init_stage2() {}

    pub const SHIFT  : uint = 12;
    pub const SIZE   : uint = 1 << SHIFT;
    pub const MASK   : uint = (!0) << SHIFT;
    pub const NSIZES : uint = 8;

    pub unsafe fn alloc<T>() -> super::Allocation<*mut T> {
        let res = c_alloc();
        if res.is_null() { Err(()) } else { Ok(res as *mut T) }
    }

    pub unsafe fn alloc_n<T>(pages: uint) -> super::Allocation<*mut T> {
        let res = c_alloc_n(pages as u32);
        if res.is_null() { Err(()) } else { Ok(res as *mut T) }
    }

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
        unsafe { transmute::<*const T, uint>(x) & (!MASK) }
    }

    #[inline]
    pub unsafe fn num_to_addr<T>(x: uint) -> *mut T {
        transmute::<uint, *mut T>(x << SHIFT)
    }

    #[inline]
    pub fn addr_to_num<T>(x: *const T) -> uint {
        unsafe { transmute::<*const T, uint>(x) >> SHIFT }
    }

    #[inline]
    pub fn aligned<T>(x: *const T) -> bool {
        0 == offset(x)
    }

    #[inline]
    pub fn same<T>(x: *const T, y: *const T) -> bool {
        unsafe { const_align_down(x) == const_align_down(y) }
    }

    // TODO Make traits implemented by uint and *const/mut T for these so we don't need to call
    // them directly.
}

mod std {
    pub use core::cmp;
    pub use core::fmt;
    pub use core::option;
    pub use core::num;
    pub use core::clone;
}
