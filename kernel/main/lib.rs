// TODO Copyright Header

#![crate_name="main"]
#![crate_type="rlib"]
#![no_std]

#![feature(globs, phase)]


#[phase(plugin, link)] extern crate core;
#[phase(plugin, link)] extern crate base;
extern crate alloc;

use alloc::boxed::*;

use core::iter::*;


#[no_split_stack]
fn clear_screen(background: u16) {
    for i in range(0u, 80 * 25) {
        unsafe {
            *((0xb8000 + i * 2) as *mut u16) = background << 12;
        }
    }
}

pub fn init_stage1() { }
pub fn init_stage2() { }

#[no_mangle]
#[no_split_stack]
pub fn bootstrap() {
    /* TODO Export the symbols so I can run this.
    dbg!(debug::CORE, "Kernel binary:\n");
    dbg!(debug::CORE, "  text: 0x%p-0x%p\n", &kernel_start_text, &kernel_end_text);
    dbg!(debug::CORE, "  data: 0x%p-0x%p\n", &kernel_start_data, &kernel_end_data);
    dbg!(debug::CORE, "  bss:  0x%p-0x%p\n", &kernel_start_bss, &kernel_end_bss);
    */
    dbg!(debug::MM, "hi {}", "debugging");
    clear_screen(13);
    loop {}
}

pub mod gdt {
    extern "C" {
        fn gdt_init();
    }
    pub fn init() {
        unsafe { gdt_init(); }
    }
}

mod std {
    pub use core::fmt;
}
