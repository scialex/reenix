
use mm::alloc::*;
use std::cell::*;
use libc::c_void;
use std::boxed;

pub struct ProcArgs<T> {
    val: Box<UnsafeCell<T>>,
}

impl<T: Sized> ProcArgs<T> {
    #[inline(never)]
    pub fn new(v: T) -> Allocation<ProcArgs<T>> {
        Ok(ProcArgs { val: try!(alloc!(try_box UnsafeCell::new(v))), })
    }

    pub unsafe fn to_arg(self) -> *mut c_void {
        let ProcArgs { val } = self;
        boxed::into_raw(val) as *mut c_void
    }

    pub unsafe fn from_arg(v: *mut c_void) -> ProcArgs<T> {
        assert!(!v.is_null());
        ProcArgs { val: Box::from_raw(v as *mut UnsafeCell<T>), }
    }

    pub fn unwrap(self) -> T { unsafe { (*self.val).into_inner() } }
}
