// TODO Copyright Header
#![crate_name="startup"]
#![crate_type="rlib"]
#![no_std]
#![feature(asm, macro_rules, globs, concat_idents,lang_items, trace_macros, phase, intrinsics)]

//! # The Reenix startup stuff.
///
/// This contains the definitions for various startup related pieces including gdt,
/// PIT. Currently this is mostly just stubs but it may become rust in the future.


// TODO This should be placed before base in initialization and we should
// remove the pci stuff to drivers.

#[phase(plugin, link)] extern crate core;
#[phase(plugin, link)] extern crate base;
extern crate mm;
extern crate libc;

#[no_split_stack]
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
    pub static INTERRUPT : u8 = 0xf1;
    pub fn init_stage1() { not_yet_implemented!(pit::init_stage1) }
    pub fn init_stage2() { not_yet_implemented!(pit::init_stage2) }
    pub fn set_handler(_h: extern fn()) { not_yet_implemented!(pit::set_handler) }
}

/// Thread specific data support.
pub mod tsd {
    use core::option::*;
    use core::iter::range;
    use libc::c_void;
    use core::fmt;
    pub fn init_stage1() {
        use mm::alloc::request_slab_allocator;
        use core::intrinsics::size_of;
        request_slab_allocator("Thread Specific Data (TSD) allocator", unsafe { size_of::<TSDInfo>() as u32 });
    }

    pub fn init_stage2() {}

    #[cfg(target_arch="x86")]
    #[repr(C, packed)]
    pub struct TSDInfo {
        vlow : [*mut c_void, ..11],
        stack_high : u32, // At offset 0x30
        open_slot : u16,
    }

    // TODO Write an implementation like pthread_setspecific.
    // TODO Write a set_stack_bottom function.
    impl TSDInfo {
        pub fn new(high: u32) -> TSDInfo { TSDInfo{vlow: [0 as *mut c_void, ..11], stack_high: high, open_slot: 1 << 12 } }
        pub fn set_slot(&mut self, i: u8, v: *mut c_void) {
            assert!(i != 12, "cannot set reserved slot.");
            self.open_slot |= 1 << (i as uint);
            self.vlow[i as uint] = v;
        }

        pub fn get_slot(&mut self, i: u8) -> Option<*mut c_void> {
            if 0 != self.open_slot & 1 << (i as uint) {
                Some(self.vlow[i as uint])
            } else {
                None
            }
        }

        pub fn set_open_slot(&mut self, v: *mut c_void) -> Option<u8> {
            for i in range(0,12) {
                if !self.is_slot_used(i) {
                    self.set_slot(i, v);
                    return Some(i);
                }
            }
            return None;
        }

        pub fn is_slot_used(&mut self, i : u8) -> bool {
            self.get_slot(i).is_some()
        }
    }
    impl fmt::Show for TSDInfo {
        fn fmt(&self, f : &mut fmt::Formatter) -> fmt::Result {
            write!(f, "TSDInfo {{ [{:p}, {:p}, {:p}, {:p}, {:p}, {:p}, {:p}, {:p}, {:p}, {:p}, {:p}], stack_high: 0x{:x} }}",
                      self.vlow[0],self.vlow[1],self.vlow[2],self.vlow[3],self.vlow[4],self.vlow[5],self.vlow[6],self.vlow[7],
                      self.vlow[8],self.vlow[9],self.vlow[10], self.stack_high)
        }
    }
    pub static initial_tsd : TSDInfo = TSDInfo { vlow: [0 as *mut c_void, ..11], stack_high: 0, open_slot: 0 };
}

// TODO I should move this to rust.
pub mod gdt {
    use libc::{c_void, c_int};
    use core::ptr::*;
    pub static ZERO        : u16 = 0;
    pub static KERNEL_TEXT : u16 = 0x08;
    pub static KERNEL_DATA : u16 = 0x10;
    pub static USER_TEXT   : u16 = 0x18;
    pub static USER_DATA   : u16 = 0x20;
    pub static TSS         : u16 = 0x28;
    pub static THREAD_SPECIFIC : u32 = 0x40;
    extern "C" {
        fn gdt_init();

        #[link_name = "gdt_set_kernel_stack"]
        pub fn set_kernel_stack(addr: *mut c_void);

        #[link_name = "gdt_set_entry"]
        fn set_entry(segment: u32, base: *const c_void, limit: u32, ring: u8, exec: c_int, dir: c_int, rw: c_int);
    }

    extern "rust-intrinsic" {
        fn transmute<T,U>(val: T) -> U;
    }
    #[no_split_stack]
    pub fn init_stage1() {
        unsafe { gdt_init(); }
        unsafe {
            set_entry(THREAD_SPECIFIC as u32, transmute(&::tsd::initial_tsd), 0x100, 0, 0, 0, 0);
            asm!("mov $$0x40, %ax; mov %ax, %gs": : : "eax");
        }
    }

    pub fn init_stage2() {}

    #[no_split_stack]
    pub fn set_tsd(ptr: *const ::tsd::TSDInfo) {
        use core::mem::size_of;
        unsafe {
            set_entry(THREAD_SPECIFIC as u32, ptr as *const c_void, size_of::<::tsd::TSDInfo>() as u32, 0, 0, 0, 0);
        }
    }

    pub fn get_tsd() -> &'static mut ::tsd::TSDInfo {
        let ret : *mut ::tsd::TSDInfo;
        unsafe {
            asm!("leal %gs:0, $0" : "=r"(ret) : : : "volatile");
            return ret.as_mut().expect("Illegal value for %gs");
        }
    }
}


mod std {
    pub use core::fmt;
}
