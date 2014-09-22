// TODO Copyright Header

use debug::printing::*;
use core;

#[cold]
#[inline(never)]
#[no_mangle]
#[lang="begin_unwind"]
#[allow(unused_must_use)]
fn begin_unwind(msg: &core::fmt::Arguments,
                       file: &'static str,
                       line: uint) -> ! {
    unsafe { core::fmt::write(&mut DBG_WRITER, msg); }
    panic!("Begin Unwind at {:s}:{:u}",file, line);
}

#[lang="eh_personality"]
#[allow(unused_must_use)]
fn eh_personality() {
    panic!("eh_personality called");
}

#[cold]
#[inline(never)]
#[lang = "stack_exhausted"]
#[allow(unused_must_use)]
fn stack_exhausted(fmt: &core::fmt::Arguments,
                          file: &'static str,
                          line: uint) -> ! {
    unsafe { core::fmt::write(&mut DBG_WRITER, fmt); }
    panic!("Stack Exhausted at {:s}:{:u}",file, line);
}

