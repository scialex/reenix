//! The base of what devices are and their id's and such. Placed here more for dependency purposes
//! than any real organizational reason. Some crates need this but don't really need to know much
//! more about drivers.

use core::fmt::{mod, Show, Formatter};
use core::prelude::*;
use errno::KResult;

/// The ID number of a device, it consists of a pair of 8 bit numbers that together identify the
/// device uniquely.
#[deriving(Eq, PartialEq, Ord, PartialOrd, Clone)]
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

#[macro_export]
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


