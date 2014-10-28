// TODO Copyright Header

#![crate_name="drivers"]
#![crate_type="rlib"]
#![no_std]
#![feature(asm, macro_rules, globs, concat_idents, lang_items, phase, intrinsics, if_let)]

//! # The Reenix drivers stuff.
///
/// This is all the drivers code in reenix.

#[phase(plugin, link)] extern crate core;
#[phase(plugin, link)] extern crate base;
#[phase(plugin, link)] extern crate mm;
#[phase(plugin, link)] extern crate procs;
extern crate startup;
extern crate collections;
extern crate alloc;
extern crate libc;

use core::kinds::Sized;
use base::errno::KResult;
use core::fmt::{mod, Formatter, Show};
use core::prelude::*;

pub fn init_stage1() {
    bytedev::init_stage1();
    blockdev::init_stage1();
}

pub fn init_stage2() {
    bytedev::init_stage2();
    blockdev::init_stage2();
}
pub fn init_stage3() {
    bytedev::init_stage3();
    blockdev::init_stage3();
}

#[deriving(Eq, PartialEq, Ord, PartialOrd, Clone)]
pub struct DeviceId(u16);
impl DeviceId {
    pub fn create(major: u8, minor: u8) -> DeviceId {
        DeviceId(((major as u16) << 8) | (minor as u16))
    }
    pub fn get_major(&self) -> u8 {
        let &DeviceId(dev) = self;
        (dev >> 8) as u8
    }
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

pub trait RDevice<T> {
    /// Read buf.len() objects from the device starting at offset. Returns the number of objects
    /// read from the stream, or errno if it fails.
    fn read_from(&mut self, offset: uint, buf: &mut [T]) -> KResult<uint>;
}

pub trait WDevice<T> {
    /// Write the buffer to the device, starting at the given offset from the start of the device.
    /// Returns the number of bytes written or errno if an error happens.
    fn write_to(&mut self, offset: uint, buf: &[T]) -> KResult<uint>;
}

pub trait Device<T> : WDevice<T> + RDevice<T> + 'static + Sized {}

pub mod bytedev;
pub mod blockdev;

mod std {
    pub use core::clone;
    pub use core::cmp;
    pub use core::fmt;
    pub use core::num;
    pub use core::option;
    pub use collections::hash;
}
