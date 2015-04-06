// TODO Copyright Header

#![crate_name="drivers"]
#![crate_type="rlib"]
#![doc(html_logo_url = "https://avatars.io/gravatar/d0ad9c6f37bb5aceac2d7ac95ba82607?size=large",
       html_favicon_url="https://avatars.io/gravatar/d0ad9c6f37bb5aceac2d7ac95ba82607?size=small")]
#![feature(asm, concat_idents, lang_items, plugin, intrinsics, box_syntax, core, alloc, libc)]

#![plugin(bassert)]

//! # The Reenix drivers stuff.
///
/// This is all the drivers code in reenix.

#[macro_use] #[no_link] extern crate bassert;
#[macro_use] extern crate base;
#[macro_use] extern crate mm;
#[macro_use] extern crate procs;
extern crate umem;
extern crate util;
extern crate startup;
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

