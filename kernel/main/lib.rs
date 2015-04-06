// TODO Copyright Header

#![crate_name="main"]
#![crate_type="staticlib"]
#![doc(html_logo_url = "https://avatars.io/gravatar/d0ad9c6f37bb5aceac2d7ac95ba82607?size=large",
       html_favicon_url="https://avatars.io/gravatar/d0ad9c6f37bb5aceac2d7ac95ba82607?size=small")]

#![feature(plugin, asm, unsafe_destructor, lang_items, box_syntax)]
#![feature(alloc)]
#![feature(core)]
#![feature(libc)]
#![feature(collections)]

#![plugin(bassert)]
#[macro_use] #[no_link] extern crate bassert;
#[macro_use] extern crate base;
#[macro_use] extern crate procs;
#[macro_use] extern crate mm;
extern crate startup;
extern crate libc;
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
use procs::interrupt;

mod proctest;
mod kshell;
mod langs;

extern "C" { fn dbg_init(); }

extern "Rust" fn page_fault_temp(regs: &mut interrupt::Registers) {
    panic!("Page Fault occured! regs {:?}, proc {:?}, thr {:?}", regs, current_proc!(), current_thread!());
}

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
    interrupt::register(interrupt::PAGE_FAULT, page_fault_temp);
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
    dbg!(debug::CORE, "got into process {:?} and thread {:?}", current_proc!(), current_thread!());
    finish_init();
    match KProc::new("Init Proc".to_string(), init_proc_run, 0, 0 as *mut c_void) {
        Ok(ProcId(1)) => {},
        Ok(v) => { kpanic!("Unable to create init proc at {:?}, got one at {:?} instead", ProcId(1), v); },
        x => { kpanic!("Unable to create init proc {:?}", x); }
    }
    let pageoutd_id = match KProc::new("PageOutD".to_string(), umem::pageoutd_run, 0, 0 as *mut c_void) {
        Ok(v) => v,
        Err(_) => { kpanic!("Unable to make pageoutd!"); }
    };
    dbg!(debug::CORE, "pageoutd is {:?}", pageoutd_id);

    match KProc::waitpid(Pid(ProcId(1)), 0) {
        Ok((pid, pst)) => { dbg!(debug::CORE, "init Returned {:?}, 0x{:x}", pid, pst); },
        Err(errno) => {dbg!(debug::CORE, "init returned errno {:?}", errno);}
    }

    let pgd = KProc::get_proc(&pageoutd_id).expect("Pageoutd was reaped!?");
    pgd.borrow_mut().kill(0);
    drop(pgd);
    // match KProc::waitpid(Pid(pageoutd_id), 0) {
    //     Ok((pid, pst)) => { dbg!(debug::CORE, "pagetoutd Returned {:?}, 0x{:x}", pid, pst); },
    //     Err(errno) => {kpanic!("pageoutd returned errno {:?}", errno); }
    // }
    panic!("hi");
    shutdown();
}

extern "C" fn init_proc_run(_: i32, _: *mut c_void) -> *mut c_void {
    interrupt::enable();
    dbg!(debug::CORE, "got into process {:?} and thread {:?}", current_proc!(), current_thread!());

    kshell::start(0);
    loop {
        let x = KProc::waitpid(kproc::Any, 0);
        match x {
            Ok((pid, pst)) => { dbg!(debug::CORE, "{:?} Returned {:?} (0x{:x})", pid, pst, pst); },
            Err(errno) => {
                dbg!(debug::CORE, "returned errno {:?}", errno);
                if errno == errno::ECHILD {
                    break;
                }
            }
        }
    }
    return 0 as *mut c_void;
}

//#[doc(hidden)]
//struct Estr;
//impl fmt::Show for Estr { fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "\x08") } }
//#[doc(hidden)]
//static EMPTY_STR : Estr = Estr;


#[no_mangle]
#[no_stack_check]
#[doc(hidden)]
#[allow(improper_ctypes)]
pub extern "C" fn get_dbg_pid() -> Option<ProcId> {
    if unsafe { !IS_PROCS_UP } { None } else { Some(current_pid!()) }
}

