// TODO Copyright Header

//! KMutex thing

use kqueue::KQueue;
use core::intrinsics::size_of;
use mm::alloc::request_slab_allocator;
use core::fmt;
use core::fmt::{Show, Formatter};

pub fn init_stage1() {
    request_slab_allocator("KMutex allocator", unsafe { size_of::<KMutex>() as u32 });
}

pub fn init_stage2() {}

pub struct KMutex {
    name : &'static str,
    held : bool,
    queue : KQueue,
    //no_copy : core::kinds::marker::NoCopy,
}

impl KMutex {
    pub fn new(name: &'static str) -> KMutex {
        KMutex { name : name, held : false, queue : KQueue::new() }
    }

    /// Obtain the lock, waiting until it is freed. Note that there are no ordering/fairness
    /// gaurentees on who gets a lock when it is contested.
    pub fn lock(&mut self) {
        dbg!(debug::SCHED, "locking {} for {} of {}", self, current_thread!(), current_proc!());
        while self.held {
            self.queue.wait(false);
        }
        self.held = true;
        return;
    }

    /// Returns true if we got the lock, False if we didn't because of being canceled.
    pub fn lock_cancelable(&mut self) -> bool {
        dbg!(debug::SCHED, "cancelable locking {} for {} of {}", self, current_thread!(), current_proc!());
        while self.held {
            if !self.queue.wait(true) {
                return false;
            }
        }
        self.held = true;
        return true;
    }

    pub fn try_lock(&mut self) -> bool {
        if !self.held {
            dbg!(debug::SCHED, "locking {} for {} of {}", self, current_thread!(), current_proc!());
            self.held = true;
            true
        } else {
            dbg!(debug::SCHED, "locking {} for {} of {} failed", self, current_thread!(), current_proc!());
            false
        }
    }

    pub fn unlock(&mut self) {
        dbg!(debug::SCHED, "unlocking {} for {} of {} failed", self, current_thread!(), current_proc!());
        assert!(self.held);
        self.held = false;
        self.queue.signal();
    }
}

impl Show for KMutex {
    fn fmt(&self, f : &mut Formatter) -> fmt::Result {
        write!(f, "KMutex {} {{ held: {}, waiters: {} }}", self.name, self.held, self.queue.len())
    }
}
