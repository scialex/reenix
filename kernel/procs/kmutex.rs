// TODO Copyright Header

//! KMutex thing

use kqueue::KQueue;
use std::intrinsics::{size_of, transmute};
use mm::alloc::request_slab_allocator;
use std::fmt::{self, Debug, Formatter};
use std::cell::*;
use std::ptr::*;
use std::sync::atomic::*;
use sync::Wakeup;
use kthread::KThread;

pub fn init_stage1() {
    request_slab_allocator("KMutex allocator", unsafe { size_of::<KMutex>() as u32 });
}

pub fn init_stage2() {}

/// A basic re-entrant mutex
pub struct KMutex {
    name : &'static str,
    held : AtomicUsize,
    holder: AtomicPtr<KThread>,
    queue : UnsafeCell<KQueue>,
    //no_copy : core::kinds::marker::NoCopy,
}

impl KMutex {
    /// Create a new mutex with the given name.
    pub fn new(name: &'static str) -> KMutex {
        KMutex { name : name, held : AtomicUsize::new(0), holder: AtomicPtr::new(null_mut()), queue : UnsafeCell::new(KQueue::new()) }
    }

    /// Obtain the lock, waiting until it is freed. Note that there are no ordering/fairness
    /// gaurentees on who gets a lock when it is contested.
    pub fn lock_nocancel(&self) {
        dbg!(debug::SCHED, "locking {:?} for {:?} of {:?}", self, current_thread!(), current_proc!());
        let null = null_mut();
        let thr = current_thread!() as *mut KThread;
        if self.holder.load(Ordering::SeqCst) != thr {
            while self.holder.compare_and_swap(null, thr, Ordering::SeqCst) != null {
                unsafe { self.queue.get().as_mut().expect("Kmutex queue cannot be null").wait_on(false) };
            }
            assert!(self.holder.load(Ordering::SeqCst) == thr, "We should have gotten mutex but didn't");
            assert!(self.held.load(Ordering::SeqCst) == 0, "Multiple threads with same thread pointer!");
        } else {
            assert!(self.held.load(Ordering::SeqCst) != 0, "Multiple threads with same thread pointer!");
        }
        self.held.fetch_add(1, Ordering::SeqCst);
        dbg!(debug::SCHED, "locked {:?} for {:?} of {:?}", self, current_thread!(), current_proc!());
        return;
    }

    /// Returns true if we got the lock, False if we didn't because of being canceled.
    pub fn lock(&self) -> bool {
        dbg!(debug::SCHED, "cancelable locking {:?} for {:?} of {:?}", self, current_thread!(), current_proc!());
        let null = null_mut();
        let thr = current_thread!() as *mut KThread;
        if self.holder.load(Ordering::SeqCst) != thr {
            while self.holder.compare_and_swap(null, thr, Ordering::SeqCst) != null {
                if unsafe { !self.queue.get().as_mut().expect("Kmutex queue cannot be null").wait_on(true) } {
                    return false;
                }
            }
            assert!(self.holder.load(Ordering::SeqCst) == thr, "We should have gotten mutex but didn't");
            assert!(self.held.load(Ordering::SeqCst) == 0, "Multiple threads with same thread pointer!");
        } else {
            assert!(self.held.load(Ordering::SeqCst) != 0, "Multiple threads with same thread pointer!");
        }
        self.held.fetch_add(1, Ordering::SeqCst);
        dbg!(debug::SCHED, "locked {:?} for {:?} of {:?}", self, current_thread!(), current_proc!());
        return true;
    }

    /// Returns true if we get the lock. False, without sleeping, if we did not.
    pub fn try_lock(&self) -> bool {
        if self.holder.compare_and_swap(null_mut(), current_thread!() as *mut KThread, Ordering::SeqCst) == null_mut() {
            self.held.fetch_add(1, Ordering::SeqCst);
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
        let thr = current_thread!() as *mut KThread;
        match self.held.fetch_sub(1, Ordering::SeqCst) {
            0 => { panic!("Unlocked a mutex thats not locked!"); },
            1 => {
                if self.holder.compare_and_swap(thr, null_mut(), Ordering::SeqCst) != thr {
                    panic!("Unlocked a mutex locked by another thread!");
                }
                unsafe { self.queue.get().as_mut().expect("Kmutex queue cannot be null")}.signal();
            },
            _ => { assert!(self.holder.load(Ordering::SeqCst) == thr, "Unlocked a mutex held by another thread."); }
        }
    }

    /// Used for sleeping on a mutex, Unlocks the mutex as many times as it has been locked and
    /// returns the number of times it has been locked.
    pub fn unlock_all(&self) -> usize {
        let held = self.held.swap(1, Ordering::SeqCst);
        self.unlock();
        return held;
    }

    pub fn relock_all(&self, t: usize) -> bool {
        self.lock_nocancel();
        if self.held.compare_and_swap(1, t, Ordering::SeqCst) != 1 {
            panic!("{:?} Used by this thread prior after calling unlock_all", self);
        }
        return true;
    }
}

impl Debug for KMutex {
    fn fmt(&self, f : &mut Formatter) -> fmt::Result {
        write!(f, "KMutex '{}' {{ holder: {:?}, held: {:?}, waiters: {} }}", self.name,
                unsafe { transmute::<*const KThread, &KThread>(self.holder.load(Ordering::SeqCst)) },
                self.held.load(Ordering::SeqCst),
                unsafe { self.queue.get().as_mut().expect("Kmutex queue cannot be null")}.len())
    }
}
