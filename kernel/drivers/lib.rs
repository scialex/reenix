// TODO Copyright Header

#![crate_name="drivers"]
#![crate_type="rlib"]
#![doc(html_logo_url = "https://avatars.io/gravatar/d0ad9c6f37bb5aceac2d7ac95ba82607?size=large",
       html_favicon_url="https://avatars.io/gravatar/d0ad9c6f37bb5aceac2d7ac95ba82607?size=small")]
#![no_std]
#![feature(asm, macro_rules, globs, concat_idents, lang_items, phase, intrinsics, if_let)]

//! # The Reenix drivers stuff.
///
/// This is all the drivers code in reenix.

#[phase(plugin)] extern crate bassert;
#[phase(plugin, link)] extern crate core;
#[phase(plugin, link)] extern crate base;
#[phase(plugin, link)] extern crate mm;
#[phase(plugin, link)] extern crate procs;
extern crate startup;
extern crate collections;
extern crate alloc;
extern crate libc;

use base::errno::KResult;
use core::fmt::{mod, Formatter, Show};
use core::prelude::*;

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

/// The ID number of a device, it consists of a pair of 8 bit numbers that together identify the
/// device uniquely.
#[deriving(Eq, PartialEq, Ord, PartialOrd, Clone)]
pub struct DeviceId(u16);
impl DeviceId {
    /// Create a DeviceId with the given major and minor Id numbers.
    pub fn create(major: u8, minor: u8) -> DeviceId {
        DeviceId(((major as u16) << 8) | (minor as u16))
    }
    /// Get the major device id number.
    pub fn get_major(&self) -> u8 {
        let &DeviceId(dev) = self;
        (dev >> 8) as u8
    }
    /// Get the minor device id number.
    pub fn get_minor(&self) -> u8 {
        let &DeviceId(dev) = self;
        (dev & 0xff) as u8
    }
}

impl Show for DeviceId {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result { write!(f, "DeviceId({}.{})", self.get_major(), self.get_minor()) }
}
macro_rules! DeviceId_static(
    ($h:expr, $l:expr) => ( DeviceId((($h as u16) << 8) | ($l as u16)) )
)

/// A device capable of reading in units of `T`.
pub trait RDevice<T> {
    /// Read buf.len() objects from the device starting at offset. Returns the number of objects
    /// read from the stream, or errno if it fails.
    fn read_from(&mut self, offset: uint, buf: &mut [T]) -> KResult<uint>;
}

/// A device capable of writing in units of `T`.
pub trait WDevice<T> {
    /// Write the buffer to the device, starting at the given offset from the start of the device.
    /// Returns the number of bytes written or errno if an error happens.
    fn write_to(&mut self, offset: uint, buf: &[T]) -> KResult<uint>;
}

/// A Device that can both read and write.
pub trait Device<T> : WDevice<T> + RDevice<T> + 'static + Sized {}

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
