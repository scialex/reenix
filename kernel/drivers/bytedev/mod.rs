
//! The reenix byte device interface and usage functions.

use alloc::boxed::Box;
use core::prelude::*;
use super::{DeviceId, Device, WDevice};
use collections::*;
use core::fmt;

mod tty;

/// Do initialization that does not require allocating memory.
pub fn init_stage1() {
    tty::init_stage1();
}

/// Do initialization that requires allocating memory.
pub fn init_stage2() {
    init_device_tree();
    tty::init_stage2();
}

/// Do initialization that requires running in a process context.
pub fn init_stage3() {
    tty::init_stage3();
}

/// Handle computer shutdown.
pub fn shutdown() {
    tty::shutdown();
}
static mut DEVICES : *mut BTreeMap<DeviceId, Box<Device<u8> + 'static>> = 0 as *mut BTreeMap<DeviceId, Box<Device<u8> + 'static>>;
fn init_device_tree() {
    use core::mem::transmute;
    unsafe {
        assert!(DEVICES.is_null());
        let d = box BTreeMap::<DeviceId, Box<ByteDevice>>::new();
        DEVICES = transmute(d);
    }
}

/// Get the tree holding static references to all devices.
fn get_device_tree() -> &'static mut BTreeMap<DeviceId, Box<ByteDevice>> {
    unsafe { DEVICES.as_mut().expect("Device tree is null!") }
}

/// A device capable of reading and writing at byte granularity.
pub type ByteDevice = Device<u8>;

pub fn lookup(dev: DeviceId) -> Option<&'static (Device<u8> + 'static)> {
    get_device_tree().get(&dev).map(|bd| { &**bd })
}

pub fn register(id: DeviceId, dev: Box<Device<u8> + 'static>) -> bool {
    let m = get_device_tree();
    if m.contains_key(&id) { false } else { m.insert(id, dev).is_none() }
}

pub struct ByteWriter<'a>(pub &'a mut Device<u8>);

impl<'a> fmt::Writer for ByteWriter<'a> {
    fn write_str<'b>(&'b mut self, s: &str) -> fmt::Result {
        let &mut ByteWriter(ref mut this) = self;
        match this.write_to(0, s.as_bytes()) {
            Ok(_) => Ok(()),
            Err(_) => Err(fmt::Error),
        }
    }
}
