
use core::ops::{Deref, DerefMut};
use core::cell::*;
use core::prelude::*;
use core::fmt;

/// A Structure which allows unfiltered interior mutability.
///
/// Use of this is an assertion that unsyncronized mutability is safe.
pub struct SafeCell<T> { inner: UnsafeCell<T> }

unsafe impl<T> Sync for SafeCell<T> where T: Sync {}
unsafe impl<T> Send for SafeCell<T> where T: Send {}

impl<T> SafeCell<T> {
    /// Constructs a new safe cell.
    pub fn new(value: T) -> SafeCell<T> { SafeCell { inner: UnsafeCell::new(value) } }
    /// Gets an immutable reference to the value in this cell.
    pub fn get_ref<'a>(&'a self) -> SafeRef<'a, T> { SafeRef(&self.inner) }
    /// Gets mutable reference to the value in this cell.
    pub fn get_mut<'a>(&'a self) -> SafeMutRef<'a, T> { SafeMutRef(&self.inner) }
}

impl<T> fmt::Debug for SafeCell<T> where T: fmt::Debug {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { (&*self.get_ref()).fmt(f) }
}

impl<T> fmt::Display for SafeCell<T> where T: fmt::Display {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { (&*self.get_ref()).fmt(f) }
}

pub struct SafeMutRef<'a, T: 'a>(&'a UnsafeCell<T>);

impl<'a, T> fmt::Debug for SafeMutRef<'a, T> where T: fmt::Debug {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { (&**self).fmt(f) }
}

impl<'a, T> fmt::Display for SafeMutRef<'a, T> where T: fmt::Display {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { (&**self).fmt(f) }
}

impl<'a, T> DerefMut for SafeMutRef<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { self.0.get().as_mut().expect("SafeCell cannot be null") }
    }
}

impl<'a, T> Deref for SafeMutRef<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { self.0.get().as_ref().expect("SafeCell cannot be null") }
    }
}

pub struct SafeRef<'a, T: 'a>(&'a UnsafeCell<T>);

impl<'a, T> Deref for SafeRef<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { self.0.get().as_ref().expect("SafeCell cannot be null") }
    }
}
impl<'a, T> fmt::Debug for SafeRef<'a, T> where T: fmt::Debug {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { (&**self).fmt(f) }
}

impl<'a, T> fmt::Display for SafeRef<'a, T> where T: fmt::Display {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { (&**self).fmt(f) }
}
