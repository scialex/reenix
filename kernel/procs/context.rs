
//! The implementation of a process context

use mm::pagetable;
use mm::page;
use startup::{gdt, tsd};
use libc::{c_void, uintptr_t};
use interrupt;
use std::mem::{transmute, transmute_copy};
use std::collections::RingBuf;
use std::ptr::null_mut;
use std::rc::*;
use pcell::*;


pub type ContextFunc = extern "C" fn (i: i32, v: *mut c_void) -> *mut c_void;

#[repr(C)]
struct CContext {
    eip : uintptr_t,
    esp : uintptr_t,
    ebp : uintptr_t,
}

pub struct Context {
    ccontext : CContext,

    pd  : *mut pagetable::PageDir, /* Pointer to this processes page directory */
    pub tsd : Box<tsd::TSDInfo>,

    kstack : usize,
    kstack_size : usize,
}

static mut BOOTSTRAP_FUNC_CTX : *mut Context = 0 as *mut Context;
pub fn enter_bootstrap_func(f: ContextFunc, i: i32, v: *mut c_void) -> ! {
    dbg!(debug::CORE, "Entering bootstrap");
    unsafe {
        let bstack = page::alloc_n::<u8>(4).unwrap_or_else(|_| {kpanic!("Unable to allocate stack for bootstrap function") });
        let ctx = box Context::new(f, i, v, bstack, 4 * page::SIZE, transmute(pagetable::current));
        BOOTSTRAP_FUNC_CTX = transmute_copy(&ctx);
        ctx.make_active();
    }
}

pub fn cleanup_bootstrap_function() {
    unsafe {
        assert!(BOOTSTRAP_FUNC_CTX != null_mut());
        let x : Box<Context> = transmute(BOOTSTRAP_FUNC_CTX);
        BOOTSTRAP_FUNC_CTX = null_mut();
        drop(x);
    }
}

struct SleepingThread(*mut Context);
struct RunQueue(RingBuf<SleepingThread>);

impl RunQueue {
    fn push(&mut self, ctx: &mut Context) {
        assert!(interrupt::get_ipl() == interrupt::HIGH);
        let &mut RunQueue(ref mut b) = self;
        b.push_back(SleepingThread(unsafe { transmute(ctx) }));
        dbg!(debug::SCHED, "there are now {} threads waiting to be executed", b.len());
    }

    /// Needed to make sure the whole check isnt optimized away.
    ///
    /// NOTE This is the closest way I can say that a value is volatile...
    unsafe fn get_inner(&mut self) -> &mut RingBuf<SleepingThread> {
        use std::intrinsics::volatile_load;
        let &mut RunQueue(ref mut b) = self;
        volatile_load::<&mut RingBuf<SleepingThread>>(&b as *const &mut RingBuf<SleepingThread>)
    }

    fn pop(&mut self) -> &mut Context {
        loop {
            assert!(interrupt::get_ipl() == interrupt::HIGH);
            if let Some(next) = unsafe { self.get_inner().pop_front() } {
                // TODO Put this dbg back in.
                //dbg!(debug::SCHED, "found context for thead {} in {}", next.get_current_thread(), next.get_current_proc());
                dbg!(debug::SCHED, "found a thread and executing it");
                assert!(interrupt::get_ipl() == interrupt::HIGH);
                let SleepingThread(c) = next;
                return unsafe { c.as_mut().expect("Null thread in queue?") };
            }
            interrupt::disable();
            interrupt::set_ipl(interrupt::LOW);
            dbg!(debug::SCHED, "No threads waiting to be executed!");
            interrupt::wait();
            interrupt::set_ipl(interrupt::HIGH);
        }
        /* NB The below does not seem to work. I am unsure why. I think it is trying to be smart
         * and realizes that since we don't add anything to b it must continue to be false so it
         * just infinite loops maybe?
         */
        /*
        while b.is_empty() {
            interrupt::disable();
            interrupt::set_ipl(interrupt::LOW);
            dbg!(debug::SCHED, "No threads waiting to be executed!");
            interrupt::wait();
            interrupt::set_ipl(interrupt::HIGH);
        }
        if let Some(next) = b.pop_front() {
            // TODO Put this dbg back in.
            //dbg!(debug::SCHED, "found context for thead {} in {}", next.get_current_thread(), next.get_current_proc());
            let SleepingThread(c) = next;
            unsafe { c.as_mut().expect("Null thread in queue?") }
        } else {
            kpanic!("No context found for next thread despite is_empty returning false!");
        }
        */
    }
}

static mut runq : *mut RunQueue = 0 as *mut RunQueue;

pub fn init_stage1() {}
pub fn init_stage2() {
    let x = box RunQueue(RingBuf::new());
    unsafe {
        runq = transmute(x);
    }
}
pub fn init_stage3() {
    unsafe { REACHED_IDLE_FUNC = true; }
}

static mut REACHED_IDLE_FUNC : bool = false;
static mut INITIAL_SWITCH : bool = false;
pub fn initial_ctx_switch() -> ! {
    assert!(unsafe { INITIAL_SWITCH } == false);
    interrupt::set_ipl(interrupt::HIGH);
    unsafe { INITIAL_SWITCH = true; }
    let nxt = pop_runable_ctx();
    unsafe {
        nxt.make_active();
    }
}

pub fn die() -> ! {
    interrupt::set_ipl(interrupt::HIGH);
    let nxt = pop_runable_ctx();
    assert!(interrupt::HIGH == interrupt::get_ipl());
    let thr = current_thread!();
    unsafe { thr.ctx.switch_to(nxt) };
    kpanic!("Returned to killed thread!");
}

