// TODO Copyright Header

//! A very basic libstd front
//!
//! This is where all the stuff relating to processes is, including context switching, interrupts,
//! and processes/threads. Because of order of initialization and their use in interrupt handling
//! acpi and apic are in here as well.

#![crate_name="basicstd"]
#![crate_type="rlib"]
#![doc(html_logo_url = "https://avatars.io/gravatar/d0ad9c6f37bb5aceac2d7ac95ba82607?size=large",
       html_favicon_url="https://avatars.io/gravatar/d0ad9c6f37bb5aceac2d7ac95ba82607?size=small")]

#![no_std]

#[macro_reexport(assert, assert_eq, debug_assert, write, writeln)]
#[macro_use] extern crate "core" as rcore;
#[macro_use] extern crate "collections" as rcollections;
extern crate "rand" as rrand;
extern crate "alloc" as ralloc;

pub use ralloc::{boxed, rc};
pub use rcore::{any, borrow, cell, char, clone, cmp, default};
pub use rcore::{f32, f64, finally, hash, i16, i32, i64, i8, int, intrinsics};
pub use rcore::{isize, iter, marker, mem, num, ops, option, panicking, ptr, raw};
pub use rcore::{result, simd, slice, str, u16, u32, u64, u8, uint, usize};

pub use rcollections::vec;
pub use rcollections::string;

#[macro_use] mod macros;

pub mod collections {
    pub use rcollections::*;
}

pub mod rand {
    pub use ::rrand::*;
}

pub mod sync {
    pub use rcore::atomic;
    pub use ralloc::arc::{Arc, Weak};
}

pub mod fmt {
    pub use rcore::fmt::*;
    pub fn format(args: Arguments) -> ::string::String {
        let mut output = ::string::String::new();
        let _ = write!(&mut output, "{}", args);
        output
    }
}

pub mod error;

pub mod rt {
    pub use ralloc::heap;

    pub fn begin_unwind(msg: &str, fl: &(&'static str, usize)) -> ! {
        ::rcore::panicking::panic_fmt(format_args!("{}", msg), fl)
    }
    pub fn begin_unwind_fmt(msg: ::fmt::Arguments, file_line: &(&'static str, usize)) -> ! {
        ::rcore::panicking::panic_fmt(msg, file_line)
    }
}

pub mod prelude {
    pub mod v1 {
        pub use rcore::prelude::*;
        pub use ralloc::boxed::Box;
        pub use rcollections::vec::Vec;
        pub use rcollections::string::{String, ToString};
    }
}

#[doc(hidden)]
mod std {
    pub use super::*;
}
