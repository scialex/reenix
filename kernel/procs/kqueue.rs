// TODO Copyright Header

//! KQueue thing

use std::collections::*;
use std::mem::{transmute, transmute_copy};
use std::cell::*;
use std::cmp::Ordering::{self, Equal};
use std::{fmt, ptr};
use kthread::KThread;
use kthread;
use sync;
use base::cell::*;

#[allow(raw_pointer_derive)]
#[derive(Hash)]
pub struct QueuedThread(*mut KThread);
pub struct KQueue(SafeCell<HashSet<QueuedThread>>);

pub fn init_stage1() {}
pub fn init_stage2() {}

impl Ord for QueuedThread {
    fn cmp(&self, other: &QueuedThread) -> Ordering {
        let &QueuedThread(me) = self;
        let &QueuedThread(o) = other;
        (me as usize).cmp(&(o as usize))
    }
}

impl PartialOrd for QueuedThread {
    fn partial_cmp(&self, other: &QueuedThread) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for QueuedThread {
    fn eq(&self, other: &QueuedThread) -> bool {
        self.cmp(other) == Equal
    }
}

impl Eq for QueuedThread {}
impl KQueue {
    pub fn len(&self) -> usize {
        let &KQueue(ref s) = self;
        (*s.get_ref()).len()
    }

    /// Remove a thread from this queue without waking it.
    pub fn remove(&mut self, t: &mut KThread) {
        block_interrupts!({
            assert!((self as *mut KQueue) == t.queue, "Attempting to cancel on incorrect queue.");
            t.queue = ptr::null_mut();
            let k : *mut KThread = t as *mut KThread;
            let &mut KQueue(ref s) = self;
            assert!((*s.get_mut()).remove(&QueuedThread(k)));
        });
    }

    fn add(&mut self, t: &mut KThread) {
        block_interrupts!({
            let &mut KQueue(ref s) = self;
            assert!((*s.get_mut()).insert(QueuedThread(unsafe { transmute(t) })));
        });
    }

    /// Add a thread into this queue. This returns after some call to signal. false if we were
    /// canceled, true otherwise.
    pub fn wait_on(&mut self, cancelable: bool) -> bool {
        let t = current_thread!();
        if cancelable && t.cancelled {
            dbg!(debug::SCHED, "Not waiting for cancelation because thread {:?} is already canceled", t);
            return false;
        }
        block_interrupts!({
            dbg!(debug::SCHED, "{:?} begining wait", t);
            bassert!(t.queue == ptr::null_mut());
            unsafe {
                t.queue = transmute_copy(&self);
            }
            t.state = if cancelable { kthread::State::SLEEPCANCELLABLE } else { kthread::State::SLEEP };
            self.add(t);
            t.ctx.switch();
        });
        return !t.cancelled;
    }

    fn wakeup_one(&self, t: *mut KThread) {
        unsafe {
            let x = t.as_mut().expect("Null thread being waited for!");
            x.queue = ptr::null_mut();
            x.make_runable();
            dbg!(debug::SCHED, "Waking up {:?}", x);
        }
    }

    pub fn new() -> KQueue {
        KQueue ( SafeCell::new(HashSet::new()) )
    }
}

impl sync::Wakeup for KQueue {
    /// Wake up all waiting threads in this queue.
    fn signal(&self) {
        block_interrupts!({
            let &KQueue(ref q) = self;
            dbg!(debug::SCHED, "Waking up {} threads", q.get_ref().len());
            let mut lst = q.get_mut();
            for QueuedThread(x) in lst.drain() {
                self.wakeup_one(x);
            }
        });
    }
}

pub struct WQueue(UnsafeCell<KQueue>);

impl WQueue {
    pub fn new() -> WQueue { WQueue(UnsafeCell::new(KQueue::new())) }
    #[inline]
    fn get_inner<'a>(&'a self) -> &'a mut KQueue { let &WQueue(ref kq) = self; unsafe { transmute(kq.get()) } }
    pub fn len(&self) -> usize { self.get_inner().len() }
    pub fn force_wait(&self) -> Result<(),()> { if self.get_inner().wait_on(false) { Ok(()) } else { Err(()) } }
}

impl sync::Wait<(),()> for WQueue {
    fn wait(&self) -> Result<(),()> {
        if self.get_inner().wait_on(true) { Ok(()) } else { Err(()) }
    }
}

impl sync::Wakeup for WQueue {
    /// Wake up all waiting threads in this queue.
    fn signal(&self) { self.get_inner().signal(); }
}
impl fmt::Debug for WQueue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        block_interrupts!( write!(f, "WQueue {{ waiters: {} }}", self.get_inner().len()) )
    }
}
