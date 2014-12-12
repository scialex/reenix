// TODO Copyright Header

#![crate_name="drivers"]
#![crate_type="rlib"]
#![doc(html_logo_url = "https://avatars.io/gravatar/d0ad9c6f37bb5aceac2d7ac95ba82607?size=large",
       html_favicon_url="https://avatars.io/gravatar/d0ad9c6f37bb5aceac2d7ac95ba82607?size=small")]
#![no_std]
#![feature(asm, macro_rules, globs, concat_idents, lang_items, phase, intrinsics)]

//! # The Reenix drivers stuff.
///
/// This is all the drivers code in reenix.

#[phase(plugin)] extern crate bassert;
#[phase(plugin, link)] extern crate core;
#[phase(plugin, link)] extern crate base;
#[phase(plugin, link)] extern crate mm;
#[phase(plugin, link)] extern crate procs;
extern crate umem;
extern crate util;
extern crate startup;
extern crate collections;
extern crate alloc;
extern crate libc;

// Reexport base::devices;
pub use base::devices::*;

/// Do initialization that does not require allocating memory.
pub fn init_stage1() {
    bytedev::init_stage1();
    blockdev::init_stage1();
}

/// Do initialization that requires allocating memory.
pub fn init_stage2() {
    bytedev::init_stage2();
    blockdev::init_stage2();
}

/// Do initialization that requires running in a process context.
pub fn init_stage3() {
    bytedev::init_stage3();
    blockdev::init_stage3();
}

pub mod memdev;
pub mod bytedev;
pub mod blockdev;

#[doc(hidden)]
mod std {
    pub use core::clone;
    pub use core::cmp;
    pub use core::fmt;
    pub use core::num;
    pub use core::option;
    pub use collections::hash;
}
