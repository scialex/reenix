// TODO Copyright Header

#![crate_name="fs"]
#![crate_type="rlib"]
#![doc(html_logo_url = "https://avatars.io/gravatar/d0ad9c6f37bb5aceac2d7ac95ba82607?size=large",
       html_favicon_url="https://avatars.io/gravatar/d0ad9c6f37bb5aceac2d7ac95ba82607?size=small")]
#![feature(plugin, unsafe_destructor, unboxed_closures)]

//! # The Reenix User memory stuff.
///
/// It has things like the pframe

#[macro_use] #[no_link] #[plugin] extern crate bassert;

#[macro_use] extern crate base;
#[macro_use] extern crate mm;
extern crate drivers;
extern crate libc;
extern crate umem;

use std::rc::*;
use ::vnode::VNode;

//pub mod s5fs;
pub mod ramfs;
pub mod vnode;

pub mod filesystem {
    #[cfg(S5FS)] pub use s5fs::*;
    #[cfg(not(S5FS))] pub use ramfs::*;
}

pub type InodeNum = u32;

pub trait FileSystem<T> where T: VNode {
    fn get_type() -> &'static str;
    fn get_fs_root<'a>(&'a self) -> T;
    /// Get the refcount of the vnode on the disk, does not count references in memory.
    fn get_refcount(&self, node: &T) -> usize;
    /// Called when a VNode is deleted from memory.
    fn vnode_freed(&self, node: &T);
    fn unmount(&mut self);
    fn get_vnode<'a>(&'a self, vnode_num: InodeNum) -> T;
}
