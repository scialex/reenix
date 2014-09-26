// TODO Copyright Header

//! # The Reenix processes library.
//!
//! This is where all the stuff relating to processes is, including context switching, interrupts,
//! and processes/threads. Because of order of initialization and their use in interrupt handling
//! acpi and apic are in here as well.

#![crate_name="procs"]
#![crate_type="rlib"]

#![no_std]
#![feature(phase, globs, macro_rules, asm)]

#[phase(link, plugin)] extern crate core;
#[phase(link, plugin)] extern crate base;
extern crate startup;
extern crate alloc;
extern crate libc;
use base::debug;

pub fn init_stage1() {
    acpi::init_stage1();
    apic::init_stage1();
    interrupt::init_stage1();
}
pub fn init_stage2() {
    acpi::init_stage2();
    apic::init_stage2();
    interrupt::init_stage2();
}

pub mod acpi {
    extern "C" {
        fn acpi_init();
    }

    pub fn init_stage1() {
        unsafe { acpi_init(); }
    }
    pub fn init_stage2() {}
}

// TODO Rewrite this in rust.
pub mod apic {
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

        #[link_name = "apic_enable_periodic_timer"]
        pub fn enable_periodic_timer(freq: u32);

        #[link_name = "apic_disable_periodic_timer"]
        pub fn disable_periodic_timer();

        #[link_name = "apic_setspur"]
        pub fn set_spurious_interrupt(intr: u8);

        #[link_name = "apic_setipl"]
        pub fn set_ipl(ipl: u8);

        #[link_name = "apic_getipl"]
        pub fn get_ipl() -> u8;

        #[link_name = "apic_eoi"]
        pub fn set_eoi();
    }
}


pub mod interrupt;

mod std {
    pub use core::clone;
    pub use core::cmp;
    pub use core::fmt;
    pub use core::num;
    pub use core::option;
}
