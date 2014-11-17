
//! Additional atomic stuff for easier atomicity

use core::atomic::*;

pub struct EasyAtomicUint(AtomicUint);
impl EasyAtomicUint {
    pub fn new(v: uint) -> EasyAtomicUint { EasyAtomicUint(AtomicUint::create(v)) }
    pub fn load(&self) -> uint { self.0.load(SeqCst) }
    pub fn swap(&self, v: uint) -> uint { self.0.swap(v, SeqCst) }
    pub fn compare_and_swap(&self, new: uint) -> uint { self.0.swap(v, SeqCst) }
    pub fn inner<'a>(&'a self) -> &'a AtomicUint { self.0 }
}

