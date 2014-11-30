// TODO Copyright Header

#![crate_name="umem"]
#![crate_type="rlib"]
#![no_std]
#![doc(html_logo_url = "https://avatars.io/gravatar/d0ad9c6f37bb5aceac2d7ac95ba82607?size=large",
       html_favicon_url="https://avatars.io/gravatar/d0ad9c6f37bb5aceac2d7ac95ba82607?size=small")]
#![feature(asm, macro_rules, globs, concat_idents, lang_items, phase, intrinsics, if_let, unsafe_destructor, tuple_indexing)]

//! # The Reenix User memory stuff.
///
/// It has things like the pframe

#[phase(plugin)] extern crate bassert;

#[phase(plugin, link)] extern crate procs;
#[phase(plugin, link)] extern crate base;
#[phase(plugin, link)] extern crate core;
#[phase(plugin, link)] extern crate util;
#[phase(plugin, link)] extern crate mm;
extern crate collections;
extern crate alloc;
extern crate libc;

// TODO We should have a MaybePinnedList that uses a LRUCache under the hood...
mod mmobj;
mod pframe;

pub fn init_stage1() {
    pframe::init_stage1();
}

pub fn init_stage2() {
    pframe::init_stage2();
}

pub fn init_stage3() {
    pframe::init_stage3();
}

#[doc(hidden)]
mod std {
    pub use core::kinds;
    pub use core::cmp;
    pub use core::fmt;
    pub use core::option;
    pub use core::num;
    pub use core::default;
    pub use core::clone;
}
