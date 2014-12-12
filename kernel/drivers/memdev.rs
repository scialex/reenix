
//! Reenix memory devices, /dev/null, /dev/zero

use core::prelude::*;
use util::Cacheable;
use DeviceId;
use base::errno::{Errno, KResult};
use core::fmt::{mod, Formatter};
use umem::mmobj::{MMObjId, MMObj};
use umem::pframe::PFrame;
use RDevice;
use WDevice;

pub fn init_stage1() {}
pub fn init_stage2() {}
// TODO Need to set this all up still.
pub fn init_stage3() {}

pub const NULL_DEVID : DeviceId = DeviceId_static!(3, 0);
pub const ZERO_DEVID : DeviceId = DeviceId_static!(3, 1);

static mut NEXT_ZERO_ID : u32 = 0;
/// The device for /dev/null
pub struct NullDev;

impl NullDev {
    pub fn new() -> NullDev { NullDev }
}

impl Cacheable for NullDev {
    fn is_still_useful(&self) -> bool { false }
}

impl WDevice<u8> for NullDev {
    /// Writes always succeed on the null device. It does nothing however.
    fn write_to(&self, _: uint, buf: &[u8]) -> KResult<uint> { Ok(buf.len()) }
}

impl RDevice<u8> for NullDev {
    /// Reads succeed but don't get anything on the null device.
    fn read_from(&self, _: uint, _: &mut [u8]) -> KResult<uint> { Ok(0) }
}

impl MMObj for NullDev {
    /// We can do LITERALLY nothing to a /dev/null. We don't need to have them being different.
    fn get_id(&self) -> MMObjId { MMObjId::new(NULL_DEVID, 0) }
    /// We fill it with 0's because thats what mmap wants.
    fn fill_page(&self,  pf: &mut PFrame)  -> KResult<()> { for i in pf.get_page_mut().iter() { *i = 0 }; Ok(()) }
    // TODO The next two maybe should panic?
    fn dirty_page(&self, _pf: &PFrame)      -> KResult<()> { Ok(()) }
    fn clean_page(&self, _pf: &PFrame)      -> KResult<()> { Ok(()) }
    fn show(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "[/dev/null device]") }
}

/// The device for /dev/zero
pub struct ZeroDev(u32);

impl ZeroDev {
    pub fn new() -> ZeroDev {
        unsafe {
            // TODO We should be using UIDGenerator...
            NEXT_ZERO_ID += 1;
            if NEXT_ZERO_ID == 0 {
                dbg!(debug::DANGER, "Amazing, we actually used 2**32 zero-devs. Maybe should use UID generator")
            }
            ZeroDev(NEXT_ZERO_ID)
        }
    }
}

impl Cacheable for ZeroDev {
    fn is_still_useful(&self) -> bool { false }
}

impl WDevice<u8> for ZeroDev {
    /// Writes always succeed on the zero device. Don't do anything however.
    fn write_to(&self, _: uint, buf: &[u8]) -> KResult<uint> { Ok(buf.len()) }
}

impl RDevice<u8> for ZeroDev {
    /// Reads succeed and get all 0's
    fn read_from(&self, _: uint, buf: &mut [u8]) -> KResult<uint> {
        for i in buf.iter_mut() { *i = 0; }
        Ok(buf.len())
    }
}

impl MMObj for ZeroDev {
    fn get_id(&self) -> MMObjId { MMObjId::new(ZERO_DEVID, self.0) }
    fn fill_page(&self,  pf: &mut PFrame)  -> KResult<()> { self.read_from(0, pf.get_page_mut()).map(|_| ()) }
    fn dirty_page(&self, _pf: &PFrame)      -> KResult<()> { Ok(()) }
    fn clean_page(&self, _pf: &PFrame)      -> KResult<()> {
        dbg!(debug::MEMDEV, "clean-page called on zero device? This might be a bug.");
        Err(Errno::ENOTSUP)
    }
    fn show(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "[/dev/zero device]") }
}
