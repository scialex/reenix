// TODO Copyright Header

#![crate_name="fs"]
#![crate_type="rlib"]
#![doc(html_logo_url = "https://avatars.io/gravatar/d0ad9c6f37bb5aceac2d7ac95ba82607?size=large",
       html_favicon_url="https://avatars.io/gravatar/d0ad9c6f37bb5aceac2d7ac95ba82607?size=small")]
#![feature(plugin, unsafe_destructor, unboxed_closures, box_syntax, core, alloc, libc)]
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



//pub mod s5fs;
pub mod vnode;
pub mod vfs;
pub mod ramfs;
//pub use vfs::FileSystem;

pub mod filesystem {
    #[cfg(S5FS)] pub use s5fs::*;
     pub use super::ramfs::*;
}

pub fn init_stage1() {
    ramfs::init_stage1();
}
pub fn init_stage2() {
    ramfs::init_stage2();
}
pub fn init_stage3() {
    ramfs::init_stage3();
}
pub fn shutdown() {
    ramfs::shutdown();
}

pub type InodeNum = usize;

