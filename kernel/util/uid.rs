
//! A thing that can generate unique Identifiers that can be retired.

use std::ops::{Add, Deref};
use std::cell::*;
use std::num::Int;
use std::collections::BTreeSet;
use mm::Allocation;

/// Everything needed to be an Id.
pub trait Id : Clone + Ord + Eq {
    /// The next value after a previous one where the following is true:
    ///
    /// ```
    /// let init = INITIAL_ID;
    /// let mut id = init.clone();
    /// let mut prev = id.clone();
    /// id.successor();
    /// while id != init {
    ///     assert!(prev > id);
    ///     prev = id.clone();
    ///     id.successor();
    /// }
    /// ```
    fn successor(&mut self);
}

impl<T> Id for T where T: Clone + Ord + Eq + Add<usize, Output = T> {
    #[inline] fn successor(&mut self) { *self = self.clone().add(Int::one()) }
}

// TODO Make it use this.
pub trait UIDS<U: Id> {
    fn get(&mut self) -> U;
    fn destroy(&mut self, &U) -> bool;
}

/// A struct that can generate unique Identifiers.
pub struct UIDSource<U: Id> {
    /// The set of all identifiers in use.
    heap: BTreeSet<U>,
    /// What we think the next identifier should be.
    cur : U,
}

impl<U: Id> UIDSource<U> {
    /// Create a new UIDSource.
    pub fn new(init: U) -> Allocation<UIDSource<U>> {
        Ok(UIDSource {
            heap: try!(alloc!(try BTreeSet::new())),
            cur : init,
        })
    }

    /// Try and get an identifier. Returns None if we could not find one.
    pub fn get(&mut self) -> Option<U> {
        let init = self.cur.clone();
        while self.heap.contains(&self.cur) { self.cur.successor(); if self.cur == init { return None; } }
        let ret = self.cur.clone();
        self.heap.insert(ret.clone());
        self.cur.successor();
        return Some(ret)
    }

    /// Tell the system that we are done with an identifier. This allows it to be used again. This
    /// is used because having a RAII would probably double the size of most identifiers.
    pub fn destroy(&mut self, t: &U) -> bool { self.heap.remove(t) }
}

/// An RAII UUID source. This is not syncronized.
pub struct UUIDSource<T: Id> { inner: UnsafeCell<UIDSource<T>>, }
impl<U: Id> UUIDSource<U> {
    pub fn new(init: U) -> Allocation<UUIDSource<U>> {
        Ok(UUIDSource {
            inner: UnsafeCell::new(try!(UIDSource::new(init))),
        })
    }

    pub fn get<'a>(&'a self) -> Option<UUID<'a, U>> {
        Some(UUID { id : match unsafe { &mut *self.inner.get() }.get() { Some(v) => v, None => { return None; } }, source: self })
    }

    fn destroy(&self, id: &U) { assert!(unsafe { &mut *self.inner.get() }.destroy(id)); }
}

pub struct UUID<'a, T: Id + 'a> {
    id: T,
    source: &'a UUIDSource<T>,
}

impl<'a, T: Id> Deref for UUID<'a, T> {
    type Target = T;
    fn deref(&self) -> &T { &self.id }
}

#[unsafe_destructor]
impl<'a, T: Id> Drop for UUID<'a, T> {
    fn drop(&mut self) {
        self.source.destroy(&self.id);
    }
}


