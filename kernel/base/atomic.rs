
//! Additional atomic stuff for easier atomicity

use core::atomic::*;

pub struct EasyAtomicUint(AtomicUint);
impl EasyAtomicUint {
    pub fn new(v: usize) -> EasyAtomicUint { EasyAtomicUint(AtomicUint::create(v)) }
    pub fn load(&self) -> usize { self.0.load(SeqCst) }
    pub fn swap(&self, v: usize) -> usize { self.0.swap(v, SeqCst) }
    pub fn compare_and_swap(&self, new: usize) -> usize { self.0.swap(v, SeqCst) }
    pub fn inner<'a>(&'a self) -> &'a AtomicUint { self.0 }
}