fn push_runable_ctx(ctx : &mut Context) {
    unsafe {
        let rq : &mut RunQueue = runq.as_mut().expect("attempt to push context before initialization finished");
        rq.push(ctx);
    }
}

// Not really static but I need to make sure it isn't collected.
fn pop_runable_ctx() -> &'static mut Context {
    unsafe {
        let rq : &mut RunQueue = runq.as_mut().expect("Attempted to pop context before initialization finished");
        rq.pop()
    }
}

extern "C" fn failure_func() {
    kpanic!("Should never reach here. context.eip is not getting properly overriden.");
}

extern "C" fn _rust_context_initial_function(f : ContextFunc, i: i32, v: *mut c_void) -> ! {
    // TODO Might still want this. We need it off for the idle-proc though :-/
    if unsafe { REACHED_IDLE_FUNC } {
        interrupt::set_ipl(interrupt::LOW);
        interrupt::enable();
    }

    let result = f(i, v);
    let thr = current_thread!();
    thr.exit(result);

    kpanic!("Should never return from kthread.exit()");
}

impl Context {
    pub fn make_runable(&mut self) {
        block_interrupts!({ push_runable_ctx(self) });
    }

    /// Places the thread in a runqueue and then switches,
    pub fn kyield(&mut self) {
        let curipl = interrupt::get_ipl();
        interrupt::set_ipl(interrupt::HIGH);
        self.make_runable();
        self.switch();
        interrupt::set_ipl(curipl);
    }

    /// Switches to another context without puting this one on the run queue
    pub fn switch(&mut self) {
        let curipl = interrupt::get_ipl();
        interrupt::set_ipl(interrupt::HIGH);
        let nxt = pop_runable_ctx();
        assert!(interrupt::HIGH == interrupt::get_ipl());
        unsafe { self.switch_to(nxt); }
        assert!(interrupt::HIGH == interrupt::get_ipl());
        interrupt::set_ipl(curipl);
    }

    /// Switches away dieing. This function never returns. It does not update the context of the
    /// calling thread.
    pub unsafe fn new(f : ContextFunc, arg1 : i32, arg2 : *mut c_void,
                      kstack : *mut u8, stack_size : usize,
                      pd: *mut pagetable::PageDir) -> Context {
        assert!(pd != null_mut());
        assert!(page::aligned(kstack as *const u8));

        let shigh= kstack.offset(stack_size as isize);
        let esp : usize;
        /* put the arguments for __contect_initial_func onto the
         * stack, leave room at the bottom of the stack for a phony
         * return address (we should never return from the lowest
         * function on the stack */
        asm!("
            pushl %ebp
            movl %esp, %ebp
            movl  $1, %esp
            pushl $2
            pushl $3
            pushl $4
            pushl $$0
            movl %esp, $0
            movl %ebp, %esp
            popl %ebp
            "
            : "=r"(esp) : "r"(shigh), "r"(arg2), "r"(arg1), "r"(f) : : "volatile");

        let temp_tsd = tsd::TSDInfo::new(kstack as u32);
        Context {
            ccontext    : CContext {
                            eip : transmute(_rust_context_initial_function),
                            ebp : esp as uintptr_t,
                            esp : esp as uintptr_t,
                          },
            kstack      : kstack as usize,
            kstack_size : stack_size,
            pd          : pd,
            tsd         : box temp_tsd,
        }
    }

    unsafe fn make_active(&self) -> ! {
        gdt::set_kernel_stack((self.kstack + self.kstack_size) as *mut c_void);
        self.pd.as_mut().expect("pagedir is missing").set_active();
        gdt::set_tsd(transmute_copy(&self.tsd));
        asm!("
            movl $0, %ebp
            movl $1, %esp
            push $2
            ret
            " : : "r"(self.ccontext.ebp), "r"(self.ccontext.esp),"r"(self.ccontext.eip) : : "volatile");

        kpanic!("control reached after context switch");
    }

    unsafe fn switch_to(&mut self, newc : &Context) {
        use kproc::{CUR_PROC_SLOT, CUR_PID_SLOT, KProc, ProcId};
        use std::ops::Deref;

        let ipl = interrupt::get_ipl();
        interrupt::set_ipl(interrupt::HIGH);
        gdt::set_kernel_stack((newc.kstack + newc.kstack_size) as *mut c_void);
        newc.pd.as_mut().expect("pagedir is missing").set_active();
        if let Some(v) = newc.tsd.get_slot(CUR_PID_SLOT) {
            dbg!(debug::SCHED, "Switching to {:?}",v.downcast_ref::<ProcId>().expect(add_file!("CUR_PID_SLOT not used")))
        }

        gdt::set_tsd(transmute_copy(&newc.tsd));

        // NOTE LLVM Really doesn't seem to like the inline ASM for some reason. If it even works
        // it gets incorrect asm. This is a function compiled by GDB.
        extern "C" { fn do_real_context_switch(cur : *mut CContext, new : *const CContext); }
        self.ccontext.eip = transmute(failure_func);
        do_real_context_switch(&mut self.ccontext, &newc.ccontext);

        // Take back ownership of the current process
        (**gdt::get_tsd().get_slot(CUR_PROC_SLOT).expect(add_file!("CUR_PROC slot not used")))
                      .downcast_ref::<Weak<ProcRefCell<KProc>>>().expect(add_file!("Item at curproc was not the right type!"))
                      .clone().upgrade().expect(add_file!("Curproc has already been destroyed!"))
                      .deref().ensure_no_borrow();
        interrupt::set_ipl(ipl);
    }
}
