#![crate_name = "reenix"]
#![crate_type = "staticlib"]

#![no_std]

#![feature(asm, macro_rules, default_type_params, phase, globs, lang_items, intrinsics)]

// The plugin phase imports compiler plugins, including regular macros.


#[phase(plugin, link)] extern crate core;
#[phase(plugin, link)] extern crate base;
extern crate mm;
extern crate main;
extern crate procs;
extern crate startup;

use base::debug::printing::DBG_WRITER;

extern "C" { fn dbg_init(); }

fn run_init() {
    use mm;
    use main;
    use procs;
    use startup;
    unsafe { dbg_init(); }

    base::init_stage1();
    mm::init_stage1();
    procs::init_stage1();
    startup::init_stage1();
    main::init_stage1();

    mm::alloc::close_requests();

    base::init_stage2();
    mm::init_stage2();
    startup::init_stage2();
    procs::init_stage2();
    main::init_stage2();
}

#[no_mangle]
#[no_split_stack]
pub extern "C" fn kmain() {
    use main;
    run_init();
    // TODO I should call the gdb hook things.
    // TODO I should do the context switch in here.
    main::bootstrap();
}

#[cold]
#[no_mangle]
#[no_split_stack]
pub extern "C" fn __morestack() {
    panic!("__morestack called. This cannot happen");
}

#[cold]
#[no_mangle]
#[inline(never)]
#[lang="fail_fmt"]
#[allow(unused_must_use)]
pub extern fn rust_begin_unwind(msg: &core::fmt::Arguments,
                                file: &'static str,
                                line: uint) -> ! {
    unsafe { core::fmt::write(&mut DBG_WRITER, msg); }
    panic!("Failed at {:s}:{:u}",file, line);
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
