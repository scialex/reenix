// TODO Copyright Header

//! A very basic libstd front
//!
//! This is where all the stuff relating to processes is, including context switching, interrupts,
//! and processes/threads. Because of order of initialization and their use in interrupt handling
//! acpi and apic are in here as well.

#![crate_name="basic"]
#![crate_type="rlib"]
#![doc(html_logo_url = "https://avatars.io/gravatar/d0ad9c6f37bb5aceac2d7ac95ba82607?size=large",
       html_favicon_url="https://avatars.io/gravatar/d0ad9c6f37bb5aceac2d7ac95ba82607?size=small")]
#![feature(globs, phase, macro_rules)]

#![no_std]

#[phase(link, plugin)] extern crate "core" as rcore;
#[phase(link, plugin)] extern crate "collections" as rcollections;
extern crate "alloc" as ralloc;

pub use ralloc::{boxed, rc};
pub use rcore::{any, bool, borrow, cell, char, clone, cmp, default};
pub use rcore::{f32, f64, finally, fmt, i16, i32, i64, i8, int, intrinsics};
pub use rcore::{iter, kinds, mem, num, ops, option, panicking, ptr, raw, result};
pub use rcore::{simd, slice, str, tuple, u16, u32, u64, u8, uint, unit};

pub use rcollections::vec;

pub mod collections {
    pub use rcollections::*;
}
pub mod sync {
    pub use rcore::atomic;
    pub use ralloc::arc::{Arc, Weak};
}

pub mod rt {
    pub use ralloc::heap;
}

pub mod prelude {
    pub use rcore::prelude::*;
    pub use ralloc::boxed::Box;
    pub use rcollections::vec::Vec;
    pub use rcollections::string::*;
}

#[macro_export]
macro_rules! panic(
    () => ({
        panic!("explicit panic")
    });
    ($msg:expr) => ({
        // static requires less code at runtime, more constant data
        static _FILE_LINE: (&'static str, uint) = (file!(), line!());
        ::std::rt::begin_unwind($msg, &_FILE_LINE)
    });
    ($fmt:expr, $($arg:tt)*) => ({
        // a closure can't have return type !, so we need a full
        // function to pass to format_args!, *and* we need the
        // file and line numbers right here; so an inner bare fn
        // is our only choice.
        //
        // LLVM doesn't tend to inline this, presumably because begin_unwind_fmt
        // is #[cold] and #[inline(never)] and because this is flagged as cold
        // as returning !. We really do want this to be inlined, however,
        // because it's just a tiny wrapper. Small wins (156K to 149K in size)
        // were seen when forcing this to be inlined, and that number just goes
        // up with the number of calls to panic!()
        //
        // The leading _'s are to avoid dead code warnings if this is
        // used inside a dead function. Just `#[allow(dead_code)]` is
        // insufficient, since the user may have
        // `#[forbid(dead_code)]` and which cannot be overridden.
        #[inline(always)]
        fn _run_fmt(fmt: &::std::fmt::Arguments) -> ! {
            static _FILE_LINE: (&'static str, uint) = (file!(), line!());
            ::std::rt::begin_unwind_fmt(fmt, &_FILE_LINE)
        }
        format_args!(_run_fmt, $fmt, $($arg)*)
    });
)

#[macro_export]
macro_rules! assert(
    ($cond:expr) => (
        if !$cond {
            panic!(concat!("assertion failed: ", stringify!($cond)))
        }
    );
    ($cond:expr, $($arg:expr),+) => (
        if !$cond {
            panic!($($arg),+)
        }
    );
)

#[macro_export]
macro_rules! assert_eq(
    ($given:expr , $expected:expr) => ({
        match (&($given), &($expected)) {
            (given_val, expected_val) => {
                // check both directions of equality....
                if !((*given_val == *expected_val) &&
                     (*expected_val == *given_val)) {
                    panic!("assertion failed: `(left == right) && (right == left)` \
                           (left: `{}`, right: `{}`)", *given_val, *expected_val)
                }
            }
        }
    })
)

