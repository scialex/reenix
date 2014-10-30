// TODO Copyright Header

use core::num;
use alloc::rc;
use alloc::rc::{Rc,Weak};
use alloc::boxed::Box;
use base::{describe, errno};
use collections::hash;
use collections::string::String;
use collections::treemap::TreeMap;
use collections::{MutableMap, Map, MutableSet, Mutable};
use context::ContextFunc;
use core::fmt;
use core::ptr::*;
use core::mem::{transmute, transmute_copy};
use core::prelude::*;
use libc::c_void;
use kthread;
use kthread::{KThread, CUR_THREAD_SLOT};
use pcell::*;
use sync::Wakeup;
use kqueue::WQueue;
use sync::Wait;
use mm::pagetable::PageDir;
use mm::AllocError;

pub const CUR_PROC_SLOT : uint = 1;
pub const CUR_PID_SLOT  : uint = 2;

#[deriving(Eq, PartialEq, Show, PartialOrd, Ord, Clone)]
pub struct ProcId(pub uint);

static mut INIT_PROC : *mut Rc<ProcRefCell<KProc>> = 0 as *mut Rc<ProcRefCell<KProc>>;
static INIT_PID : ProcId = ProcId(1);

static mut IDLE_PROC : *mut Rc<ProcRefCell<KProc>> = 0 as *mut Rc<ProcRefCell<KProc>>;
static IDLE_PID : ProcId = ProcId(0);

static mut NEXT_PID : uint = 0;
impl ProcId {
    pub fn new() -> ProcId {
        use core;
        block_interrupts!({
            let nxt = unsafe { NEXT_PID };
            let res = match range(nxt, core::uint::MAX).map(|i| { ProcId(i) })
                                                       .filter(|p| { KProc::get_proc(p).is_none() })
                                                       .nth(0) {
                None => {
                    match range(0, nxt).map(|i| { ProcId(i) })
                                       .filter(|p| { KProc::get_proc(p).is_none() })
                                       .nth(0) {
                        Some(pid) => pid,
                        None => kpanic!("Could not allocate a thread id!"),
                    }
                },
                Some(pid) => pid,
            };
            unsafe { NEXT_PID = (res.0) + 1; }
            res
        })
    }
}

impl Copy for ProcId {}

#[deriving(Show,Eq,PartialEq)]
pub enum ProcState { RUNNING, DEAD }
pub type ProcStatus = int;

pub struct KProc {
    pid      : ProcId,                      /* Our pid */
    command  : String,                      /* Process Name */
    threads  : TreeMap<u64, Box<KThread>>,  /* Our threads */
    children : TreeMap<ProcId, Rc<ProcRefCell<KProc>>>, /* Our children */
    status   : ProcStatus,                  /* Our exit status */
    state    : ProcState,                   /* running/sleeping/etc. */
    parent   : Option<Weak<ProcRefCell<KProc>>>,/* Our parent */
    pagedir  : Box<PageDir>,

    wait : WQueue,

    // TODO For VFS
    // files : [Option<KFile>, ..NFILES],
    // cwd   : RC<VNode>,

    // TODO For VM
    // brk : uint,
    // start_brk : uint,
    // vmmap : Vec<VMArea>,
}

pub fn init_stage1() {
    use mm::alloc::request_slab_allocator;
    use core::mem::size_of;
    request_slab_allocator("ProcRefCell<KProc> allocator", size_of::<ProcRefCell<KProc>>() as u32 + 16);
}

pub fn init_stage2() {
    use core::intrinsics::transmute;
    unsafe {
        let y : Box<TreeMap<ProcId, Rc<ProcRefCell<KProc>>>> = box TreeMap::new();
        PROC_LIST = transmute(y);
    }
}

static mut PROC_LIST : *mut TreeMap<ProcId, Weak<ProcRefCell<KProc>>> = 0 as *mut TreeMap<ProcId, Weak<ProcRefCell<KProc>>>;
macro_rules! proc_list(
    () => ({
        unsafe { PROC_LIST.as_mut().expect("proc_list not yet initialized") }
    })
)

static mut IDLE_STARTED : bool = false;

/// Function that is called once to start the idle process from a non-thread context.
pub fn start_idle_proc(init_main : ContextFunc, arg1: i32, arg2: *mut c_void) -> ! {
    use context;

    assert!(unsafe { IDLE_STARTED } == false, "IDLE THREAD ALREADY STARTED");
    unsafe { IDLE_STARTED = true; }

    let pid = KProc::new(String::from_str("IDLE PROCESS"), init_main, arg1, arg2).ok().expect("Unable to allocate idle proc!");

    assert!(pid == IDLE_PID);
    dbg!(debug::CORE, "Starting idle process {} now!", pid);

    context::initial_ctx_switch();
}

