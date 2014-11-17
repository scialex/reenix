use core::prelude::*;
use core::atomic::*;
use core::cell::UnsafeCell;
use core::ptr::*;

pub use kmutex::KMutex;
pub use kqueue::WQueue;
use kthread;

/// A type where you can send a signal on. This is usually paired with Wait.
pub trait Wakeup {
    /// Send a wakeup to this object, notifying it that something has happened.
    fn signal(&self);
}

/// A wakeup type where you can gaurentee that only one thread is woken up.
pub trait WakeupOne: Wakeup {
    /// Return true if we wokeup a thread.
    fn signal_one(&self) -> bool;
}

/// A trait that lets you wait on something. A wait either succeeds or fails and the caller is left
/// to deal with both cases.
pub trait Wait<R,E> {
    /// Returns true if we successfully waited. False if we were cancelled or something else went
    /// wrong.
    fn wait<'a>(&'a self) -> Result<R,E>;
}

/// An RAII Based mutex with auto-unlocking.
pub struct SMutex { inner: KMutex, wqueue: WQueue, }

/// An RAII mutex guard. It will automatically unlock the mutex when it goes out of scope.
pub struct SGuard<'a> { lock: &'a SMutex, }

impl<'a> Wait<(),()> for SGuard<'a> {
    fn wait(&self) -> Result<(),()> { self.lock.wait() }
}

impl SMutex {
    pub fn new(name: &'static str) -> SMutex {
        SMutex {
            inner: KMutex::new(name),
            wqueue: WQueue::new(),
        }
    }
    fn unlock(&self) { self.inner.unlock(); }
    fn wait(&self) -> Result<(),()> {
        block_interrupts!({
            dbg!(debug::SCHED, "{} going to sleep on mutex {} with queue {}", current_proc!(), self.inner, self.wqueue);
            self.unlock();
            let res = self.wqueue.wait();
            // Even if we failed this lock still needs to be valid.
            self.inner.lock_nocancel();
            res
        })
    }
    pub fn force_lock<'a>(&'a self) -> SGuard<'a> {
        self.inner.lock_nocancel();
        SGuard { lock: self }
    }

    pub fn lock<'a>(&'a self) -> Result<SGuard<'a>, ()> {
        if self.inner.lock() {
            Ok(SGuard { lock: self })
        } else {
            Err(())
        }
    }

    pub fn try_lock<'a>(&'a self) -> Option<SGuard<'a>> {
        if self.inner.try_lock() {
            Some(SGuard { lock: self })
        } else {
            None
        }
    }
}

impl Wakeup for SMutex {
    fn signal(&self) { dbg!(debug::SCHED, "sending wakeup on {} with {}", self.inner, self.wqueue); self.wqueue.signal(); }
}

#[unsafe_destructor]
impl<'a> Drop for SGuard<'a> {
    fn drop(&mut self) {
        self.lock.unlock();
    }
}

/// A mutex that holds some data.
pub struct Mutex<T> {
    _lock: SMutex,
    _data: UnsafeCell<T>,
}

pub struct MGuard<'a, T: 'a> {
    _data: &'a mut T,
    _lock: SGuard<'a>,
}

impl<'a, T:'a> Wait<(),()> for MGuard<'a, T> {
    fn wait(&self) -> Result<(), ()> { self._lock.wait() }
}

impl<T> Mutex<T> {
    pub fn new(name: &'static str, data: T) -> Mutex<T> {
        Mutex {
            _lock: SMutex::new(name),
            _data: UnsafeCell::new(data),
        }
    }
    pub fn lock<'a>(&'a self) -> Result<MGuard<'a, T>, ()> {
        let g = try!(self._lock.lock());
        Ok(MGuard {
            _data: unsafe { self._data.get().as_mut().expect("data in mutex shouldn't be null") },
            _lock: g,
        })
    }

    pub fn force_lock<'a>(&'a self) -> MGuard<'a, T> {
        let l = self._lock.force_lock();
        MGuard {
            _data: unsafe { self._data.get().as_mut().expect("data in mutex shouldn't be null") },
            _lock: l,
        }
    }

    pub fn try_lock<'a>(&'a self) -> Option<MGuard<'a, T>> {
        self._lock.try_lock()
                  .map(|t| { MGuard {
                                    _data: unsafe { self._data.get().as_mut().expect("data in mutex shouldnt be null") },
                                    _lock: t,
                                }
                            })
    }
}

impl<T> Wakeup for Mutex<T> {
    fn signal(&self) { self._lock.signal(); }
}

impl<'a, T> Deref<T> for MGuard<'a, T> {
    fn deref<'a>(&'a self) -> &'a T { &*self._data }
}

impl<'a, T> DerefMut<T> for MGuard<'a, T> {
    fn deref_mut<'a>(&'a mut self) -> &'a mut T { &mut *self._data }
}

/// A mutex with a containted condition that things can wait for.
pub struct CondMutex<T> {
    cond: fn(&T) -> bool,
    mtx : Mutex<T>,
}

pub struct CGuard<'a, T: 'a> {
    _lock: MGuard<'a, T>,
    _mtx : &'a CondMutex<T>,
    _sig : bool,
}

impl<'a, T: 'a> CGuard<'a, T> {
    pub fn force_wait(&self) -> Result<(),()> {
        self._lock.wait().and_then(|_| { self.wait() })
    }
}

#[unsafe_destructor]
impl<'a, T:'a> Drop for CGuard<'a, T> {
    fn drop(&mut self) {
        let r = self._mtx.cond;
        if r((*self).deref()) { self._mtx.signal(); }
    }
}

