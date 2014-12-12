// TODO Copyright Header

#![crate_name="main"]
#![crate_type="staticlib"]
#![no_std]
#![doc(html_logo_url = "https://avatars.io/gravatar/d0ad9c6f37bb5aceac2d7ac95ba82607?size=large",
       html_favicon_url="https://avatars.io/gravatar/d0ad9c6f37bb5aceac2d7ac95ba82607?size=small")]

#![feature(globs, phase, macro_rules, asm, unsafe_destructor,lang_items)]


#[phase(plugin)] extern crate bassert;
#[phase(plugin, link)] extern crate core;
#[phase(plugin, link)] extern crate base;
#[phase(plugin, link)] extern crate procs;
#[phase(plugin, link)] extern crate mm;
extern crate alloc;
extern crate startup;
extern crate libc;
extern crate collections;
extern crate drivers;
//extern crate util;
extern crate umem;

use procs::cleanup_bootstrap_function;
use base::kernel;
use base::errno;
use procs::kproc;
use procs::kproc::{KProc, Pid, ProcId};
use libc::c_void;
use mm::pagetable;
use core::prelude::*;
use collections::String;
use procs::interrupt;
use core::fmt;

mod proctest;
mod kshell;
mod langs;

extern "C" { fn dbg_init(); }

#[no_stack_check]
fn run_init() {
    use mm;
    //use util;
    use procs;
    use startup;
    base::gdb::boot_hook();
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
    //util::init_stage1();
    //dbg!(debug::CORE, "util initialized stage 1");
    procs::init_stage1();
    dbg!(debug::CORE, "procs initialized stage 1");
    umem::init_stage1();
    dbg!(debug::CORE, "umem initialized stage 1");
    drivers::init_stage1();
    dbg!(debug::CORE, "drivers initialized stage 1");

    mm::alloc::close_requests();

    base::init_stage2();
    dbg!(debug::CORE, "Base initialized stage 2");
    mm::init_stage2();
    dbg!(debug::CORE, "mm initialized stage 2");
    startup::init_stage2();
    dbg!(debug::CORE, "startup initialized stage 2");
    //util::init_stage2();
    //dbg!(debug::CORE, "util initialized stage 2");
    procs::init_stage2();
    dbg!(debug::CORE, "procs initialized stage 2");
    umem::init_stage2();
    dbg!(debug::CORE, "umem initialized stage 2");
    drivers::init_stage2();
    dbg!(debug::CORE, "drivers initialized stage 2");
}

#[export_name="kmain"]
#[no_mangle]
#[no_stack_check]
pub extern "C" fn kmain() {
    run_init();
    // TODO I should call the gdb hook things.
    // TODO I should do the context switch in here.
    procs::enter_bootstrap_func(bootstrap, 0, 0 as *mut c_void);
}

#[no_mangle]
#[no_stack_check]
pub extern "C" fn bootstrap(_: i32, _: *mut c_void) -> *mut c_void {
    dbg!(debug::CORE, "Kernel binary:");
    dbg!(debug::CORE, "  text: {:p}-{:p}", &kernel::start_text, &kernel::end_text);
    dbg!(debug::CORE, "  data: {:p}-{:p}", &kernel::start_data, &kernel::end_data);
    dbg!(debug::CORE, "  bss:  {:p}-{:p}", &kernel::start_bss, &kernel::end_bss);

    pagetable::template_init();
    kproc::start_idle_proc(idle_proc_run, 0, 0 as *mut c_void);
}

fn shutdown() -> ! {
    dbg!(debug::CORE, "Final Shutdown");
    drivers::bytedev::shutdown();
    kernel::halt();
}

pub static mut IS_PROCS_UP : bool = false;
// TODO
fn finish_init() {
    use base::gdb;
    // TODO VFS Setup.
    procs::init_stage3();
    umem::init_stage3();
    drivers::init_stage3();
    interrupt::enable();
    interrupt::set_ipl(interrupt::LOW);
    unsafe { IS_PROCS_UP = true; }
    gdb::initialized_hook();
}

extern "C" fn idle_proc_run(_: i32, _: *mut c_void) -> *mut c_void {
    cleanup_bootstrap_function();
    dbg!(debug::CORE, "got into process {} and thread {}", current_proc!(), current_thread!());
    finish_init();
    bassert!(KProc::new(String::from_str("Init Proc"), init_proc_run, 0, 0 as *mut c_void) == Ok(ProcId(1)),
            "Unable to create init proc");
    let pageoutd_id = KProc::new(String::from_str("PageOutD"), umem::pageoutd_run, 0, 0 as *mut c_void).unwrap();
    dbg!(debug::CORE, "pageoutd is {}", pageoutd_id);

    match KProc::waitpid(Pid(ProcId(1)), 0) {
        Ok((pid, pst)) => { dbg!(debug::CORE, "init Returned {}, 0x{:x}", pid, pst); },
        Err(errno) => {dbg!(debug::CORE, "init returned errno {}", errno);}
    }

    let pgd = KProc::get_proc(&pageoutd_id).expect("Pageoutd was reaped!?");
    pgd.borrow_mut().kill(0);
    drop(pgd);
    match KProc::waitpid(Pid(pageoutd_id), 0) {
        Ok((pid, pst)) => { dbg!(debug::CORE, "pagetoutd Returned {}, 0x{:x}", pid, pst); },
        Err(errno) => {kpanic!("pageoutd returned errno {}", errno); }
    }
    shutdown();
}

extern "C" fn init_proc_run(_: i32, _: *mut c_void) -> *mut c_void {
    interrupt::enable();
    dbg!(debug::CORE, "got into process {} and thread {}", current_proc!(), current_thread!());

    kshell::start(0);
    loop {
        let x = KProc::waitpid(kproc::Any, 0);
        match x {
            Ok((pid, pst)) => { dbg!(debug::CORE, "{} Returned {} (0x{:x})", pid, pst, pst); },
            Err(errno) => {
                dbg!(debug::CORE, "returned errno {}", errno);
                if errno == errno::ECHILD {
                    break;
                }
            }
        }
    }
    return 0 as *mut c_void;
}

#[doc(hidden)]
struct Estr;
impl fmt::Show for Estr { fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "\x08") } }
#[doc(hidden)]
static EMPTY_STR : Estr = Estr;

#[no_mangle]
#[no_stack_check]
pub extern "C" fn get_dbg_pid() -> &'static (fmt::Show + 'static) {
    if unsafe { !IS_PROCS_UP } { &EMPTY_STR as &'static fmt::Show } else { ((current_pid!()) as &'static fmt::Show) }
}


#[doc(hidden)]
mod std {
    pub use core::fmt;
    pub use core::clone;
}
