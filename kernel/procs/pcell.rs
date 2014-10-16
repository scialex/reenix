
use core::cell::*;
use core::kinds::marker;
use core::prelude::*;
use core::fmt;

type Borrowed = uint;
const UNUSED : Borrowed = 0;
const WRITING : Borrowed = -1;
/// A cell data structure that lets you borrow things 'silently'. This is used because the data in
/// these is associated with a single thread. Within the thread it is basically 'static and may be
/// treated as such. Outside the thread it is normal data with lifetimes and so on that could be
/// damaged if left hanging.
pub struct ProcRefCell<T> {
    value: UnsafeCell<T>,
    borrow: Cell<Borrowed>,
    nocopy: marker::NoCopy,
    noshare: marker::NoSync,
}

impl<T> ProcRefCell<T> {
    pub fn new(value: T) -> ProcRefCell<T> {
        ProcRefCell {
            value: UnsafeCell::new(value),
            borrow: Cell::new(UNUSED),
            nocopy: marker::NoCopy,
            noshare: marker::NoSync,
        }
    }

    /// Called to assert that there are no non-silent borrowers.
    pub fn ensure_no_borrow(&self) {
        if self.borrow.get() != UNUSED {
            panic!("Expected no non-silent borrowers but there were some present");
        }
    }

    /// Used to borrow within one's own thread where the variable has an implicit lifetime of
    /// 'static and we are (theoretically) borrowing from a static variable.
    pub fn try_silent_borrow<'a>(&'a self) -> Option<SilentProcRef<'a, T>> {
        match self.borrow.get() {
            WRITING => None,
            _ => {
                Some(SilentProcRef { _parent: self })
            }
        }
    }

    /// Used to borrow within one's own thread where the variable has an implicit lifetime of
    /// 'static and we are (theoretically) borrowing from a static variable.
    pub fn try_silent_borrow_mut<'a>(&'a self) -> Option<SilentProcRefMut<'a, T>> {
        match self.borrow.get() {
            UNUSED => {
                Some(SilentProcRefMut { _parent: self })
            },
            _ => None,
        }
    }

    /*
    /// Used to relinquish control of this before going to sleep. Must be paired with a restore
    /// state later. This lets us say we own it during our run but we can go to sleep and release
    /// it. This should only be used with current_proc!().
    pub fn save_state(&self) {
        assert!(self.prev_borrow.get() == None);
        self.prev_borrow.set(Some(self.borrow.get()));
        self.borrow.set(0);
    }

    /// Restore state of current_proc!(). This checks that the other thread didn't go to sleep with
    /// this being borrowed.
    pub fn restore_state(&self) {
        assert!(self.prev_borrow.get().is_some());
        assert!(self.borrow.get() == UNUSED);
        if let Some(x) = self.prev_borrow.get() {
            self.borrow.set(x);
            self.prev_borrow.set(None);
        } else {
            panic!("Previous borrow was none during call to restore_state");
        }
    }
    */

    /// Consumes the `ProcRefCell`, returning the wrapped value.
    #[unstable = "may be renamed, depending on global conventions"]
    pub fn unwrap(self) -> T {
        debug_assert!(self.borrow.get() == UNUSED);
        unsafe{self.value.unwrap()}
    }

    /// Attempts to immutably borrow the wrapped value.
    ///
    /// The borrow lasts until the returned `ProcRef` exits scope. Multiple
    /// immutable borrows can be taken out at the same time.
    ///
    /// Returns `None` if the value is currently mutably borrowed.
    #[unstable = "may be renamed, depending on global conventions"]
    pub fn try_borrow<'a>(&'a self) -> Option<ProcRef<'a, T>> {
        match self.borrow.get() {
            WRITING => None,
            borrow => {
                self.borrow.set(borrow + 1);
                Some(ProcRef { _parent: self })
            }
        }
    }

    /// Immutably borrows the wrapped value.
    ///
    /// The borrow lasts until the returned `ProcRef` exits scope. Multiple
    /// immutable borrows can be taken out at the same time.
    ///
    /// # Failure
    ///
    /// Fails if the value is currently mutably borrowed.
    #[unstable]
    pub fn borrow<'a>(&'a self) -> ProcRef<'a, T> {
        match self.try_borrow() {
            Some(ptr) => ptr,
            None => fail!("ProcRefCell<T> already mutably borrowed")
        }
    }

    /// Mutably borrows the wrapped value.
    ///
    /// The borrow lasts until the returned `ProcRefMut` exits scope. The value
    /// cannot be borrowed while this borrow is active.
    ///
    /// Returns `None` if the value is currently borrowed.
    #[unstable = "may be renamed, depending on global conventions"]
    pub fn try_borrow_mut<'a>(&'a self) -> Option<ProcRefMut<'a, T>> {
        match self.borrow.get() {
            UNUSED => {
                self.borrow.set(WRITING);
                Some(ProcRefMut { _parent: self })
            },
            _ => None
        }
    }

    /// Mutably borrows the wrapped value.
    ///
    /// The borrow lasts until the returned `ProcRefMut` exits scope. The value
    /// cannot be borrowed while this borrow is active.
    ///
    /// # Failure
    ///
    /// Fails if the value is currently borrowed.
    #[unstable]
    pub fn borrow_mut<'a>(&'a self) -> ProcRefMut<'a, T> {
        match self.try_borrow_mut() {
            Some(ptr) => ptr,
            None => fail!("ProcRefCell<T> already borrowed")
        }
    }
}

