// TODO Copyright Header

//! Some utilities.

use core::prelude::*;
use core::option::*;
use core::mem;
use core::ptr;
use core::ptr::RawMutPtr;

// TODO I should actually make this a balancing tree by making Node have values, Option<left,right>
// etc.
pub enum LightNode<T: Clone> {
    // key is the max value on the left subtree.
    Node { left: *mut LightNode<T>, right: *mut LightNode<T>, key: uint },
    Leaf { key: uint, val: T }
}

macro_rules! deref(
    ($v:expr) => (unsafe {$v.as_ref()}.expect("Should never be null"))
)
macro_rules! deref_mut(
    ($v:expr) => (unsafe {$v.as_mut()}.expect("Should never be null"))
)


pub type LightNodeAlloc<T> = unsafe fn() -> *mut LightNode<T>;

impl<T: Clone> LightNode<T> {
    /// Initialize a leaf node. Note that since this is for base mem management only a failure to
    /// allocate is considered unrecoverable.
    #[inline]
    fn init_leaf(alloc: LightNodeAlloc<T>, k: uint, v: T) -> *mut LightNode<T> {
        let tmp = unsafe { alloc() };
        if tmp == ptr::RawPtr::null() {
            fail!("Unable to allocate memory for leaf");
        }
        let val = Leaf { key: k, val: v };
        unsafe { ptr::write(tmp, val); }
        tmp
    }

    //#[inline]
    //fn init_node(alloc: unsafe || -> *mut LightNode<T>, l: *mut LightNode<T>, r: Option<*mut LightNode<T>>, k: uint) {
    //    let tmp = unsafe { alloc() };
    //    if tmp == ptr::null() {
    //        fail!("Unable to allocate memory for node");
    //    }
    //    let val = Node { left: l, right: r,  key: k, size: 0};
    //    unsafe { mem::overwrite(tmp, val); }
    //}

    fn insert_at(&mut self, key: uint, val: T, alloc: LightNodeAlloc<T>) {
        match self {
            &Node(_, _, k) if k == key => fail!("Already have a value with this key."),
            &Leaf(k, _) if k == key => fail!("Already have a value with this key."),
            &Leaf(k, ref v) if k != key => {
                let new_leaf = LightNode::init_leaf(alloc, key, val);
                let old_leaf = LightNode::init_leaf(alloc, k, v.clone());
                let new_node = if k > key {
                    Node(new_leaf, old_leaf, key)
                } else {
                    Node(old_leaf, new_leaf, k)
                };
                // We just overwrite ourself with the new value.
                unsafe { ptr::write(self as *mut LightNode<T>, new_node); }
            },
            // NOTE for some reason rust doesn't like deref_mut!(l).insert_at(...)?
            &Node(l, _, k) if k > key => { let x = deref_mut!(l); x.insert_at(key, val, alloc) },
            &Node(_, r, k) if k < key => { let x = deref_mut!(r); x.insert_at(key, val, alloc) },
            _ => unreachable!(),
        }
        // TODO I should put in a self.balance() routine.
    }

    fn search<'a>(&'a self, key: uint) -> Option<(uint, T)> {
        match *self {
            Leaf(k, ref v) if k >= key => Some((k, v.clone())),
            Leaf(k, _)     if k <  key => None,
            Node(_, r, k)  if k >  key => { let x = deref!(r); x.search(key) },
            Node(l, _, k)  if k <= key => { let x = deref!(l); x.search(key) },
            _ => unreachable!(),
        }
    }
}

pub struct LightMap<T: Clone> {
    pub root : *mut LightNode<T>,
    pub len  : uint,
    pub alloc: LightNodeAlloc<T>,
}

impl<T: Clone> LightMap<T> {
    #[inline]
    pub fn new(allocator: LightNodeAlloc<T>) -> LightMap<T> { LightMap { root : 0 as *mut LightNode<T>, len : 0, alloc : allocator } }
    pub fn add(&mut self, k: uint, v: T) {
        let cur = self.find(k);
        match cur {
            None => {},
            Some(_) => {
                dbg!(debug::MM, "Attempted to add a already present element to a LightMap");
                return;
            }
        }
        if self.root.is_null() {
            self.root = LightNode::init_leaf(self.alloc, k, v);
        } else {
            let x = deref_mut!(self.root);
            x.insert_at(k, v, self.alloc);
        }
        self.len += 1;
    }

    pub fn find<'a>(&'a self, key: uint) -> Option<T> {
        match self.find_smallest(key) {
            Some((k, ref v)) if k == key => Some(v.clone()),
            Some((k, _))     if k != key => None,
            _ => None,
        }
    }

    pub fn find_smallest<'a>(&'a self, key: uint) -> Option<(uint, T)> {
        if self.root.is_null() {
            None
        } else {
            let x = deref!(self.root);
            x.search(key)
        }
    }

    pub fn len(&self) -> uint { self.len }
}

