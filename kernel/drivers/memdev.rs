
//! Reenix memory devices, /dev/null, /dev/zero

use mm::page;
use core::prelude::*;
use util::Cacheable;
use DeviceId;
use core::cell::*;
use core::ptr::*;
use base::{io, kernel};
use procs::interrupt;
use procs::sync::*;
use base::errno::{Errno, KResult, mod};
use libc::c_void;
use core::fmt::{mod, Formatter, Show};
use umem::mmobj::{MMObjId, MMObj};
use umem::pframe::PFrame;
use RDevice;
use WDevice;

pub fn init_stage1() {}
pub fn init_stage2() {}
// TODO Need to set this all up still.
pub fn init_stage3() {}

pub const NULL_DEVID : DeviceId = DeviceId_static!(3, 0);
pub const ZERO_DEVID : DeviceId = DeviceId_static!(4, 0);

/// The device for /dev/null
struct NullDev;

impl Cacheable for NullDev {
    fn is_still_useful(&self) -> bool { true }
}

impl WDevice<u8> for NullDev {
    /// Writes always succeed on the null device.
    fn write_to(&self, _: uint, buf: &[u8]) -> KResult<uint> { Ok(buf.len()) }
}

impl RDevice<u8> for NullDev {
    /// Reads succeed but don't get anything on the null device.
    fn read_from(&self, _: uint, buf: &mut [u8]) -> KResult<uint> { Ok(0) }
}

impl MMObj for NullDev {
    fn get_id(&self) -> MMObjId { MMObjId::new(NULL_DEVID, 0) }
    fn fill_page(&self,  pf: &mut PFrame)  -> KResult<()> { Err(Errno::ENOTSUP) }
    fn dirty_page(&self, pf: &PFrame)      -> KResult<()> { Ok(()) }
    fn clean_page(&self, pf: &PFrame)      -> KResult<()> { dbg!(debug::MEMDEV, "clean-page called on null device"); Err(Errno::ENOTSUP) }
    fn show(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "[/dev/null device]") }
}

/// The device for /dev/zero
struct ZeroDev;

impl Cacheable for ZeroDev {
    fn is_still_useful(&self) -> bool { true }
}

impl WDevice<u8> for ZeroDev {
    /// Writes always succeed on the zero device.
    fn write_to(&self, _: uint, buf: &[u8]) -> KResult<uint> { Ok(buf.len()) }
}

impl RDevice<u8> for ZeroDev {
    /// Reads succeed and get all 0's
    fn read_from(&self, _: uint, buf: &mut [u8]) -> KResult<uint> {
        for i in range(0, buf.len()) { buf[i] = 0; }
        Ok(buf.len())
    }
}

impl MMObj for ZeroDev {
    fn get_id(&self) -> MMObjId { MMObjId::new(ZERO_DEVID, 0) }
    fn fill_page(&self,  pf: &mut PFrame)  -> KResult<()> { self.read_from(0, pf.get_page_mut()).map(|_| ()) }
    fn dirty_page(&self, pf: &PFrame)      -> KResult<()> { Ok(()) }
    fn clean_page(&self, pf: &PFrame)      -> KResult<()> { dbg!(debug::MEMDEV, "clean-page called on zero device"); Err(Errno::ENOTSUP) }
    fn show(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "[/dev/zero device]") }
}
