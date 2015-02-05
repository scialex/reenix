
#[macro_export]
macro_rules! add_file {
    ($s:expr) => ({ concat!(file!(),":",line!()," ", $s) })
}

#[macro_export]
macro_rules! current_thread{
    () => ({
        use startup::gdt;
        use procs::kthread::{CUR_THREAD_SLOT, KThread};
        unsafe {
            (**gdt::get_tsd().get_slot(CUR_THREAD_SLOT).expect(add_file!("CUR_THREAD slot not used")))
                             .downcast_ref::<*mut KThread>().expect(add_file!("Item at cur_thread was the wrong type"))
                             .as_mut().expect(add_file!("KThread was null"))
        }
    })
}

#[macro_export]
macro_rules! idle_proc{
    () => ({
        use procs::kproc::IDLE_PROC;
        unsafe { IDLE_PROC.as_mut().expect(add_file!("IDLE_PROC is not yet set")) }
    })
}

#[macro_export]
macro_rules! init_proc{
    () => ({
        use procs::kproc::INIT_PROC;
        unsafe { INIT_PROC.as_mut().expect(add_file!("INIT_PROC is not yet set")) }
    })
}

/// Returns the current pid. This is useful to avoid borrowing the current proc when it might
/// already be taken.
#[macro_export]
macro_rules! current_pid{
    () => ({
        use startup::gdt;
        use procs::kproc::ProcId;
        use procs::kproc::CUR_PID_SLOT;
        *((**gdt::get_tsd().get_slot(CUR_PID_SLOT).expect(add_file!("CUR_PID slot not used")))
                          .downcast_ref::<ProcId>().expect(add_file!("Item at curpid was not the right type!")))
    })
}

/// Returns an &'static mut KProc.
#[macro_export]
macro_rules! current_proc{
    () => ({
        use std::clone::*;
        use startup::gdt;
        use std::ops::Deref;
        use procs::pcell::*;
        use std::rc::*;
        use std::intrinsics::transmute;
        use procs::kproc::{CUR_PROC_SLOT, KProc};
        // We get the TSD copy of this data.
        let r = (**gdt::get_tsd().get_slot(CUR_PROC_SLOT).expect(add_file!("CUR_PROC slot not used")))
                        .downcast_ref::<Weak<ProcRefCell<KProc>>>().expect(add_file!("Item at curproc was not the right type!"))
                        .clone().upgrade().expect(add_file!("Curproc has already been destroyed!"));
        // Now we get the actual borrow.
        let v = r.deref().try_silent_borrow().expect(add_file!("Curproc is currently being borrowed by something!"));
        // Now we make that borrow have the 'static lifetime it actually has (for this thread).
        let out = |:| { unsafe { transmute::<&KProc, &'static KProc>(v.deref()) } };
        out()
    })
}
/// Returns an &'static mut KProc.
/// We do this since really the current process is a 'static but we need to let others access it
/// to. Therefore we do this stuff with a transmute.
#[macro_export]
macro_rules! current_proc_mut{
    () => ({
        use std::clone::*;
        use startup::gdt;
        use std::ops::Deref;
        use procs::pcell::*;
        use std::rc::*;
        use std::ops::DerefMut;
        use std::intrinsics::transmute;
        use procs::kproc::{CUR_PROC_SLOT, KProc};
        // We get the TSD copy of this data.
        let r = (**gdt::get_tsd().get_slot(CUR_PROC_SLOT).expect(add_file!("CUR_PROC slot not used")))
                        .downcast_ref::<Weak<ProcRefCell<KProc>>>().expect(add_file!("Item at curproc was not the right type!"))
                        .clone().upgrade().expect(add_file!("Curproc has already been destroyed!"));
        // Now we get the actual borrow.
        let mut v = r.deref().try_silent_borrow_mut().expect(add_file!("Curproc is currently being borrowed by something!"));
        // Now we make that borrow have the 'static lifetime it actually has (for this thread).
        let mut out = |:| { unsafe { transmute::<&mut KProc, &'static mut KProc>(v.deref_mut()) } };
        out()
    })
}

#[macro_export]
macro_rules! block_interrupts{
    ($e:expr) => ({
        use procs::interrupt;
        let ipl = interrupt::get_ipl();
        interrupt::set_ipl(interrupt::HIGH);
        let ret = $e;
        interrupt::set_ipl(ipl);
        ret
    })
}
