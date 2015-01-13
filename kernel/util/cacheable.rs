//! The cacheable trait.

use alloc::rc::*;
use core::cell::*;
use core::ptr::*;

/// A trait that indicates the type could be dropped but we will not nessecarially want to do so.
/// For example think of a data-point in an LRU cache, even if we have no reffernces if the data
/// can be recovered we might want to keep it around if there is no urgent need for new space.
pub trait Cacheable {
    /// This is called if we have determined this object is dropable (no body references it) and
    /// wish to see if it would be better to save it or not.
    /// Note the caller is allowed to ignore this value and might drop it anyways even if true is
    /// returned. Further note the default implementation of this returns false, one should be very
    /// sure that this value is actually useful before returning true.
    fn is_still_useful(&self) -> bool;
}

impl<T> Cacheable for Rc<T> {
    // TODO With negative bounds we could have a much nicer version of this.
    fn is_still_useful(&self) -> bool { strong_count(self) != 1 }
}

macro_rules! base_deriving {
    ($v:ty) => (impl Cacheable for $v { fn is_still_useful(&self) -> bool { true } })
}

base_deriving!(u8);
base_deriving!(i8);
base_deriving!(u16);
base_deriving!(i16);
base_deriving!(u32);
base_deriving!(i32);
base_deriving!(u64);
base_deriving!(i64);
base_deriving!(usize);
base_deriving!(isize);

impl<T> Cacheable for UnsafeCell<T> where T: Cacheable {
    fn is_still_useful(&self) -> bool { unsafe { self.get().as_ref().expect("cannot be null").is_still_useful() } }
}
