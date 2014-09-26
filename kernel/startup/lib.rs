// TODO Copyright Header
#![crate_name="startup"]
#![crate_type="rlib"]
#![no_std]
#![feature(asm, macro_rules, globs, concat_idents,lang_items, trace_macros, phase)]

//! # The Reenix startup stuff.
///
/// This contains the definitions for various startup related pieces including gdt,
/// PIT. Currently this is mostly just stubs but it may become rust in the future.



#[phase(plugin, link)] extern crate core;
#[phase(plugin, link)] extern crate base;
extern crate mm;
extern crate libc;

pub fn init_stage1() {
    gdt::init_stage1();
    pci::init_stage1();

    if cfg!(UPREEMPT) { pit::init_stage1(); }
}

pub fn init_stage2() {
    gdt::init_stage2();
    pci::init_stage2();

    if cfg!(UPREEMPT) { pit::init_stage2(); }
}

// TODO Move this into rust. Put the rest of the stuff in.
pub mod pci {
    extern "C" {
        fn pci_init();
    }
    pub fn init_stage1() { unsafe { pci_init(); } }
    pub fn init_stage2() {}
}

#[allow(dead_code)]
pub mod pit {
    pub static INTERRUPT : u8 = 0xf1;
    pub fn init_stage1() { not_yet_implemented!(pit::init_stage1) }
    pub fn init_stage2() { not_yet_implemented!(pit::init_stage2) }
    pub fn set_handler(_h: extern fn()) { not_yet_implemented!(pit::set_handler) }
}

// TODO I should move this to rust.
pub mod gdt {
    use libc::c_void;
    pub static ZERO        : u16 = 0;
    pub static KERNEL_TEXT : u16 = 0x08;
    pub static KERNEL_DATA : u16 = 0x10;
    pub static USER_TEXT   : u16 = 0x18;
    pub static USER_DATA   : u16 = 0x20;
    pub static TSS         : u16 = 0x28;
    extern "C" {
        fn gdt_init();

        #[link_name = "gdt_set_kernel_stack"]
        pub fn set_kernel_stack(addr: *mut c_void);
    }
    pub fn init_stage1() {
        unsafe { gdt_init(); }
    }
    pub fn init_stage2() {}
}


mod std {
    pub use core::fmt;
}
