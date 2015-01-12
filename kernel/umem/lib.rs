// TODO Copyright Header

#![crate_name="umem"]
#![crate_type="rlib"]
#![no_std]
#![doc(html_logo_url = "https://avatars.io/gravatar/d0ad9c6f37bb5aceac2d7ac95ba82607?size=large",
       html_favicon_url="https://avatars.io/gravatar/d0ad9c6f37bb5aceac2d7ac95ba82607?size=small")]
#![feature(asm, concat_idents, lang_items, plugin, intrinsics, unsafe_destructor, box_syntax)]

// TODO I should maybe rename this...
//! The Reenix User memory stuff.
///
/// It has things like the pframe

#[macro_use] #[plugin] #[no_link] extern crate bassert;

#[macro_use] extern crate procs;
#[macro_use] extern crate base;
#[macro_use] extern crate core;
#[macro_use] extern crate util;
#[macro_use] extern crate mm;
extern crate startup;
extern crate collections;
extern crate alloc;
extern crate libc;

pub use pframe::pageout::{pageoutd_wakeup, pageoutd_run};

// TODO We should have a MaybePinnedList that uses a LRUCache under the hood...
pub mod mmobj;
pub mod pframe;
//pub mod vnode;

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
    pub use core::marker;
    pub use core::cmp;
    pub use core::fmt;
    pub use core::option;
    pub use core::num;
    pub use core::default;
    pub use core::clone;
}
