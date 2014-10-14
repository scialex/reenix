// TODO Copyright Header

//! KQueue thing

use collections::*;
use core::mem::{transmute, transmute_copy};
use core::prelude::*;
use core::cell::*;
use core::ptr;
use core::ptr::*;
use kthread::KThread;
use kthread;

pub struct QueuedThread(*mut KThread);
pub struct KQueue(RefCell<TreeSet<QueuedThread>>);

pub fn init_stage1() {}
pub fn init_stage2() {}

impl Ord for QueuedThread {
    fn cmp(&self, other: &QueuedThread) -> Ordering {
        let &QueuedThread(me) = self;
        let &QueuedThread(o) = other;
        me.to_uint().cmp(&o.to_uint())
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
    pub fn len(&self) -> uint {
        let &KQueue(ref s) = self;
        (*s.borrow()).len()
    }

    /// Remove a thread from this queue without waking it.
    pub fn remove(&mut self, t: &mut KThread) {
        assert!((self as *mut KQueue) == t.queue, "Attempting to cancel on incorrect queue.");
        t.queue = ptr::null_mut();
        let k : *mut KThread = unsafe { transmute(t) };
        let &KQueue(ref s) = self;
        assert!((*s.borrow_mut()).remove(&QueuedThread(k)));
    }

    fn add(&mut self, t: &mut KThread) {
        let &KQueue(ref s) = self;
        assert!((*s.borrow_mut()).insert(QueuedThread(unsafe { transmute(t) })));
    }

    /// Add a thread into this queue. This returns after some call to signal. false if we were
    /// canceled, true otherwise.
    pub fn wait(&mut self, cancelable: bool) -> bool {
        let t = current_thread!();
        if cancelable && t.cancelled {
            dbg!(debug::SCHED, "Not waiting for cancelation because thread {} is already canceled", t);
            return false;
        }
        dbg!(debug::SCHED, "Switching from thread {}", t);
        block_interrupts!({
            unsafe {
                t.queue = transmute_copy(&self);
            }
            t.state = if cancelable { kthread::SLEEPCANCELLABLE } else { kthread::SLEEP };
            self.add(t);
            t.ctx.switch();
        });
        dbg!(debug::SCHED, "returning from thread {}", t);
        return !t.cancelled;
    }

    /// Wake up all waiting threads in this queue.
    pub fn signal(&self) {
        block_interrupts!({
            let &KQueue(ref q) = self;
            dbg!(debug::SCHED, "Waking up {} threads", q.borrow().len());
            for &QueuedThread(x) in q.borrow().iter() {
                self.wakeup_one(x);
            }
            q.borrow_mut().clear();
        });
    }

    fn wakeup_one(&self, t: *mut KThread) {
        unsafe {
            let x = t.as_mut().expect("Null thread being waited for!");
            x.queue = ptr::null_mut();
            x.make_runable();
        }
    }

    pub fn new() -> KQueue {
        KQueue ( RefCell::new(TreeSet::new()) )
    }
}