#[unstable = "waiting for `Clone` to become stable"]
impl<T: Clone> Clone for ProcRefCell<T> {
    fn clone(&self) -> ProcRefCell<T> {
        ProcRefCell::new(self.borrow().clone())
    }
}

#[unstable = "waiting for `PartialEq` to become stable"]
impl<T: PartialEq> PartialEq for ProcRefCell<T> {
    fn eq(&self, other: &ProcRefCell<T>) -> bool {
        *self.borrow() == *other.borrow()
    }
}

/// Wraps a silently borrowed reference to a value in a `ProcRefCell` box.
#[unstable]
pub struct SilentProcRef<'b, T:'b> {
    // FIXME #12808: strange name to try to avoid interfering with
    // field accesses of the contained type via Deref
    _parent: &'b ProcRefCell<T>
}

#[unstable = "waiting for `Deref` to become stable"]
impl<'b, T> Deref<T> for SilentProcRef<'b, T> {
    #[inline]
    fn deref<'a>(&'a self) -> &'a T {
        unsafe { &*self._parent.value.get() }
    }
}

/// Wraps a mutable silently borrowed reference to a value in a `ProcRefCell` box.
#[unstable]
pub struct SilentProcRefMut<'b, T:'b> {
    // FIXME #12808: strange name to try to avoid interfering with
    // field accesses of the contained type via Deref
    _parent: &'b ProcRefCell<T>
}


#[unstable = "waiting for `Deref` to become stable"]
impl<'b, T> Deref<T> for SilentProcRefMut<'b, T> {
    #[inline]
    fn deref<'a>(&'a self) -> &'a T {
        unsafe { &*self._parent.value.get() }
    }
}

#[unstable = "waiting for `DerefMut` to become stable"]
impl<'b, T> DerefMut<T> for SilentProcRefMut<'b, T> {
    #[inline]
    fn deref_mut<'a>(&'a mut self) -> &'a mut T {
        unsafe { &mut *self._parent.value.get() }
    }
}

impl<'b, T: fmt::Show> fmt::Show for SilentProcRef<'b, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        (**self).fmt(f)
    }
}

impl<'b, T: fmt::Show> fmt::Show for SilentProcRefMut<'b, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        (*(self.deref())).fmt(f)
    }
}

/// Wraps a borrowed reference to a value in a `ProcRefCell` box.
#[unstable]
pub struct ProcRef<'b, T:'b> {
    // FIXME #12808: strange name to try to avoid interfering with
    // field accesses of the contained type via Deref
    _parent: &'b ProcRefCell<T>
}

#[unsafe_destructor]
#[unstable]
impl<'b, T> Drop for ProcRef<'b, T> {
    fn drop(&mut self) {
        let borrow = self._parent.borrow.get();
        debug_assert!(borrow != WRITING && borrow != UNUSED);
        self._parent.borrow.set(borrow - 1);
    }
}

#[unstable = "waiting for `Deref` to become stable"]
impl<'b, T> Deref<T> for ProcRef<'b, T> {
    #[inline]
    fn deref<'a>(&'a self) -> &'a T {
        unsafe { &*self._parent.value.get() }
    }
}

/// Copy a `ProcRef`.
///
/// The `ProcRefCell` is already immutably borrowed, so this cannot fail.
///
/// A `Clone` implementation would interfere with the widespread
/// use of `r.borrow().clone()` to clone the contents of a `ProcRefCell`.
#[experimental = "likely to be moved to a method, pending language changes"]
pub fn clone_pref<'b, T>(orig: &ProcRef<'b, T>) -> ProcRef<'b, T> {
    // Since this ProcRef exists, we know the borrow flag
    // is not set to WRITING.
    let borrow = orig._parent.borrow.get();
    debug_assert!(borrow != WRITING && borrow != UNUSED);
    orig._parent.borrow.set(borrow + 1);

    ProcRef {
        _parent: orig._parent,
    }
}

/// Wraps a mutable borrowed reference to a value in a `ProcRefCell` box.
#[unstable]
pub struct ProcRefMut<'b, T:'b> {
    // FIXME #12808: strange name to try to avoid interfering with
    // field accesses of the contained type via Deref
    _parent: &'b ProcRefCell<T>
}

#[unsafe_destructor]
#[unstable]
impl<'b, T> Drop for ProcRefMut<'b, T> {
    fn drop(&mut self) {
        let borrow = self._parent.borrow.get();
        debug_assert!(borrow == WRITING);
        self._parent.borrow.set(UNUSED);
    }
}

#[unstable = "waiting for `Deref` to become stable"]
impl<'b, T> Deref<T> for ProcRefMut<'b, T> {
    #[inline]
    fn deref<'a>(&'a self) -> &'a T {
        unsafe { &*self._parent.value.get() }
    }
}

#[unstable = "waiting for `DerefMut` to become stable"]
impl<'b, T> DerefMut<T> for ProcRefMut<'b, T> {
    #[inline]
    fn deref_mut<'a>(&'a mut self) -> &'a mut T {
        unsafe { &mut *self._parent.value.get() }
    }
}

impl<'b, T: fmt::Show> fmt::Show for ProcRef<'b, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        (**self).fmt(f)
    }
}

impl<'b, T: fmt::Show> fmt::Show for ProcRefMut<'b, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        (*(self.deref())).fmt(f)
    }
}