impl<'a, T> Wait<(),()> for CGuard<'a, T> {
    fn wait(&self) -> Result<(),()> {
        let r = self._mtx.cond;
        while !r(self.deref()) {
            try!(self._lock.wait());
        }
        Ok(())
    }
}

impl<'a, T> Deref<T> for CGuard<'a, T> {
    fn deref<'a>(&'a self) -> &'a T { self._lock.deref() }
}

impl<'a, T> DerefMut<T> for CGuard<'a, T> {
    fn deref_mut<'a>(&'a mut self) -> &'a mut T { self._lock.deref_mut() }
}


impl<T> CondMutex<T> {
    pub fn new(name: &'static str, data: T, cond: fn(&T)->bool) -> CondMutex<T> {
        CondMutex {
            mtx : Mutex::new(name, data),
            cond: cond,
        }
    }
    /// Lock the mutex, if we get it and the state of the value causes condition to return true,
    /// send the signal to sleeping threads.
    pub fn lock<'a>(&'a self, send_signal: bool) -> Result<CGuard<'a, T>, ()> {
        let g = try!(self.mtx.lock());
        Ok(CGuard {
            _lock: g,
            _mtx : self,
            _sig : send_signal,
        })
    }

    /// Lock the mutex, if we get it and the state of the value causes condition to return true,
    /// send the signal to sleeping threads.
    pub fn force_lock<'a>(&'a self, send_signal: bool) -> CGuard<'a, T> {
        let l = self.mtx.force_lock();
        CGuard {
            _lock: l,
            _mtx : self,
            _sig : send_signal,
        }
    }

    /// Lock the mutex, if we get it and the state of the value causes condition to return true,
    /// send the signal to sleeping threads.
    pub fn try_lock<'a>(&'a self, send_signal: bool) -> Option<CGuard<'a, T>> {
        self.mtx.try_lock()
                .map(|t| { CGuard {
                                    _lock: t,
                                    _mtx : self,
                                    _sig : send_signal,
                                }
                            })
    }
}

impl<T> Wakeup for CondMutex<T> {
    fn signal(&self) { self.mtx.signal(); }
}

/// An RAII Based mutex with auto-unlocking.
/// This uses spin-lock semantics for locking in interrupt handlers.
pub struct SpinLock { inner: AtomicBool, }

/// A RAII Guard.
pub struct SpinGuard<'a> { lock: &'a SpinLock, }

impl SpinLock {
    /// Create a new spinlock
    pub fn new() -> SpinLock {
        SpinLock { inner: AtomicBool::new(false), }
    }

    /// Do the real unlock operation.
    fn unlock(&self) { assert!(self.inner.swap(false, SeqCst)); }

    /// Lock the spin lock.
    pub fn force_lock<'a>(&'a self) -> SpinGuard<'a> {
        while self.inner.compare_and_swap(false, true, SeqCst) { kthread::kyield(); }
        SpinGuard { lock: self }
    }

    /// Try to lock the spin lock, will fail if we are canceled before we get the lock.
    #[inline]
    pub fn lock<'a>(&'a self) -> Result<SpinGuard<'a>, ()> {
        while self.inner.compare_and_swap(false, true, SeqCst) {
            if (current_thread!()).cancelled {
                return Err(());
            } else {
                kthread::kyield();
            }
        }
        Ok(SpinGuard { lock: self })
    }

    /// Try to lock the spin lock, will return None if the lock is already held.
    pub fn try_lock<'a>(&'a self) -> Option<SpinGuard<'a>> {
        if !self.inner.compare_and_swap(false, true, SeqCst) {
            Some(SpinGuard { lock: self })
        } else {
            None
        }
    }
}

#[unsafe_destructor]
impl<'a> Drop for SpinGuard<'a> {
    fn drop(&mut self) {
        self.lock.unlock();
    }
}

/// A spin-lock mutex that holds some data.
pub struct SpinMutex<T> {
    _lock: SpinLock,
    _data: UnsafeCell<T>,
}

/// A RAII Guard that is the data the lock contains.
pub struct SMGuard<'a, T: 'a> {
    _data: &'a mut T,
    _lock: SpinGuard<'a>,
}

impl<T> SpinMutex<T> {
    pub fn new(data: T) -> SpinMutex<T> {
        SpinMutex {
            _lock: SpinLock::new(),
            _data: UnsafeCell::new(data),
        }
    }
    pub fn lock<'a>(&'a self) -> Result<SMGuard<'a, T>, ()> {
        let g = try!(self._lock.lock());
        Ok(SMGuard {
            _data: unsafe { self._data.get().as_mut().expect("data in mutex shouldn't be null") },
            _lock: g,
        })
    }

    pub fn force_lock<'a>(&'a self) -> SMGuard<'a, T> {
        let l = self._lock.force_lock();
        SMGuard {
            _data: unsafe { self._data.get().as_mut().expect("data in mutex shouldn't be null") },
            _lock: l,
        }
    }

    pub fn try_lock<'a>(&'a self) -> Option<SMGuard<'a, T>> {
        self._lock.try_lock()
                  .map(|t| { SMGuard {
                                    _data: unsafe { self._data.get().as_mut().expect("data in mutex shouldnt be null") },
                                    _lock: t,
                                }
                            })
    }
}

impl<'a, T> Deref<T> for SMGuard<'a, T> {
    fn deref<'a>(&'a self) -> &'a T { &*self._data }
}

impl<'a, T> DerefMut<T> for SMGuard<'a, T> {
    fn deref_mut<'a>(&'a mut self) -> &'a mut T { &mut *self._data }
}
