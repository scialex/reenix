// TODO Copyright Header
#![crate_name="startup"]
#![crate_type="rlib"]
#![no_std]
#![feature(asm, concat_idents, lang_items, intrinsics)]
#![feature(core)]
#![feature(alloc)]
#![feature(collections)]
#![doc(html_logo_url = "https://avatars.io/gravatar/d0ad9c6f37bb5aceac2d7ac95ba82607?size=large",
       html_favicon_url="https://avatars.io/gravatar/d0ad9c6f37bb5aceac2d7ac95ba82607?size=small")]

//! # The Reenix startup stuff.
///
/// This contains the definitions for various startup related pieces including gdt,
/// PIT. Currently this is mostly just stubs but it may become rust in the future.


// TODO This should be placed before base in initialization and we should
// remove the pci stuff to drivers.

#[macro_use] extern crate core;
#[macro_use] extern crate base;
extern crate mm;
extern crate collections;
extern crate alloc;
extern crate libc;

#[no_stack_check]
pub fn init_stage1() {
    // NOTE gdt is explicitly initialized by reenix.rs since it is what holds onto our stack
    // monitoring code.
    // gdt::init_stage1();
    pci::init_stage1();
    acpi::init_stage1();
    tsd::init_stage1();

    if cfg!(UPREEMPT) { pit::init_stage1(); }
}

pub fn init_stage2() {
    gdt::init_stage2();
    pci::init_stage2();
    acpi::init_stage2();
    tsd::init_stage2();

    if cfg!(UPREEMPT) { pit::init_stage2(); }
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
    pub const INTERRUPT : u8 = 0xf1;
    pub fn init_stage1() { not_yet_implemented!(pit::init_stage1) }
    pub fn init_stage2() { not_yet_implemented!(pit::init_stage2) }
}

/// Thread specific data support.
pub mod tsd {
    use alloc::boxed::*;
    use collections::*;
    use core::prelude::*;
    use core::any::Any;
    use core::fmt;
    pub fn init_stage1() {
        use mm::alloc::request_slab_allocator;
        use core::intrinsics::size_of;
        request_slab_allocator("Thread Specific Data (TSD) allocator", unsafe { size_of::<TSDInfo>() as u32 });
        request_slab_allocator("Very small object allocator", 8);
    }

    pub fn init_stage2() {}

    #[cfg(target_arch="x86")]
    #[repr(C, packed)]
    pub struct TSDInfo {
        vlow : [u8; 0x30],
        stack_high : u32, // At offset 0x30
        data : VecMap<Box<Any>>,
    }

    // TODO Write an implementation like pthread_setspecific.
    // TODO Write a set_stack_bottom function.
    impl TSDInfo {
        #[cfg(target_arch="x86")]
        pub fn new(high: u32) -> TSDInfo {
            TSDInfo{vlow: [0; 0x30], stack_high: high, data : VecMap::with_capacity(4) }
        }

        pub fn set_slot(&mut self, i: usize, v: Box<Any>) { self.data.insert(i, v); }
        pub fn get_slot(&self, i: usize) -> Option<&Box<Any>> { self.data.get(&i) }
        pub fn get_slot_mut(&mut self, i: usize) -> Option<&mut Box<Any>> { self.data.get_mut(&i) }

        pub fn set_open_slot(&mut self, v: Box<Any>) -> usize {
            for i in range(0, self.data.len() + 1) {
                if self.data.contains_key(&i) {
                    assert!(self.data.insert(i, v).is_none());
                    return i;
                }
            }
            kpanic!("Somehow we couldn't add a new item");
        }

        pub fn is_slot_used(&mut self, i : usize) -> bool { self.data.contains_key(&i) }
        pub fn pop_slot(&mut self, i : usize) -> Option<Box<Any>> { self.data.remove(&i) }
        pub fn remove_slot(&mut self, i : usize) -> bool { self.pop_slot(i).is_some() }
    }
    impl fmt::Debug for TSDInfo {
        fn fmt(&self, f : &mut fmt::Formatter) -> fmt::Result {
            write!(f, "TSDInfo {{ stack_high: 0x{:x}, {} data items }}", self.stack_high, self.data.len())
        }
    }

    #[derive(Copy)]
    #[repr(C, packed)]
    pub struct InitialTSDInfo { vlow : [u8; 0x30], stack_high : u32}
    pub static INITIAL_TSD : InitialTSDInfo = InitialTSDInfo { vlow: [0; 0x30], stack_high: 0};
}

// TODO I should move this to rust.
pub mod gdt {
    use libc::{c_void, c_int};
    use core::ptr::*;
    pub const ZERO        : u16 = 0;
    pub const KERNEL_TEXT : u16 = 0x08;
    pub const KERNEL_DATA : u16 = 0x10;
    pub const USER_TEXT   : u16 = 0x18;
    pub const USER_DATA   : u16 = 0x20;
    pub const TSS         : u16 = 0x28;
    pub const THREAD_SPECIFIC : u32 = 0x40;
    extern "C" {
        fn gdt_init();

        #[link_name = "gdt_set_kernel_stack"]
        pub fn set_kernel_stack(addr: *mut c_void);

        #[link_name = "gdt_set_entry"]
        fn set_entry(segment: u32, base: *const c_void, limit: u32, ring: u8, exec: c_int, dir: c_int, rw: c_int);

        #[link_name = "gdt_get_entry_base"]
        fn get_entry_base(segment: u32) -> *const c_void;
    }

    extern "rust-intrinsic" {
        fn transmute<T,U>(val: T) -> U;
    }
    #[no_stack_check]
    pub fn init_stage1() {
        unsafe { gdt_init(); }
        unsafe {
            // TODO Currently set_entry always makes the segment be sized in pages. This means we
            // TODO cannot really give any good guidelines about how large it should be. We should maybe
            // TODO cange this.
            set_entry(THREAD_SPECIFIC as u32, transmute(&::tsd::INITIAL_TSD), 0x1, 0, 0, 0, 0);
            asm!("mov $$0x40, %ax; mov %ax, %gs": : : "eax");
        }
    }

    pub fn init_stage2() {}

    #[no_stack_check]
    pub fn set_tsd(ptr: *const ::tsd::TSDInfo) {
        unsafe {
            set_entry(THREAD_SPECIFIC as u32, ptr as *const c_void, 1, 0, 0, 0, 0);
            asm!("mov $$0x40, %ax; mov %ax, %gs": : : "eax");
        }
    }

    pub fn get_tsd() -> &'static mut ::tsd::TSDInfo {
        unsafe {
            let ret : *mut ::tsd::TSDInfo = get_entry_base(0x40) as *mut ::tsd::TSDInfo;
            return ret.as_mut().expect("Illegal value for base of tsd segment");
        }
    }
}


#[doc(hidden)]
mod std {
    pub use core::marker;
    pub use core::fmt;
}
