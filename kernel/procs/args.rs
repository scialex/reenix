
use mm::alloc::*;
use alloc::boxed::*;
use core::prelude::*;
use core::cell::*;
use core::mem::{transmute, transmute_copy};
use libc::c_void;
use core::intrinsics::forget;

pub struct ProcArgs<T> {
    val: Box<UnsafeCell<T>>,
}

impl<T: Sized> ProcArgs<T> {
    pub fn new(v: T) -> Allocation<ProcArgs<T>> {
        Ok(ProcArgs { val: try!(alloc!(try_box UnsafeCell::new(v))), })
    }

    pub unsafe fn to_arg(self) -> *mut c_void {
        let out = transmute_copy::<Box<UnsafeCell<T>>, *mut c_void>(&(&self).val);
        forget(self);
        out
    }

    pub unsafe fn from_arg(v: *mut c_void) -> ProcArgs<T> {
        assert!(!v.is_null());
        ProcArgs { val: transmute(v), }
    }

    pub fn unwrap(self) -> T { unsafe { (*self.val).unwrap() } }
}
