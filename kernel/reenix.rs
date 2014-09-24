#![crate_name = "reenix"]
#![crate_type = "staticlib"]

#![no_std]

#![feature(asm, macro_rules, default_type_params, phase, globs, lang_items, intrinsics)]

// The plugin phase imports compiler plugins, including regular macros.


#[phase(plugin, link)] extern crate core;
extern crate mm;
extern crate alloc;
#[phase(plugin, link)] extern crate base;
extern crate rlibc;

use base::debug::printing::DBG_WRITER;
pub mod main;
//mod hacky;


#[cold]
#[no_mangle]
#[no_split_stack]
pub extern "C" fn __morestack() {
    panic!("__morestack called. This cannot happen");
}

#[cold]
#[no_mangle]
#[inline(never)]
#[lang="begin_unwind"]
#[allow(unused_must_use)]
pub extern fn rust_begin_unwind(msg: &core::fmt::Arguments,
                                file: &'static str,
                                line: uint) -> ! {
    unsafe { core::fmt::write(&mut DBG_WRITER, msg); }
    panic!("Begin Unwind at {:s}:{:u}",file, line);
}

#[cold]
#[inline(never)]
#[lang="eh_personality"]
#[allow(unused_must_use)]
pub extern fn eh_personality() {
    panic!("eh_personality called");
}

#[cold]
#[inline(never)]
#[lang = "stack_exhausted"]
#[allow(unused_must_use)]
pub extern fn stack_exhausted(fmt: &core::fmt::Arguments,
                          file: &'static str,
                          line: uint) -> ! {
    unsafe { core::fmt::write(&mut DBG_WRITER, fmt); }
    panic!("Stack Exhausted at {:s}:{:u}",file, line);
}

mod std {
    pub use core::fmt;
}
