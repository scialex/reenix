
/// An implementation of some sync methods using spinners. These should be used with extreeme
/// caution and are only here to allow some stdlib provided initialization code.

use core::prelude::*;
use core::atomic::{AtomicUint, ATOMIC_UINT_INIT};
use core::atomic::Ordering::*;

/// We have never been called yet.
const UNUSED       : uint = 0;
/// We are currently running some initialization function
const INITIALIZING : uint = 1;
/// We have finished initializing.
const INITIALIZED  : uint = 2;

/// A once implementation that will have other threads spin when the initialization is being done.
/// Use only with extreeme caution. Prefer procs::sync::Once if possible.
pub struct SpinOnce {
    /// What the current state is.
    state: AtomicUint,
}

pub const SPIN_ONCE_INIT : SpinOnce = SpinOnce {
    state: ATOMIC_UINT_INIT,
};

impl SpinOnce {
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
            bassert!(
                self.state.compare_and_swap(INITIALIZING, INITIALIZED, SeqCst) == INITIALIZING,
                "Something illegally modified SpinOnce structure during execution!"
            );
            true
        } else if self.state.load(SeqCst) == INITIALIZED { false } else {
            if is_enabled!(REAL_SPIN_ONCE) {
                dbg!(debug::DANGER | debug::SCHED,
                    "Entered SpinOnce::try_it and it was initializing, \
                     Since no kernel preempt this might spin forever!");
                while self.state.load(SeqCst) != INITIALIZING { }
            } else {
                kpanic!("Entered SpinOnce::try_it and it was initializing, \
                         Since no kernel preempt this would probably spin forever!");
            }
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
