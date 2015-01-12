
//! The reenix block device interface and usage functions.

use alloc::boxed::Box;
use alloc::rc::*;
use core::prelude::*;
use mm::page;
use super::{DeviceId, Device};
use collections::*;
use umem::mmobj::*;

pub fn init_stage1() { disk::init_stage1(); }
pub fn init_stage2() {
    init_device_tree();
    disk::init_stage2();
}
pub fn init_stage3() {}

pub trait BlockDevice : Device<[u8; page::SIZE]> + MMObj {}

/// What we give out to those who want block devices.
pub type ExternBlockDevice = Rc<Box<BlockDevice>>;

static mut DEVICES : *mut BTreeMap<DeviceId, ExternBlockDevice> = 0 as *mut BTreeMap<DeviceId, ExternBlockDevice>;
fn init_device_tree() {
    use core::mem::transmute;
    unsafe {
        assert!(DEVICES.is_null());
        let d = box BTreeMap::<DeviceId, ExternBlockDevice>::new();
        DEVICES = transmute(d);
    }
}

fn get_device_tree() -> &'static mut BTreeMap<DeviceId, ExternBlockDevice> {
    unsafe { DEVICES.as_mut().expect("Device tree is null!") }
}

pub fn lookup(dev: DeviceId) -> Option<ExternBlockDevice> { get_device_tree().get_mut(&dev).map(|bd| bd.clone()) }

pub fn register(id: DeviceId, dev: Box<BlockDevice>) -> bool {
    block_interrupts!({
        let m = get_device_tree();
        if m.contains_key(&id) { false } else { m.insert(id, Rc::new(dev)).is_none() }
    })
}

mod disk;
