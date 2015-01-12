
//! Stuff to make a Linked List.
use core::prelude::*;
use alloc::boxed::*;
use core::cell::*;
use core::mem::{transmute, transmute_copy};

/// A node in a linked list used to for various things in this crate. Specifically it is used to
/// implement both the LruCache and the pinnable cache by providing the backing storage to hold the
/// actual items in the cache.
pub struct ListNode<K, V> {
    val : Option<(K, V)>,
    next: Cell<*mut ListNode<K, V>>,
    prev: Cell<*mut ListNode<K, V>>,
}

impl<K, V> ListNode<K, V> {
    pub fn new(k: K, v: V) -> Box<ListNode<K, V>> {
        let mut res = ListNode::initial();
        res.val = Some((k, v));
        res
    }

    /// Create the initial list node. This is the only one which has a val of None. This should
    /// never be inserted into a list.
    pub fn initial() -> Box<ListNode<K, V>> {
        let res = box ListNode {
            val : None,
            next: Cell::new(0 as *mut ListNode<K, V>),
            prev: Cell::new(0 as *mut ListNode<K, V>),
        };
        unsafe {
            res.next.set(transmute_copy(&res));
            res.prev.set(transmute_copy(&res));
        }
        res
    }

    #[inline]
    pub fn is_in_list(&self) -> bool {
        assert!(!self.next.get().is_null() && !self.prev.get().is_null(), "Illegal next or prev values for node");
        self.prev.get() == self.next.get() && self.prev.get() == unsafe { transmute(self) }
    }

    /// Take this entry out of the list (if it is in one). This needs to take a non-mutable pointer
    /// so we can do this even in cases where we are doing (i.e. get).
    pub fn remove_self(&self) {
        assert!(!self.is_start());
        self.next().prev.set(self.prev.get());
        self.prev().next.set(self.next.get());
        unsafe {
            self.next.set(transmute(self));
            self.prev.set(transmute(self));
        }
    }
    /// Marks this one as being used less recently than o, but more recently than o.next()
    pub fn insert_after(&self, o: &ListNode<K, V>) {
        assert!(!self.is_start());
        assert!(!self.is_in_list());
        self.next.set(o.next.get());
        unsafe {
            self.prev.set(transmute(&*o));
            self.prev().next.set(transmute(&*self));
            self.next().prev.set(transmute(&*self));
        }
    }
    /// Get the one used less recently then this one.
    pub fn next_mut<'a>(&'a mut self) -> &'a mut ListNode<K, V> {
        unsafe { self.next.get().as_mut().expect("Bad Linked List state, no next pointer") }
    }
    /// Get the one that was used more recently than this one.
    pub fn prev_mut<'a>(&'a mut self) -> &'a mut ListNode<K, V> {
        unsafe { self.prev.get().as_mut().expect("Bad Linked List state, no prev pointer") }
    }
    pub fn next<'a>(&'a self) -> &'a ListNode<K, V> {
        unsafe { self.next.get().as_ref().expect("Bad Linked List state, no next pointer") }
    }
    pub fn prev<'a>(&'a self) -> &'a ListNode<K, V> {
        unsafe { self.prev.get().as_ref().expect("Bad LinkedList state, no prev pointer") }
    }

    #[inline]
    pub fn is_start(&self) -> bool { self.val.is_none() }

    /// Return a ref to the value. Panic if we are not a list element.
    pub fn value<'a>(&'a self) -> &'a V { self.entry().1 }

    /// Return a ref mut to the value. Panic if we are not a list element.
    pub fn value_mut<'a>(&'a mut self) -> &'a mut V { self.entry_mut().1 }

    /// Return a ref to the key. Panic if we are not a list element.
    pub fn key<'a>(&'a self) -> &'a K { self.entry().0 }

    /// Take the value from the element. This destroys the element. The element cannot be in any
    /// cache when this is called.
    pub fn take(self) -> V { self.take_full().1 }

    /// Take the value and key from the element. This destroys the element. The element cannot be
    /// in any cache when this is called.
    pub fn take_full(mut self) -> (K, V) {
        assert!(!self.is_start(),   "Attempting to take value of start element of node list!");
        assert!(!self.is_in_list(), "Attempting to take value of a element still in node list!");
        self.val.take().expect("Take called on start element")
    }

    pub fn entry_mut<'a>(&'a mut self) -> (&'a K, &'a mut V) {
        match self.val {
            Some((ref k, ref mut v)) => (k, v),
            None => kpanic!("entry_mut called on start element"),
        }
    }

    pub fn entry<'a>(&'a self) -> (&'a K, &'a V) {
        match self.val {
            Some((ref k, ref v)) => (k, v),
            None => kpanic!("entry called on start element"),
        }
    }
}
