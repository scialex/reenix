// TODO Copyright Header

#![crate_name="fs"]
#![crate_type="rlib"]
#![doc(html_logo_url = "https://avatars.io/gravatar/d0ad9c6f37bb5aceac2d7ac95ba82607?size=large",
       html_favicon_url="https://avatars.io/gravatar/d0ad9c6f37bb5aceac2d7ac95ba82607?size=small")]
#![feature(plugin, unsafe_destructor, unboxed_closures, box_syntax, core, alloc)]
#![plugin(bassert)]

//! # The Reenix User memory stuff.
///
/// It has things like the pframe

#[macro_use] #[no_link] extern crate bassert;

#[macro_use] extern crate base;
#[macro_use] extern crate mm;
extern crate drivers;
extern crate libc;
extern crate umem;
extern crate procs;

use std::borrow::Borrow;
use ::vnode::VNode;

pub use vfs::FileSystem;

//pub mod s5fs;
pub mod ramfs;
pub mod vnode;
pub mod vfs;

pub mod filesystem {
    #[cfg(S5FS)] pub use s5fs::*;
    #[cfg(not(S5FS))] pub use ramfs::*;
}

pub fn init_stage1() {
    filesystem::init_stage1();
}
pub fn init_stage2() {
    filesystem::init_stage2();
}
pub fn init_stage3() {
    filesystem::init_stage3();
}
pub fn shutdown() {
    filesystem::shutdown();
}

pub type InodeNum = usize;

pub trait FileSystem {
    type Real: VNode;
    type Node: Borrow<Self::Real> + Clone;
    fn get_type(&self) -> &'static str;
    fn get_fs_root<'a>(&'a self) -> Self::Node;
}
