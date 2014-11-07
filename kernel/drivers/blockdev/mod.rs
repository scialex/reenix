
//! The reenix block device interface and usage functions.

use alloc::boxed::Box;
use core::prelude::*;
use core::ptr::*;
use mm::page;
use super::{DeviceId, Device};
use collections::*;

pub fn init_stage1() { disk::init_stage1(); }
pub fn init_stage2() {
    init_device_tree();
    disk::init_stage2();
}
pub fn init_stage3() {}

pub type BlockDevice = Device<[u8, ..page::SIZE]>;

static mut DEVICES : *mut TreeMap<DeviceId, Box<BlockDevice>> = 0 as *mut TreeMap<DeviceId, Box<BlockDevice>>;
fn init_device_tree() {
    use core::mem::transmute;
    unsafe {
        assert!(DEVICES.is_null());
        let d = box TreeMap::<DeviceId, Box<BlockDevice>>::new();
        DEVICES = transmute(d);
    }
}

fn get_device_tree() -> &'static mut TreeMap<DeviceId, Box<BlockDevice>> {
    unsafe { DEVICES.as_mut().expect("Device tree is null!") }
}

pub fn lookup_mut(dev: DeviceId) -> Option<&'static mut BlockDevice> { get_device_tree().get_mut(&dev).map(|bd| { &mut **bd }) }
pub fn lookup(dev: DeviceId) -> Option<&'static BlockDevice> { get_device_tree().get_mut(&dev).map(|bd| { &**bd }) }

pub fn register(id: DeviceId, dev: Box<BlockDevice>) -> bool {
    block_interrupts!({
        let m = get_device_tree();
        if m.contains_key(&id) { false } else { m.insert(id, dev).is_none() }
    })
}

mod disk;
