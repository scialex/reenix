// TODO Copyright Header

//! # The Reenix base util stuff.

#![crate_name="base"]
#![crate_type="rlib"]
#![no_std]
#![doc(html_logo_url = "https://avatars.io/gravatar/d0ad9c6f37bb5aceac2d7ac95ba82607?size=large",
       html_favicon_url="https://avatars.io/gravatar/d0ad9c6f37bb5aceac2d7ac95ba82607?size=small")]
#![feature(asm, concat_idents, lang_items, plugin, unboxed_closures)]
#![feature(core, hash)]

#[plugin] #[no_link] #[macro_use] extern crate bassert;
#[plugin] #[no_link] #[macro_use] extern crate enabled;
#[macro_use] extern crate core;
extern crate libc;

//pub use errno::*;
pub use core::nonzero;

// NOTE Needs to go first so everything else can get the macro's defined in it.
#[macro_use] mod bitflags;
#[macro_use] mod macros;
#[macro_use] pub mod debug;

pub mod make;

pub mod devices;

pub mod gdb;

pub mod errno;

pub mod io;

pub mod kernel;
pub mod sync;
pub mod cell;

pub mod pid;
pub fn init_stage1() { debug::setup(); }
pub fn init_stage2() {}

// NOTE Needed for the #[deriving] stuff to work. Because that makes sense.
#[doc(hidden)]
mod std {
    pub use core::cmp;
    pub use core::fmt;
    pub use core::option;
    pub use core::num;
    pub use core::default;
    pub use core::clone;
    pub use core::marker;
    pub use core::ops;
    pub use core::iter;
    pub use core::hash;
}
mod base { pub use debug; pub use kernel; }
