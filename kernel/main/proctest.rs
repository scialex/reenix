// TODO Copyright Header

use procs::kproc;
use procs::kmutex::KMutex;
use procs::kmutex;
use libc::c_void;
use core::prelude::*;
use core::ptr::*;
use collections::string;
use core::fmt::*;
use procs::kthread;
use core::mem::transmute_copy;

const GOOD : *mut c_void = 1 as *mut c_void;
const BAD  : *mut c_void = 0 as *mut c_void;

pub fn start() {
    let mut total : uint = 0;
    let mut pass : uint = 0;
    macro_rules! basic_test(
        ($name:expr, $v:expr) => ({
            let cnt1 = kproc::KProc::new(string::String::from_str(stringify!($name)), $name, $v, 0 as *mut c_void);
            match kproc::KProc::waitpid(kproc::Pid(cnt1), 0) {
                Ok((pid, status)) => {
                    total += 1;
                    if status == GOOD as kproc::ProcStatus {
                        dbg!(debug::TESTPASS, "Test {} {} passes", total, stringify!($name));
                        pass += 1;
                    } else {
                        dbg!(debug::TESTFAIL, "Test {} {} failed with {}", total, stringify!($name), status);
                    }
                },
                Err(errno) => {
                    total += 1;
                    dbg!(debug::TESTFAIL, "test {} {} failed with errno {}", total, stringify!($name), errno);
                }
            }
        });
        ($name:expr) => (basic_test!($name, 0))
    )
    basic_test!(uncontested_mutex);
    basic_test!(contested_mutex, 1);
    basic_test!(contested_mutex, 2);
    basic_test!(contested_mutex, 5);

    dbg!(debug::TEST, "passed {} of {} tests", pass, total);
}

extern "C" fn uncontested_mutex(_: i32, _: *mut c_void) -> *mut c_void {
    dbg!(debug::TEST, "Attempting to create a mutex and lock it.");
    let x = kmutex::KMutex::new("test a mutex");
    x.lock();
    dbg!(debug::TEST, "locking of mutex succeeded");
    x.unlock();
    dbg!(debug::TEST, "unlocking of mutex succeeded");
    return GOOD;
}

static mut c_mutex : *mut KMutex  = 0 as *mut KMutex;
static mut cnt : i32 = 0;

fn get_c_mutex() -> &'static KMutex {
    unsafe { c_mutex.as_ref().expect("CMutex is not set") }
}

extern "C" fn contested_mutex(n : i32, _: *mut c_void) -> *mut c_void {
    use base::debug;
    debug::remove_mode(debug::SCHED);
    let y = unsafe {
        let x = box KMutex::new("contested mutex test");
        c_mutex = transmute_copy(&x);
        x
    };

    let high : i32 = 200;

    for _ in range(0, n) {
        // TODO How to make this say which number they are?
        let cnt1 = kproc::KProc::new(string::String::from_str("counter n"), counter, high, 0 as *mut c_void);
    }

    let mut tot : i32 = 0;
    for _ in range(0, n) {
        let (p, v) = match kproc::KProc::waitpid(kproc::Any, 0) {
            Ok(e) => e,
            Err(e) => { return BAD; },
        };
        dbg!(debug::TEST, "pid {} returned {}", p, v);
        tot += v as i32;
    }
    let ret = if tot == unsafe { cnt } {
        dbg!(debug::TESTPASS, "successfully counted to {} with {} counters", tot, n);
        GOOD
    } else {
        dbg!(debug::TESTFAIL, "failed counted to {} with {} counters, got {}", high, n, tot);
        BAD
    };
    unsafe { cnt = 0; c_mutex = 0 as *mut KMutex; }
    debug::add_mode(debug::SCHED);
    return ret;
}

extern "C" fn counter(h: i32, _ : *mut c_void) -> *mut c_void {
    let mut c : uint = 0;
    loop {
        get_c_mutex().lock();
        if c % 2 == 0 {
            kthread::kyield();
        }
        if unsafe { cnt == h } {
            get_c_mutex().unlock();
            return c as *mut c_void;
        } else {
            unsafe {cnt += 1; }
            c += 1;
            if c % 5 == 0 {
                kthread::kyield();
            }
            get_c_mutex().unlock();
            kthread::kyield();
        }
    }
}
