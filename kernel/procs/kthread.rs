// TODO Copyright Header

use core::prelude::*;
//use base::describe;
use mm::alloc;
use alloc::boxed::*;
use mm::page;
use libc::c_void;
use core::any::*;
use base::errno;
use core::ptr;
use core::fmt;
use context;
use core::num;
use core::mem::{size_of, transmute_copy};
use core::ptr::*;
use core::cmp;
use kqueue::KQueue;
use context::{Context, ContextFunc};
use collections::hash;
use mm::pagetable::PageDir;

pub static CUR_THREAD_SLOT : uint = 0;
pub static DEFAULT_STACK_PAGES : uint = 16;

#[allow(raw_pointer_deriving)] #[deriving(Hash, Eq, PartialEq)]
pub struct KStack(uint, *mut u8);

impl KStack {
    pub fn with_size(pages : uint) -> KStack {
        KStack(pages, unsafe { page::alloc_n(pages as u32) as *mut u8})
    }

    pub fn new() -> KStack {
        KStack::with_size(DEFAULT_STACK_PAGES)
    }

    pub fn copy(&mut self) -> KStack {
        let &KStack(size, _) = self;
        let mut new = KStack::with_size(size);
        new.copy_from(self);
        new
    }

    pub fn copy_from(&mut self, other: &KStack) {
        let &KStack(msize, mptr) = self;
        let &KStack(osize, optr) = other;
        let size = cmp::min(msize, osize);
        unsafe { copy_nonoverlapping_memory(mptr, optr as *const u8, size); }
    }

    pub fn num_pages(&self) -> uint {
        let &KStack(size, _) = self;
        size
    }

    pub fn ptr(&self) -> *mut c_void {
        let &KStack(_, p) = self;
        p as *mut c_void
    }
}

impl Drop for KStack {
    fn drop(&mut self) {
        let &KStack(size, s) = self;
        if size != 0 {
            unsafe { page::free_n(s as *mut c_void, size as u32); }
        }
        //*self = KStack(0, 0 as *mut u8);
    }
}

#[deriving(Show)]
pub enum KThreadMode { USER, KERNEL }

#[deriving(Show, Eq, PartialEq)]
pub enum KThreadState { NOSTATE, RUN, SLEEP, SLEEPCANCELLABLE, EXITED }

pub struct KThread {
    pub ctx : Context, // The threads context
    pub kstack : KStack, // The threads stack
    pub retval : *mut c_void, // The threads return value, if we have one.
    pub errno : Option<errno::Errno>, // The current errno, if applicable.
    pub cancelled : bool, // True if we are canceled, false otherwise.
    pub state : KThreadState, // Our state.
    pub mode  : KThreadMode, // Whether we are in user or kernel mode
    pub queue : *mut KQueue, // The queue we are currently blocking on.
}

pub fn init_stage1() { alloc::request_slab_allocator("kthread", size_of::<KThread>() as u32) }
pub fn init_stage2() {}

pub fn kyield() {
    let ct = current_thread!();
    ct.ctx.kyield();
}

impl<S: hash::Writer> hash::Hash<S> for KThread {
    fn hash(&self, state: &mut S) {
        self.kstack.hash(state)
    }
}

impl KThread {
    pub fn new(pdir: &Box<PageDir>, main: ContextFunc, arg1 : i32, arg2 : *mut c_void) -> KThread {
        let kstack = KStack::new();
        KThread {
            ctx       : unsafe { Context::new(main, arg1, arg2, kstack.ptr() as *mut u8,
                                              page::num_to_addr::<u8>(kstack.num_pages()) as uint,
                                              transmute_copy(pdir)) },
            kstack    : kstack,
            retval    : ptr::null_mut(),
            errno     : None,
            cancelled : false,
            state     : NOSTATE,
            mode      : KERNEL,
            queue     : 0 as *mut KQueue
        }
    }

    /// returns true if this is the current thread, false otherwise.
    pub fn is_current_thread(&self) -> bool { self.kstack == current_thread!().kstack }

    pub fn make_runable(&mut self) {
        assert!(self.queue == ptr::null_mut());
        if self.state == RUN {
            return;
        }
        assert!(self.state == SLEEP || self.state == SLEEPCANCELLABLE || self.state == NOSTATE);
        self.state = RUN;
        self.ctx.make_runable();
    }

    pub fn exit(&mut self, v: *mut c_void) {
        if self.is_current_thread() {
            self.exit_self(v)
        } else {
            self.cancel(v)
        }
    }

    fn cancel(&mut self, v: *mut c_void) {
        self.cancelled = true;
        if self.state == EXITED {
            dbg!(debug::THR, "cancel called on an already exited thread");
            return;
        }
        assert!(self.state != NOSTATE, "Illegal state for a process");
        assert!(self.state == SLEEP || self.state == SLEEPCANCELLABLE, "State is illegal for canceling");
        self.retval = v;
        if self.state == SLEEPCANCELLABLE {
            if let Some(queue) = unsafe { self.queue.as_mut() } {
                queue.remove(self);
            }
            self.make_runable();
        }
    }
    fn exit_self(&mut self, v: *mut c_void) -> ! {
        self.retval = v;
        // TODO Add this check back in.
        //assert!(transmute(self) == gdt::get_tsd().cur_thr);
        assert!(self.state == RUN);
        dbg!(debug::THR, "Thread {} of process {} ended with a status of 0x{:x} ({})",
             self, current_proc!(), v as uint, num::from_uint::<errno::Errno>(v as uint));
        (current_proc!()).thread_exited(v);
        self.state = EXITED;
        context::die();
    }
}

impl fmt::Show for KThread {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "KThread {{ cancelled: {}, state: {}, errno: {} }}",
               self.cancelled, self.state, self.errno)
    }
}
