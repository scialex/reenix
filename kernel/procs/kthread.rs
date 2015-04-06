// TODO Copyright Header

//use base::describe;
use mm::{alloc, page};
use libc::c_void;
use base::errno;
use std::{cmp, hash, fmt, num};
use context;
use std::mem::{size_of, transmute_copy};
use std::ptr::{self, copy_nonoverlapping};
use kqueue::KQueue;
use context::{Context, ContextFunc};
use mm::pagetable::PageDir;
use mm::{AllocError, Allocation};

pub static CUR_THREAD_SLOT : usize = 0;
pub static DEFAULT_STACK_PAGES : usize = 16;

#[allow(raw_pointer_derive)] #[derive(Hash, Eq, PartialEq)]
pub struct KStack(usize, *mut u8);

impl KStack {
    pub fn with_size(pages : usize) -> Allocation<KStack> {
        Ok(KStack(pages, try!(unsafe { page::alloc_n::<u8>(pages) })))
    }

    pub fn new() -> Allocation<KStack> {
        KStack::with_size(DEFAULT_STACK_PAGES)
    }

    pub fn copy(&mut self) -> Result<KStack, AllocError> {
        let &mut KStack(size, _) = self;
        let mut new = try!(KStack::with_size(size));
        new.copy_from(self);
        Ok(new)
    }

    pub fn copy_from(&mut self, other: &KStack) {
        let &mut KStack(msize, mptr) = self;
        let &KStack(osize, optr) = other;
        let size = cmp::min(msize, osize);
        unsafe { copy_nonoverlapping(mptr, optr as *const u8, size); }
    }

    pub fn num_pages(&self) -> usize {
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
        let &mut KStack(size, s) = self;
        if size != 0 {
            unsafe { page::free_n(s as *mut c_void, size as u32); }
        }
        //*self = KStack(0, 0 as *mut u8);
    }
}

#[derive(Debug, Copy)]
pub enum Mode { USER, KERNEL }

#[derive(Debug, Eq, PartialEq, Copy)]
pub enum State { NOSTATE, RUN, SLEEP, SLEEPCANCELLABLE, EXITED }

pub struct KThread {
    pub ctx : Context, // The threads context
    pub kstack : KStack, // The threads stack
    pub retval : *mut c_void, // The threads return value, if we have one.
    pub errno : Option<errno::Errno>, // The current errno, if applicable.
    pub cancelled : bool, // True if we are canceled, false otherwise.
    pub state : State, // Our state.
    pub mode  : Mode, // Whether we are in user or kernel mode
    pub queue : *mut KQueue, // The queue we are currently blocking on.
}

pub fn init_stage1() { alloc::request_slab_allocator("kthread", size_of::<KThread>() as u32) }
pub fn init_stage2() {}

pub fn kyield() {
    let ct = current_thread!();
    ct.ctx.kyield();
}

impl hash::Hash for KThread {
    fn hash<S: hash::Hasher>(&self, state: &mut S) {
        self.kstack.hash(state)
    }
}

impl KThread {
    pub fn new(pdir: &PageDir, main: ContextFunc, arg1 : i32, arg2 : *mut c_void) -> Allocation<KThread> {
        let kstack = try!(KStack::new());
        Ok(KThread {
            ctx       : unsafe { Context::new(main, arg1, arg2, kstack.ptr() as *mut u8,
                                              page::num_to_addr::<u8>(kstack.num_pages()) as usize,
                                              pdir) },
            kstack    : kstack,
            retval    : ptr::null_mut(),
            errno     : None,
            cancelled : false,
            state     : State::NOSTATE,
            mode      : Mode::KERNEL,
            queue     : 0 as *mut KQueue
        })
    }

    /// returns true if this is the current thread, false otherwise.
    pub fn is_current_thread(&self) -> bool { self.kstack == current_thread!().kstack }

    pub fn make_runable(&mut self) {
        assert!(self.queue == ptr::null_mut());
        if self.state == State::RUN {
            return;
        }
        assert!(self.state == State::SLEEP || self.state == State::SLEEPCANCELLABLE || self.state == State::NOSTATE);
        self.state = State::RUN;
        self.ctx.make_runable();
    }

    pub fn exit(&mut self, v: *mut c_void) {
        if self.is_current_thread() {
            self.exit_self(v)
        } else {
            self.cancel(v)
        }
    }

    /// This will mark the given thread as cancelled. This will not affect the thread at all until
    /// this status is checked later.
    pub fn cancel(&mut self, v: *mut c_void) {
        self.cancelled = true;
        if self.state == State::EXITED {
            dbg!(debug::THR, "cancel called on an already exited thread");
            return;
        }
        assert!(self.state != State::NOSTATE, "Illegal state for a process");
        self.retval = v;
        if self.state == State::SLEEPCANCELLABLE {
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
        assert!(self.state == State::RUN);
        dbg!(debug::THR, "Thread {:?} of process {:?} ended with a status of 0x{:x} ({:?})",
             self, current_proc!(), v as usize, num::from_uint::<errno::Errno>(v as usize));
        (current_proc_mut!()).thread_exited(v);
        self.state = State::EXITED;
        context::die();
    }
}

impl fmt::Debug for KThread {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "KThread {{ cancelled: {}, state: {:?}, errno: {:?} }}",
               self.cancelled, self.state, self.errno)
    }
}
