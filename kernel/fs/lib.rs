// TODO Copyright Header

#![crate_name="fs"]
#![crate_type="rlib"]
#![no_std]
#![doc(html_logo_url = "https://avatars.io/gravatar/d0ad9c6f37bb5aceac2d7ac95ba82607?size=large",
       html_favicon_url="https://avatars.io/gravatar/d0ad9c6f37bb5aceac2d7ac95ba82607?size=small")]
#![feature(asm, macro_rules, globs, concat_idents, lang_items, phase, intrinsics, unsafe_destructor)]

//! # The Reenix User memory stuff.
///
/// It has things like the pframe

#[phase(plugin)] extern crate bassert;

#[phase(plugin, link)] extern crate base;
#[phase(plugin, link)] extern crate core;
#[phase(plugin, link)] extern crate mm;
extern crate drivers;
extern crate collections;
extern crate alloc;
extern crate libc;
extern crate umem;

use alloc::rc::*;

//pub mod s5fs;
pub mod ramfs;
pub mod vnode;

pub mod filesystem {
    #[cfg(S5FS)] pub use s5fs::*;
    #[cfg(not(S5FS))] pub use ramfs::*;
}

pub type InodeNum = u32;

pub trait FileSystem<T> where T: Vnode {
    fn get_type() -> &'static str;
    fn get_fs_root<'a>(&'a self) -> T + 'a;
    /// Get the refcount of the vnode on the disk, does not count references in memory.
    fn get_refcount(&self, node: &T) -> uint;
    /// Called when a VNode is deleted from memory.
    fn vnode_freed(&self, node: &T);
    fn unmount(&mut self);
    fn get_vnode<'a>(&'a self, vnode_num: InodeNum) -> T + 'a;
}
