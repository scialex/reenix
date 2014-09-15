// TODO Copyright Header

//! # The Reenix base util stuff.

#![crate_name="base"]
#![crate_type="rlib"]
#![no_std]
#![allow(missing_doc)]
#![feature(macro_rules, globs)]

extern crate core;
extern crate libc;

pub use errno::*;
use bitflags;
pub use debug::dbg_mode;

mod errno;

pub mod debug;
pub mod io;

pub mod kernel {
    //! The linker script will initialize these symbols. Note
    //! that the linker does not actually allocate any space
    //! for these variables (thus the void type) it only sets
    //! the address that the symbol points to. So for example
    //! the address where the kernel ends is &kernel_end,
    //! NOT kernel_end.
    use libc::c_void;
    #[allow(dead_code)]
    extern "C" {
        #[link_name="kernel_start"]
        static start : *const c_void;
        #[link_name="kernel_start_text"]
        static start_text : *const c_void;
        #[link_name="kernel_start_data"]
        static start_data : *const c_void;
        #[link_name="kernel_start_bss"]
        static start_bss : *const c_void;
        #[link_name="kernel_start_init"]
        static start_init: *const c_void;
        
        #[link_name="kernel_end"]
        static end : *const c_void;
        #[link_name="kernel_end_text"]
        static end_text : *const c_void;
        #[link_name="kernel_end_data"]
        static end_data : *const c_void;
        #[link_name="kernel_end_bss"]
        static end_bss : *const c_void;
        #[link_name="kernel_end_init"]
        static end_init: *const c_void;
    }
}

// Needed for the #[deriving] stuff to work.
mod std {
    pub use core::cmp;
    pub use core::fmt;
    pub use core::option;
    pub use core::num;
}
