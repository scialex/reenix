
#![macro_escape]

#[macro_export]
macro_rules! current_thread(
    () => ({
        use startup::gdt;
        use core::any::*;
        use procs::kthread::{CUR_THREAD_SLOT, KThread};
        use core::ptr::RawMutPtr;
        unsafe {
            (**gdt::get_tsd().get_slot(CUR_THREAD_SLOT).expect("CUR_THREAD slot not used"))
                             .downcast_ref::<*mut KThread>().expect("Item at cur_thread was the wrong type")
                             .as_mut().expect("KThread was null")
        }
    })
)

#[macro_export]
macro_rules! idle_proc(
    () => ({
        use procs::kproc::IDLE_PROC;
        unsafe { IDLE_PROC.as_mut().expect("IDLE_PROC is not yet set") }
    })
)

#[macro_export]
macro_rules! init_proc(
    () => ({
        use procs::kproc::INIT_PROC;
        unsafe { INIT_PROC.as_mut().expect("INIT_PROC is not yet set") }
    })
)

#[macro_export]
macro_rules! add_file(
    ($s:expr) => ({ concat!(file!(),":",line!()," ", $s) })
)

/// Returns the current pid. This is useful to avoid borrowing the current proc when it might
/// already be taken.
macro_rules! current_pid(
    () => ({
        use startup::gdt;
        use core::any::*;
        (*(**gdt::get_tsd().get_slot(CUR_PID_SLOT).expect(add_file!("CUR_PID slot not used")))
                           .downcast_ref::<ProcId>().expect(add_file!("Item at curpid was not the right type!")))
    })
)

/// Returns an &'static mut KProc.
#[macro_export]
macro_rules! current_proc(
    () => ({
        use core::clone::*;
        use startup::gdt;
        use core::ops::Deref;
        use procs::pcell::*;
        use alloc::rc::*;
        use core::any::*;
        use procs::kproc::{CUR_PROC_SLOT, KProc};
        (**gdt::get_tsd().get_slot(CUR_PROC_SLOT).expect(add_file!("CUR_PROC slot not used")))
                      .downcast_ref::<Weak<ProcRefCell<KProc>>>().expect(add_file!("Item at curproc was not the right type!"))
                      .clone().upgrade().expect(add_file!("Curproc has already been destroyed!"))
                      .deref().try_borrow_mut().expect(add_file!("Curproc is currently being borrowed by something!"))
    })
)

#[macro_export]
macro_rules! block_interrupts(
    ($e:expr) => ({
        use procs::interrupt;
        let ipl = interrupt::get_ipl();
        interrupt::set_ipl(interrupt::HIGH);
        let ret = $e;
        interrupt::set_ipl(ipl);
        ret
    })
)