pub enum WaitProcId { Any, Pid(ProcId) }
pub type WaitOps = u32;

impl KProc {
    pub fn get_pagedir<'a>(&'a self) -> &'a PageDir {
        &*self.pagedir
    }
    /// Perform the waitpid syscall. This simply passes the call along to the current process. It
    /// returns Ok((killed_PID,status)) on success and Err(errno) on failure.
    pub fn waitpid(pid: WaitProcId, options : WaitOps) -> Result<(ProcId, ProcStatus),errno::Errno> {
        (current_proc_mut!()).do_waitpid(pid, options)
    }

    /// Checks if this process is the one we are currently running in.
    pub fn is_current_process(&self) -> bool {
        (current_pid!()) == self.pid
    }

    /// Wait on a process.
    ///
    /// If pid is Any we should wait on one of our our exited children. If there are no exited
    /// children we should wait until there is one by waiting on our own wait queue.
    ///
    /// If pid is Pid(x) we should wait on the process identified by the given PID.
    ///
    /// If we have no children or the given pid is not one of our children, we should exit with
    /// Err(ECHILD).
    ///
    /// Options other than 0 are unsupported.
    ///
    /// This operation cannot be canceled.
    fn do_waitpid(&mut self, pid: WaitProcId, options : WaitOps) -> Result<(ProcId, ProcStatus), errno::Errno> {
        if options != 0 {
            dbg!(debug::PROC, "waitpid with options 0b{:t} is not supported.", options);
            return Err(errno::ECHILD);
        }

        // This should only be called while running in our own context.
        assert!(self.is_current_process());
        // Get the process we will actually wait on.
        let to_kill = try!(match pid {
                                Any => self.wait_any_process(options),
                                Pid(p) => self.wait_specific_process(p, options),
                            });
        let final_pid = (*to_kill).borrow().pid.clone();
        if let Pid(p) = pid { assert!(final_pid == p); }
        let result = (*to_kill).borrow().status;
        // Remove it from our child map.
        self.children.remove(&final_pid);

        // Remove all child threads.
        (*to_kill).borrow_mut().threads.clear();

        // Remove it from the global map.
        KProc::remove_proc(&final_pid);

        // If we are waiting on the init_proc we need to make sure the collect the global init_proc
        // pointer we saved during startup.
        if final_pid == INIT_PID {
            unsafe {
                let x : Box<Rc<ProcRefCell<KProc>>> = transmute(INIT_PROC);
                dbg!(debug::CORE, "Dropped init_proc!");
                drop(x);
                INIT_PROC = 0 as *mut Rc<ProcRefCell<KProc>>;
            }
        }

        // This should always be true (except for the init-proc) since all this processes' children
        // should have been reparented to init in KProc::cleanup. We will just ignore the init-proc
        // and allow it to leak some memory, since we will only get here for it if we are shutting
        // down.
        assert!(rc::is_unique(&to_kill), "{} is not unique", (*to_kill).borrow());

        // Actually destroy the process.
        drop(to_kill);

        dbg!(debug::PROC, "{} Successfully waited on process {} which exited with {} (0x{:X})", self, final_pid, result, result);
        return Ok((final_pid, result));
    }

    /// Wait for any child process to die
    fn wait_any_process(&mut self, options: WaitOps) -> Result<Rc<ProcRefCell<KProc>>, errno::Errno> {
        assert!(options == 0);
        loop {
            if self.children.is_empty() {
                dbger!(debug::PROC, errno::ECHILD, "Process {} attempted to wait on any child when no children were availible.",
                       self);
                return Err(errno::ECHILD);
            }
            if let Some(ref kproc) =
                    self.children.values().find(|a: &&Rc<ProcRefCell<KProc>>| -> bool {
                                                    (**a).borrow().state == DEAD
                                                }) {
                return Ok((*kproc).clone());
            }
            if self.wait.wait().is_err() {
                dbg!(debug::PROC, "Process {} interrupted while waiting for any children to exit", self);//describe!(self));
                return Err(errno::ECANCELED);
            }
        }
    }
    /// Wait for a specific PID to exit.
    fn wait_specific_process(&mut self, pid: ProcId, options: WaitOps) -> Result<Rc<ProcRefCell<KProc>>, errno::Errno> {
        assert!(options == 0);
        match self.children.find(&pid) {
            Some(v) => {
                let pr = v.clone();
                loop {
                    let b = (*pr).borrow();
                    if b.state == DEAD {
                        break;
                    } else {
                        // We need to make sure the borrow isn't held during the sleep. Something
                        // else might want to look at it.
                        drop(b);
                        if self.wait.wait().is_err() {
                            dbg!(debug::PROC, "Process {} interrupted while waiting for child {} to exit",self, pid); //describe!(self), pid);
                            return Err(errno::ECANCELED);
                        }
                    }
                }
                Ok(pr)
            },
            None => {
                dbger!(debug::PROC, errno::ECHILD, "Attempt by {} to wait on pid {} failed because it is not a child.", self, pid);
                Err(errno::ECHILD)
            },
        }
    }

    /// Returns true if all threads (other then the current one) are EXITED.
    fn all_threads_dead(&self) -> bool {
        for a in self.threads.values() {
            if !a.is_current_thread() && a.state != kthread::EXITED {
                return false;
            }
        }
        return true;
    }

    pub fn kill_all() -> ! {
        for p in (proc_list!()).values().map(|v| -> Option<Rc<ProcRefCell<KProc>>> { v.clone().upgrade() }) {
            match p {
                Some(pr) => {
                    let mut canidate = pr.deref().borrow_mut();
                    if !canidate.is_current_process() && canidate.pid != IDLE_PID && canidate.pid != INIT_PID {
                        canidate.kill(errno::ECANCELED as ProcStatus);
                    }
                }
                _ => (),
            }
        }
        (current_proc_mut!()).kill(errno::ECANCELED as ProcStatus);
        kpanic!("Should not return from killing yourself");
    }

    pub fn get_proc(pid: &ProcId) -> Option<Rc<ProcRefCell<KProc>>> {
        let r = proc_list!().find(pid);
        match r {
            Some(ref p) => p.clone().upgrade().or_else(|| { KProc::remove_proc(pid); None }),
            None => None,
        }
    }

    fn add_proc(pid: ProcId, p : Weak<ProcRefCell<KProc>>) {
        block_interrupts!({
            let lst = proc_list!();
            lst.insert(pid, p)
        });
    }

    fn remove_proc(pid: &ProcId) {
        block_interrupts!({
            let lst = proc_list!();
            lst.remove(pid);
        })
    }

    /// The base creation function for a process. This should not generally be used.
    pub fn create(name: String) -> Result<KProc, AllocError> {
        Ok(KProc {
            pid : ProcId::new(),
            command : name,
            // TODO Maybe I should just have this be a box for now.
            threads : try!(alloc!(try TreeMap::new())),
            children : try!(alloc!(try TreeMap::new())),
            status : 0,
            state : RUNNING,
            parent : None,
            pagedir : try!(alloc!(try_box PageDir::new())),
            wait : try!(alloc!(try WQueue::new())),
        })
    }

    pub fn new(name: String, init_main : ContextFunc, arg1: i32, arg2: *mut c_void) -> Result<ProcId, AllocError> {
        let is_idle = unsafe { IDLE_PROC == null_mut() };
        let is_init = unsafe { !is_idle && INIT_PROC == null_mut() };

        let rcp = match alloc!(try Rc::new(ProcRefCell::new(try!(KProc::create(name))))) {
            Ok(e) => e,
            Err(e) => { dbg!(debug::PROC, "Unable to allocate a Process."); return Err(e); }
        };

        let mut init_thread = match alloc!(try_box try!(KThread::new(&(*rcp).borrow_mut().deref().pagedir, init_main, arg1, arg2))) {
            Ok(t) => t,
            Err(s) => { dbg!(debug::PROC|debug::THR, "Unable to allocate kthread."); return Err(s); }
        };


        let hash = hash::hash(&*init_thread);
        let pid = (*rcp).borrow_mut().pid.clone();
        // TODO This should really actually use a Rc or something.
        try!(alloc!(try {
            let thr_ptr = unsafe { transmute_copy::<Box<KThread>,*mut KThread>(&init_thread) };
            init_thread.ctx.tsd.set_slot(CUR_THREAD_SLOT, box thr_ptr);
            init_thread.ctx.tsd.set_slot(CUR_PROC_SLOT, box rcp.clone().downgrade());
            init_thread.ctx.tsd.set_slot(CUR_PID_SLOT, box pid.clone());
        }));

        {
            let mut p = (*rcp).borrow_mut();
            if !is_idle {
                p.parent = Some(KProc::get_proc(&current_proc!().pid).clone().expect("Only the idle thread should have no parent").downgrade());
            } else {
                dbg!(debug::CORE, "IDLE PROCESS BEING CREATED");
                assert!(pid == ProcId(0));
            }
            try!(alloc!(try p.threads.insert(hash, init_thread)));
        }

        // TODO These few things should also be wraped in try-catch
        KProc::add_proc(pid.clone(), rcp.clone().downgrade());
        if !is_idle {
            (current_proc_mut!()).children.insert(pid.clone(), rcp.clone());
        }

        // We need to set up IDLE and INIT process globals. These are just here.
        if is_idle {
            dbg!(debug::CORE | debug::PROC, "Setting IDLE PROC");
            let tmp = box rcp.clone();
            unsafe { IDLE_PROC = transmute(tmp); }
        } else if is_init {
            dbg!(debug::CORE | debug::PROC, "Setting INIT PROC");
            let tmp = box rcp.clone();
            unsafe { INIT_PROC = transmute(tmp); }
        }
        rcp.borrow_mut().threads.find_mut(&hash).expect("thread must still be present").make_runable();
        return Ok(pid);
    }

    pub fn get_pid(&self) -> &ProcId {
        &self.pid
    }

    /// This has nothing to do with signals and kill(1).
    ///
    /// This is called to have a process cancel all of its threads.
    pub fn kill(&mut self, status: ProcStatus) {
        dbg!(debug::PROC, "proc::kill(status = {} {}) called on {}. Called by {}",
             status, num::from_int::<errno::Errno>(status), self, current_proc!());
        for (_, thr) in self.threads.iter_mut() {
            if !thr.is_current_thread() { thr.exit(status as *mut c_void); }
            if cfg!(MTP) {
                not_yet_implemented!("MTP: proc::kill");
            }
        }
        if self.is_current_process() {
            (current_thread!()).exit(status as *mut c_void);
        }
    }

    /// This is a callback by a thread when it exits. We need to record that it has exited and
    /// decide if we need to quit. If it is the last thread we clean up what we can then return.
    pub fn thread_exited(&mut self, exit: *mut c_void) {
        assert!(self.threads.contains_key(&hash::hash(current_thread!())));
        if self.all_threads_dead() {
            self.cleanup(exit as ProcStatus);
        } else {
            not_yet_implemented!("MTP: thread_exited for multithreaded programming");
        }
    }

    /// This cleans up any parts of the process we can before being wait'd on.
    fn cleanup(&mut self, status: ProcStatus) {
        assert!(self.is_current_process());
        assert!(self.pid != IDLE_PID);
        let parent = self.parent.clone().expect("PARENT PROCESS UNSET").upgrade().expect("Parent process should not have been destroyed!");
        dbg!(debug::PROC, "{} cleaning up. Sending wakeup to parent {}, exit status was 0x{:x}", self, parent.borrow(), status);
        self.status = status;
        self.state = DEAD;
        let init = init_proc!();
        for (pid, child) in self.children.iter() {
            dbg!(debug::PROC, "moving {} to init proc", pid);
            (**child).borrow_mut().parent = Some(init.clone().downgrade());
            init.borrow_mut().children.insert(pid.clone(), child.clone());
        }
        // get rid of our ref's to the children.
        self.children.clear();

        // TODO VFS CLOSE ALL FILES
        // TODO VFS CLOSE CWD
        // TODO VM  DELETE VMMAP

        parent.borrow().wait.signal();
    }
}

impl fmt::Show for KProc {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "KProc {} ('{}' {:p})", self.pid, self.command, self)
    }
}

impl describe::Describeable for KProc {
    fn describe(&self, f: &mut fmt::Formatter) -> fmt::Result {
        try!(write!(f, "{{ {} children are: [", self));
        let mut started = false;
        for x in self.children.keys() {
            if started {
                try!(write!(f, ", "))
            }
            try!(write!(f, "{}", x));
            started = true;
        }
        try!(write!(f, "] parent is: "));
        if self.parent.is_none() {
            write!(f, "{} }}", "<NOTHING>")
        } else {
            write!(f, "{} }}", self.parent.clone().expect("PARENT IS NULL").upgrade().expect("Parent being used!").deref().borrow())
        }
    }
}

impl PartialEq for KProc {
    fn eq(&self, other: &KProc) -> bool {
        self.pid == other.pid
    }
}
