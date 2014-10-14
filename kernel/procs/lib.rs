// TODO Copyright Header

//! # The Reenix processes library.
//!
//! This is where all the stuff relating to processes is, including context switching, interrupts,
//! and processes/threads. Because of order of initialization and their use in interrupt handling
//! acpi and apic are in here as well.

#![crate_name="procs"]
#![crate_type="rlib"]

#![no_std]
#![feature(phase, globs, macro_rules, asm, if_let, default_type_params, unsafe_destructor)]

#[phase(link, plugin)] extern crate core;
#[phase(link, plugin)] extern crate base;
#[phase(link, plugin)] extern crate collections;
extern crate mm;
extern crate startup;
extern crate alloc;
extern crate libc;

pub use context::{enter_bootstrap_func, cleanup_bootstrap_function};
pub use context::ContextFunc;

pub fn init_stage1() {
    apic::init_stage1();
    interrupt::init_stage1();
    kqueue::init_stage1();
    kmutex::init_stage1();
    context::init_stage1();
    kthread::init_stage1();
    kproc::init_stage1();
}

pub fn init_stage2() {
    apic::init_stage2();
    interrupt::init_stage2();
    kqueue::init_stage2();
    kmutex::init_stage2();
    context::init_stage2();
    kthread::init_stage2();
    kproc::init_stage2();
}

mod procs {
    pub use super::kproc;
    pub use super::kthread;
    pub use super::interrupt;
    pub use super::kqueue;
    pub use super::pcell;
}
pub mod pcell;
mod macros;
mod kqueue;
mod context;

pub mod kthread;
pub mod kmutex;
pub mod kproc;
pub mod interrupt;


// TODO Rewrite this in rust.
mod apic {
    extern "C" {
        fn apic_init();
    }
    pub fn init_stage1() {
        unsafe { apic_init(); }
    }
    pub fn init_stage2() {}
    extern "C" {
        #[link_name = "apic_setredir"]
        pub fn set_redirect(irq: u32, intr: u8);

        #[link_name = "apic_setspur"]
        pub fn set_spurious_interrupt(intr: u8);

        #[link_name = "apic_setipl"]
        pub fn set_ipl(ipl: u8);

        #[link_name = "apic_getipl"]
        pub fn get_ipl() -> u8;
    }

    #[allow(dead_code)]
    extern "C" {
        #[link_name = "apic_enable_periodic_timer"]
        pub fn enable_periodic_timer(freq: u32);

        #[link_name = "apic_disable_periodic_timer"]
        pub fn disable_periodic_timer();

        #[link_name = "apic_eoi"]
        pub fn set_eoi();
    }
}


mod std {
    pub use core::clone;
    pub use core::cmp;
    pub use core::fmt;
    pub use core::num;
    pub use core::option;
    pub use collections::hash;
}
