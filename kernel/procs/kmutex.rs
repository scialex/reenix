// TODO Copyright Header

//! KMutex thing

use kqueue::KQueue;
use std::intrinsics::size_of;
use mm::alloc::request_slab_allocator;
use std::fmt::{self, Debug, Formatter};
use std::cell::*;
use std::ptr::*;
use std::sync::atomic::*;
use sync::Wakeup;

pub fn init_stage1() {
    request_slab_allocator("KMutex allocator", unsafe { size_of::<KMutex>() as u32 });
}

pub fn init_stage2() {}

pub struct KMutex {
    name : &'static str,
    held : AtomicBool,
    queue : UnsafeCell<KQueue>,
    //no_copy : core::kinds::marker::NoCopy,
}

impl KMutex {
    /// Create a new mutex with the given name.
    pub fn new(name: &'static str) -> KMutex {
        KMutex { name : name, held : AtomicBool::new(false), queue : UnsafeCell::new(KQueue::new()) }
    }

    /// Obtain the lock, waiting until it is freed. Note that there are no ordering/fairness
    /// gaurentees on who gets a lock when it is contested.
    pub fn lock_nocancel(&self) {
        dbg!(debug::SCHED, "locking {:?} for {:?} of {:?}", self, current_thread!(), current_proc!());
        while self.held.compare_and_swap(false, true, Ordering::SeqCst) != false {
            unsafe { self.queue.get().as_mut().expect("Kmutex queue cannot be null").wait_on(false) };
        }
        return;
    }

    /// Returns true if we got the lock, False if we didn't because of being canceled.
    #[warn(unused_results)]
    pub fn lock(&self) -> bool {
        dbg!(debug::SCHED, "cancelable locking {:?} for {:?} of {:?}", self, current_thread!(), current_proc!());
        while self.held.compare_and_swap(false, true, Ordering::SeqCst) != false {
            if unsafe { !self.queue.get().as_mut().expect("Kmutex queue cannot be null").wait_on(true) } {
                return false;
            }
        }
        return true;
    }

    /// Returns true if we get the lock. False, without sleeping, if we did not.
    pub fn try_lock(&self) -> bool {
        if !self.held.compare_and_swap(false, true, Ordering::SeqCst) {
            dbg!(debug::SCHED, "locking {:?} for {:?} of {:?}", self, current_thread!(), current_proc!());
            true
        } else {
            dbg!(debug::SCHED, "locking {:?} for {:?} of {:?} failed", self, current_thread!(), current_proc!());
            false
        }
    }

    /// Unlocks the lock. This should only be called by the thread that originally locked it.
    pub fn unlock(&self) {
        dbg!(debug::SCHED, "unlocking {:?} for {:?} of {:?}", self, current_thread!(), current_proc!());
        assert!(self.held.load(Ordering::SeqCst));
        self.held.store(false, Ordering::SeqCst);
        unsafe { self.queue.get().as_mut().expect("Kmutex queue cannot be null")}.signal();
    }
}

impl Debug for KMutex {
    fn fmt(&self, f : &mut Formatter) -> fmt::Result {
        write!(f, "KMutex '{}' {{ held: {}, waiters: {} }}", self.name, self.held.load(Ordering::SeqCst),
                unsafe { self.queue.get().as_mut().expect("Kmutex queue cannot be null")}.len())
    }
}
