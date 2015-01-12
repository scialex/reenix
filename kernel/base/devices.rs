//! The base of what devices are and their id's and such. Placed here more for dependency purposes
//! than any real organizational reason. Some crates need this but don't really need to know much
//! more about drivers.

use core::cell::*;
use core::fmt::{self, Show, Formatter};
use core::prelude::*;
use errno::KResult;

/// The ID number of a device, it consists of a pair of 8 bit numbers that together identify the
/// device uniquely.
#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Copy)]
pub struct DeviceId(pub u16);
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

/// Create a device ID in a static manner.
#[macro_export]
macro_rules! DeviceId_static {
    ($h:expr, $l:expr) => ( DeviceId((($h as u16) << 8) | ($l as u16)) )
}

/// A device capable of reading in units of `T`.
pub trait RDevice<T> {
    /// Read buf.len() objects from the device starting at offset. Returns the number of objects
    /// read from the stream, or errno if it fails.
    fn read_from(&self, offset: uint, buf: &mut [T]) -> KResult<uint>;
}

/// A device capable of writing in units of `T`.
pub trait WDevice<T> {
    /// Write the buffer to the device, starting at the given offset from the start of the device.
    /// Returns the number of bytes written or errno if an error happens.
    fn write_to(&self, offset: uint, buf: &[T]) -> KResult<uint>;
}

// NOTE Doing this feels really icky. It's basically due to a disconnect between the userland view
// of a device as immutable and the kernel land knowledge that some use mutation.

/// A device capable of reading in units of `T` when mutably held.
pub trait RDeviceMut<T> {
    /// Read buf.len() objects from the device starting at offset. Returns the number of objects
    /// read from the stream, or errno if it fails.
    fn read_from(&mut self, offset: uint, buf: &mut [T]) -> KResult<uint>;
}

/// A device capable of writing in units of `T` when mutably held.
pub trait WDeviceMut<T> {
    /// Write the buffer to the device, starting at the given offset from the start of the device.
    /// Returns the number of bytes written or errno if an error happens.
    fn write_to(&mut self, offset: uint, buf: &[T]) -> KResult<uint>;
}

impl<T, D> RDevice<T> for UnsafeCell<D> where D: RDeviceMut<T> {
    fn read_from(&self, offset: uint, buf: &mut [T]) -> KResult<uint> {
        // TODO I might want to replace this with a trait that just lets us do the deref, that
        // TODO would let us keep more safety.
        unsafe { self.get().as_mut() }.expect("illegal cell state").read_from(offset, buf)
    }
}

impl<T, D> WDevice<T> for UnsafeCell<D> where D: WDeviceMut<T> {
    fn write_to(&self, offset: uint, buf: &[T]) -> KResult<uint> {
        unsafe { self.get().as_mut() }.expect("illegal cell state").write_to(offset, buf)
    }
}

/// A Device that can both read and write.
pub trait Device<T> : WDevice<T> + RDevice<T> + 'static {}

impl<T,D> Device<T> for UnsafeCell<D> where D: RDeviceMut<T> + WDeviceMut<T> + 'static {}
