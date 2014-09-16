// TODO Copyright Header

//! # The Reenix base util stuff.

#![crate_name="base"]
#![crate_type="rlib"]
#![no_std]
#![allow(missing_doc)]
#![feature(asm, macro_rules, globs, log_syntax)]


extern crate core;
extern crate libc;

pub use errno::*;

// NOTE Needs to go first so everything else can get the macro's defined in it.
mod bitflags;

mod errno;

pub mod io;
pub mod debug;
pub mod kernel;

// NOTE Needed for the #[deriving] stuff to work. Because that makes sense.
mod std {
    pub use core::cmp;
    pub use core::fmt;
    pub use core::option;
    pub use core::num;
}
