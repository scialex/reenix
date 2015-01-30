
//! A basic std::sync front;
//!
//! When compiled with feature="spin" we will include as many of the sync primitives as we can
//! using only atomic spin lock style implementations.
//!
//! Note we can only provide Mutex and Once in this case.

pub use core::atomic;
pub use alloc::arc::{Arc, Weak};
#[cfg(feature="spin")] pub use self::spin::*;
#[cfg(feature="spin")] pub use self::poison::*;

#[cfg(feature="spin")] #[path="../../../external/rust/src/libstd/sync/poison.rs"] mod poison;
#[cfg(feature="spin")] mod spin {
    use super::atomic::{AtomicBool, AtomicUsize, ATOMIC_USIZE_INIT};
    use super::atomic::Ordering::SeqCst;
    use cell::*;
    use super::poison::*;
    use mem::transmute;
    use ops::{Deref, DerefMut};
    use prelude::v1::*;

    pub struct MutexGuard<'a, T: 'a> { __mtx : &'a Mutex<T>, }
    impl<'a, T: 'a> !Send for MutexGuard<'a, T> {}

    impl<'m, T> DerefMut for MutexGuard<'m, T> {
        fn deref_mut<'a>(&'a mut self) -> &'a mut T {
            unsafe { transmute(self.__mtx.val.get()) }
        }
    }

    impl<'m, T> Deref for MutexGuard<'m, T> {
        type Target = T;
        fn deref<'a>(&'a self) -> &'a T {
            unsafe { transmute(self.__mtx.val.get()) }
        }
    }

    #[unsafe_destructor]
    impl<'a, T: 'a> Drop for MutexGuard<'a, T> {
        fn drop(&mut self) { self.__mtx.locked.swap(false, SeqCst); }
    }

    pub struct Mutex<T> {
        locked: AtomicBool,
        val: UnsafeCell<T>,
    }

    unsafe impl<T: Send> Send for Mutex<T> {}
    unsafe impl<T: Send> Sync for Mutex<T> {}

    impl<T: Send> Mutex<T> {
        /// See std::sync::Mutex::new
        pub fn new(t: T) -> Mutex<T> {
            Mutex { locked: AtomicBool::new(false), val: UnsafeCell::new(t) }
        }
        pub fn lock(&self) -> LockResult<MutexGuard<T>> {
            while self.locked.compare_and_swap(false, true, SeqCst) { }
            Ok(MutexGuard { __mtx: self })
        }
        pub fn try_lock(&self) -> TryLockResult<MutexGuard<T>> {
            if self.locked.compare_and_swap(false, true, SeqCst) {
                Err(TryLockError::WouldBlock)
            } else {
                Ok(MutexGuard { __mtx: self })
            }
        }
    }

    /// We have never been called yet.
    const UNUSED       : usize = 0;
    /// We are currently running some initialization function
    const INITIALIZING : usize = 1;
    /// We have finished initializing.
    const INITIALIZED  : usize = 2;

    /// A once implementation that will have other threads spin when the initialization is being done.
    /// Use only with extreeme caution. Prefer procs::sync::Once if possible.
    pub struct Once {
        /// What the current state is.
        state: AtomicUsize,
    }

    pub const ONCE_INIT : Once = Once {
        state: ATOMIC_USIZE_INIT,
    };

    impl Once {
        /// Perform an initialization routine once and only once. The given closure will be executed if
        /// this is the first time `try_it` has been called, and otherwise the routine will *not* be
        /// invoked.
        ///
        /// This method will spin until at least one initialization routine has been completed.
        ///
        /// Returns true if the function given was executed, false otherwise.
        pub fn try_it<F>(&self, f: F) -> bool where F: Fn() {
            if self.state.load(SeqCst) == INITIALIZED {
                false
            } else if self.state.compare_and_swap(UNUSED, INITIALIZING, SeqCst) == UNUSED {
                // We are the ones who won the race.
                f();
                self.state.store(INITIALIZED, SeqCst);
                true
            } else if self.state.load(SeqCst) == INITIALIZED { false } else {
                while self.state.load(SeqCst) != INITIALIZED { }
                false
            }
        }

        /// Perform an initialization routine once and only once. The given closure will be executed if
        /// this is the first time `doit` has been called, and otherwise the routine will *not* be
        /// invoked.
        ///
        /// This method will block the calling task if another initialization routine is currently
        /// running.
        ///
        /// When this function returns, it is guaranteed that some initialization has run and completed
        /// (it may not be the closure specified).
        #[inline]
        pub fn doit<F>(&self, f: F) where F: Fn() { self.try_it(f); }
    }
}
