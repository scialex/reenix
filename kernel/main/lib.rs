// TODO Copyright Header

#![crate_name="main"]
#![crate_type="rlib"]
#![no_std]

#![feature(globs, phase, macro_rules, asm)]


#[phase(plugin, link)] extern crate core;
#[phase(plugin, link)] extern crate base;
#[phase(plugin, link)] extern crate procs;
extern crate alloc;
extern crate startup;
extern crate mm;
extern crate libc;
extern crate collections;
extern crate drivers;

use alloc::boxed::*;

use core::fmt::FormatWriter;
use drivers::bytedev;
use drivers::blockdev;
use mm::page;
use drivers::DeviceId;
use core::str::from_utf8;
use procs::cleanup_bootstrap_function;
use base::kernel;
use base::errno;
use procs::kproc;
use procs::kproc::{KProc, Pid, ProcId};
use core::iter::*;
use libc::c_void;
use mm::pagetable;
use core::prelude::*;
use collections::String;
use procs::interrupt;

mod proctest;

#[no_stack_check]
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
    kernel::halt();
}

// TODO
fn finish_init() {
    // TODO VFS Setup.
    drivers::init_stage3();
    interrupt::enable();
    interrupt::set_ipl(interrupt::LOW);
}
extern "C" fn idle_proc_run(_: i32, _: *mut c_void) -> *mut c_void {
    cleanup_bootstrap_function();
    dbg!(debug::CORE, "got into process {} and thread {}", current_proc!(), current_thread!());
    finish_init();
    KProc::new(String::from_str("Init Proc"), init_proc_run, 0, 0 as *mut c_void);
    let x = KProc::waitpid(Pid(ProcId(1)), 0);
    dbg!(debug::CORE, "done with waitpid");
    match x {
        Ok((pid, pst)) => { dbg!(debug::CORE, "Returned {}, 0x{:x}", pid, pst); },
        Err(errno) => {dbg!(debug::CORE, "returned errno {}", errno);}
    }
    shutdown();
}

extern "C" fn second_proc_run(_: i32, _: *mut c_void) -> *mut c_void {
    dbg!(debug::CORE, "Reached second process");
    dbg!(debug::CORE, "got into process {} and thread {}", current_proc!(), current_thread!());
    KProc::new(String::from_str("tty proc 0"), tty_proc_run, 0, 0 as *mut c_void);
    KProc::new(String::from_str("tty proc 1"), tty_proc_run, 1, 0 as *mut c_void);
    KProc::new(String::from_str("tty proc 2"), tty_proc_run, 2, 0 as *mut c_void);
    KProc::new(String::from_str("blockdev proc1"), block_dev_proc, 0, 0 as *mut c_void);
    //proctest::start();
    return 0xdeadbeef as *mut c_void;
}

extern "C" fn block_dev_proc(_: i32, _:*mut c_void) -> *mut c_void {
    // Try write
    let disk = blockdev::lookup_mut(DeviceId::create(1,0)).expect("should have tty");
    let mut buf : Box<[[u8, ..page::SIZE], ..3]> = box [[0, ..page::SIZE], ..3];
    let res = disk.write_to(0, &*buf);
    dbg!(debug::TEST, "result is {}", res);
    let res = disk.read_from(0, &mut *buf);
    dbg!(debug::TEST, "result is {}", res);
    0 as *mut c_void
}

extern "C" fn tty_proc_run(v:i32, _:*mut c_void) -> *mut c_void {
    let tty = bytedev::lookup_mut(DeviceId::create(2,v as u8)).expect("should have tty");
    loop {
        let mut arr : [u8,..256] = [0,..256];
        let size = match tty.read_from(0, arr) {
            Ok(v) => { v },
            Err(e) => { dbg!(debug::TERM, "reading failed because {}", e); return 0 as *mut c_void }
        };
        dbg!(debug::TEST, "recieved {}", from_utf8(arr.slice_to(size)).unwrap_or("<unknown>"));
        write!(bytedev::ByteWriter(tty), "recieved {}\n", from_utf8(arr.slice_to(size - 1)).unwrap_or("<unknown>"));
    }
}

extern "C" fn init_proc_run(_: i32, _: *mut c_void) -> *mut c_void {
    interrupt::enable();
    dbg!(debug::CORE, "got into process {} and thread {}", current_proc!(), current_thread!());
    KProc::new(String::from_str("test proc"), second_proc_run, 0, 0 as *mut c_void);
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

mod std {
    pub use core::fmt;
}
