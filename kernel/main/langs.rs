//! all of the language items we need to define.

use core;
use base::debug::printing::DBG_WRITER;

#[cold]
#[no_mangle]
#[no_stack_check]
#[allow(unused_must_use)]
pub extern "C" fn __morestack() {
    use base::debug::printing::DBG_WRITER;
    use core::fmt::*;
    use base::kernel;
    unsafe { DBG_WRITER.write(b"\n__morestack was called. We ran out of stack space!\n"); }
    kernel::halt();
}

#[cold]
#[no_mangle]
#[inline(never)]
#[lang="panic_fmt"]
pub extern fn rust_begin_unwind(msg: &core::fmt::Arguments,
                                file: &'static str,
                                line: uint) -> ! {
    use base::kernel;
    dbg!(debug::PANIC, "Failed at {:s}:{:u} -> {}",file, line, msg);
    //unsafe { core::fmt::write(&mut DBG_WRITER, msg); }
    kernel::halt();
}

#[cold]
#[inline(never)]
#[lang="eh_personality"]
pub extern fn eh_personality() {
    kpanic!("eh_personality called");
}

#[cold]
#[inline(never)]
#[lang = "stack_exhausted"]
#[allow(unused_must_use)]
pub extern fn stack_exhausted(fmt: &core::fmt::Arguments,
                              file: &'static str,
                              line: uint) -> ! {
    unsafe { core::fmt::write(&mut DBG_WRITER, fmt); }
    kpanic!("Stack Exhausted at {:s}:{:u}",file, line);
}
