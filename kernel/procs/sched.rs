// TODO Copyright Header
// TODO GET RID FO THIS

//! The scheduler internals

struct SleepingThread(*mut KThread);
static mut runq : Option<*mut RingBuf<SleepingThread>> = None;
fn add_thread(thr : *KThread) {
    // TODO
}

/// Places the thread in a runqueue and then switches, 
pub fn yield(thr : &KThread) {
    // TODO
    add_thread(thr);
    switch(thr);
}

/// Switches away from the thread, If the thread is later resumed it will be started after this
/// call.
pub fn switch(thr : &KThread) {
    let curipl = interrupt::get_ipl();
    interrupt::set_ipl(interrupt::HIGH);
    let nxt = get_next_thr();
    assert!(interrupt::HIGH == interrupt::get_ipl());
    
}

static mut dead_ctx : *mut Context = 0 as *mut Context;

/// Switches away dieing. This function never returns. It does not update the context of the
/// calling thread.
pub fn die() -> ! {
    // TODO
    interrupt::set_ipl(interrupt::HIGH);
    let nxt = get_next_thr();
    assert!(interrupt::HIGH == interrupt::get_ipl());
    nxt.make_active(nxt.ctx);
}

fn get_next_thr() -> *mut KThread {
}

pub fn init_stage1() {  /* TODO */ }
