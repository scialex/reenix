#![crate_name = "reenix"]
#![crate_type = "staticlib"]

#![no_std]

#![feature(asm, macro_rules, default_type_params, phase, globs, lang_items, intrinsics)]

// The plugin phase imports compiler plugins, including regular macros.


#[phase(plugin, link)] extern crate core;
#[phase(plugin, link)] extern crate base;
#[phase(plugin, link)] extern crate procs;
extern crate mm;
extern crate main;
extern crate startup;
extern crate libc;

use libc::c_void;

use base::debug::printing::DBG_WRITER;

extern "C" { fn dbg_init(); }

#[no_stack_check]
fn run_init() {
    use mm;
    use main;
    use procs;
    use startup;
    unsafe { dbg_init(); }

    // This sets up the gdt based stack checking.
    startup::gdt::init_stage1();
    dbg!(debug::CORE, "gdt initialized stage 1");
    base::init_stage1();
    dbg!(debug::CORE, "base initialized stage 1");
    mm::init_stage1();
    dbg!(debug::CORE, "mm initialized stage 1");
    startup::init_stage1();
    dbg!(debug::CORE, "startup initialized stage 1");
    procs::init_stage1();
    dbg!(debug::CORE, "procs initialized stage 1");
    main::init_stage1();
    dbg!(debug::CORE, "main initialized stage 1");

    mm::alloc::close_requests();

    base::init_stage2();
    dbg!(debug::CORE, "Base initialized stage 2");
    mm::init_stage2();
    dbg!(debug::CORE, "mm initialized stage 2");
    startup::init_stage2();
    dbg!(debug::CORE, "startup initialized stage 2");
    procs::init_stage2();
    dbg!(debug::CORE, "procs initialized stage 2");
    main::init_stage2();
    dbg!(debug::CORE, "main initialized stage 2");
}

#[no_mangle]
#[no_stack_check]
pub extern "C" fn kmain() {
    use main;
    run_init();
    // TODO I should call the gdb hook things.
    // TODO I should do the context switch in here.
    procs::enter_bootstrap_func(main::bootstrap, 0, 0 as *mut c_void);
}

#[cold]
#[no_mangle]
#[no_stack_check]
pub extern "C" fn __morestack() {
    panic!("__morestack called. This should happen");
}

#[cold]
#[no_mangle]
#[inline(never)]
#[lang="fail_fmt"]
#[allow(unused_must_use)]
pub extern fn rust_begin_unwind(msg: &core::fmt::Arguments,
                                file: &'static str,
                                line: uint) -> ! {
    dbg!(debug::PANIC, "Failed at {:s}:{:u}",file, line);
    unsafe { core::fmt::write(&mut DBG_WRITER, msg); }
    panic!("");
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
