// TODO Copyright Header

//! # The Reenix base allocation library.
//!
//! This is currently just a shim around C code. It might become rust later

#![crate_name="mm"]
#![crate_type="rlib"]
#![allow(non_camel_case_types)]
#![doc(html_logo_url = "https://avatars.io/gravatar/d0ad9c6f37bb5aceac2d7ac95ba82607?size=large",
       html_favicon_url="https://avatars.io/gravatar/d0ad9c6f37bb5aceac2d7ac95ba82607?size=small")]
#![feature(plugin, asm, core, no_std, unsafe_no_drop_flag, libc)]
#![no_std]

#![plugin(bassert)]
#![plugin(enabled)]
#[no_link] #[macro_use] extern crate bassert;
#[no_link] #[macro_use] extern crate enabled;
#[macro_use] extern crate core;
#[macro_use] extern crate base;
extern crate libc;

use libc::{c_void, size_t};
/// Reexports for liballoc.
pub use alloc::{AllocError, Allocation, allocate, deallocate, reallocate};
/// Reexports for liballoc.
pub use alloc::{reallocate_inplace, usable_size, stats_print};

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

#[doc(hidden)]
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
    pub const MEM_LOW  : usize = 0x00400000;
    pub const MEM_HIGH : usize = 0xc0000000;
}

pub mod pointer {
    use core::usize;
    pub const SIZE : usize = (usize::BYTES as usize);
    pub const MASK : usize = ((usize::BYTES - 1) as usize);
}

pub mod memman {
    /// Mapping protection
    pub mod prot {
        pub const NONE  : isize = 0x0;
        pub const READ  : isize = 0x1;
        pub const WRITE : isize = 0x2;
        pub const EXEC  : isize = 0x4;
        pub const MASK  : isize = 0x7;
    }
    pub mod map {
        /// Mapping type
        pub const SHARED  : isize = 0x1;
        pub const PRIVATE : isize = 0x2;
        pub const MASK    : isize = 0x3;
        /// Mapping flags
        pub const FIXED : isize = 0x4;
        pub const ANON  : isize = 0x8;
        pub const FAILED : usize = !0;
    }
}

pub mod tlb {
    use libc::c_void;
    pub fn init_stage1() {}
    pub fn init_stage2() {}

    pub unsafe fn flush(vaddr : *mut c_void) {
        asm!("invlpg ($0)" : : "r"(vaddr) : "memory" : "volatile")
    }

    #[allow(unused_variables)]
    pub unsafe fn flush_range(vaddr: *mut c_void, pages: usize) {
        use super::page;
        let mut uv = vaddr as usize;
        for i in 0..pages {
            flush(uv as *mut c_void);
            uv += page::SIZE;
        }
    }

    pub unsafe fn flush_all() {
        let pdir : usize;
        asm!("movl %cr3, $0" : "=r"(pdir) :           :          : "volatile");
        asm!("movl $0, %cr3" :            : "r"(pdir) : "memory" : "volatile");
    }
}

pub mod page {
    use core::intrinsics::transmute;
    use libc::{uintptr_t, c_void};
    use core::prelude::*;
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

    pub const SHIFT  : usize = 12;
    pub const SIZE   : usize = 1 << SHIFT;
    pub const MASK   : usize = (!0) << SHIFT;
    pub const NSIZES : usize = 8;

    pub unsafe fn alloc<T>() -> super::Allocation<*mut T> {
        let res = c_alloc();
        if res.is_null() { Err(super::AllocError) } else { Ok(res as *mut T) }
    }

    pub unsafe fn alloc_n<T>(pages: usize) -> super::Allocation<*mut T> {
        let res = c_alloc_n(pages as u32);
        if res.is_null() { Err(super::AllocError) } else { Ok(res as *mut T) }
    }

    #[inline]
    pub unsafe fn const_align_down<T>(x: *const T) -> *const T {
        transmute::<usize, *const T>(
            transmute::<*const T, usize>(x) & MASK)
    }

    #[inline]
    pub unsafe fn align_down<T>(x: *mut T) -> *mut T {
        transmute::<usize, *mut T>(
            transmute::<*mut T, usize>(x) & MASK)
    }

    #[inline]
    pub unsafe fn align_up<T>(x: *mut T) -> *mut T {
        transmute::<usize, *mut T>(
            ((transmute::<*mut T, usize>(x) - 1) & MASK) + SIZE)
    }

    #[inline]
    pub unsafe fn const_align_up<T>(x: *const T) -> *const T {
        transmute::<usize, *const T>(
            ((transmute::<*const T, usize>(x) - 1) & MASK) + SIZE)
    }

    #[inline]
    pub fn offset<T>(x: *const T) -> usize {
        unsafe { transmute::<*const T, usize>(x) & (!MASK) }
    }

    #[inline]
    pub unsafe fn num_to_addr<T>(x: usize) -> *mut T {
        transmute::<usize, *mut T>(x << SHIFT)
    }

    #[inline]
    pub fn addr_to_num<T>(x: *const T) -> usize {
        unsafe { transmute::<*const T, usize>(x) >> SHIFT }
    }

    #[inline]
    pub fn aligned<T>(x: *const T) -> bool {
        0 == offset(x)
    }

    #[inline]
    pub fn same<T>(x: *const T, y: *const T) -> bool {
        unsafe { const_align_down(x) == const_align_down(y) }
    }

    // TODO Make traits implemented by usize and *const/mut T for these so we don't need to call
    // them directly.
}

#[doc(hidden)]
mod std {
    pub use core::marker;
    pub use core::cmp;
    pub use core::fmt;
    pub use core::option;
    pub use core::num;
    pub use core::clone;
    pub use core::iter;
}
