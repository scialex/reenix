//! A Least Recently Used cache.

use std::default::Default;
use std::{fmt, ops};
use std::mem::{size_of, transmute};
use std::iter::{self, IntoIterator, FromIterator};
use std::collections::BTreeMap;
use key_ref::*;
use list_node::ListNode;
use mm::alloc::Allocation;

/// This is a LRU cache implemented by a TreeMap referencing nodes in a linked list.
///
/// A node is considered used when anything finds it. A node is not generally considered to be used
/// by iterating over it. We also provide methods that lets one get a node without using it and to
/// assign nodes to particular places in the cache (see touch_value and curse_value).
pub struct LruCache<K, V> {
    map: BTreeMap<KeyRef<K>, Box<ListNode<K, V>>>,
    ptr: Box<ListNode<K, V>>,
}

/// This can be called to make sure there is an allocator able to efficiently hold be used by a lru
/// cache on the given key-value types.
pub fn request_lru_cache_allocator<K, V>(n: &'static str) {
    use mm::alloc::request_slab_allocator;
    request_slab_allocator(n, size_of::<ListNode<K, V>>() as u32);
}

/// An iterator going from least to most recently used that yields mutable references.
pub struct LTMMutEntries<'a, K: 'a, V: 'a> { cur: &'a mut ListNode<K, V>, }
impl<'a, K: Ord, V> Iterator for LTMMutEntries<'a, K, V> {
    type Item = (&'a K, &'a mut V);
    fn next(&mut self) -> Option<(&'a K, &'a mut V)> {
        if self.cur.is_start() { None } else {
            // For some reason it cannot prove that the lifetimes are the same for either of these...
            let out = unsafe { transmute(Some(self.cur.entry_mut())) };
            self.cur = unsafe { transmute(self.cur.prev_mut()) };
            out
        }
    }
}

/// An iterator going from most to least recently used that yields mutable references.
pub struct MTLMutEntries<'a, K: 'a, V: 'a> { cur: &'a mut ListNode<K, V>, }
impl<'a, K: Ord, V> Iterator for MTLMutEntries<'a, K, V> {
    type Item = (&'a K, &'a mut V);
    fn next(&mut self) -> Option<(&'a K, &'a mut V)> {
        if self.cur.is_start() { None } else {
            // For some reason it cannot prove that the lifetimes are the same for either of these...
            let out = unsafe { transmute(Some(self.cur.entry_mut())) };
            self.cur = unsafe { transmute(self.cur.next_mut()) };
            out
        }
    }
}

/// An iterator going from least to most recently used that yields immutable references.
pub struct LTMEntries<'a, K: 'a, V: 'a> { cur: &'a ListNode<K, V>, }
impl<'a, K: Ord, V> Iterator for LTMEntries<'a, K, V> {
    type Item = (&'a K, &'a V);
    fn next(&mut self) -> Option<(&'a K, &'a V)> {
        if self.cur.is_start() { None } else {
            let out = Some((self.cur.key(), self.cur.value()));
            self.cur = self.cur.prev();
            out
        }
    }
}

/// An iterator going from most to least recently used that yields immutable references.
pub struct MTLEntries<'a, K: 'a, V: 'a> { cur: &'a ListNode<K, V>, }
impl<'a, K: Ord, V> Iterator for MTLEntries<'a, K, V> {
    type Item = (&'a K, &'a V);
    fn next(&mut self) -> Option<(&'a K, &'a V)> {
        if self.cur.is_start() { None } else {
            let out = Some((self.cur.key(), self.cur.value()));
            self.cur = self.cur.next();
            out
        }
    }
}


/// A wrapper type that is a value contained in a LruCache that can be removed from it.
pub struct ModifiableEntry<'a, K: 'a, V: 'a> { val: (&'a K, &'a V), cache: &'a mut LruCache<K, V>, }
impl<'a, K: Ord, V> ModifiableEntry<'a, K, V> {
    /// Take the current entry out of the LruCache.
    pub fn remove_entry(self) -> (K, V) { self.cache.pop_entry(self.val.0).expect("entry is gone, concurrent RW on LRU cache") }
}
impl<'a, K: Ord, V> ops::Deref for ModifiableEntry<'a, K, V> {
    type Target = (&'a K, &'a V);
    fn deref<'b>(&'b self) -> &'b (&'a K, &'a V) { &self.val }
}

/// An iterator going from most to least recently used that yields removable references.
pub struct MTLModifyEntries<'a, K: 'a, V: 'a> { cur: &'a ListNode<K, V>, cache: &'a mut LruCache<K, V>, }
impl<'a, K: Ord, V> Iterator for MTLModifyEntries<'a, K, V> {
    type Item = ModifiableEntry<'a, K, V>;
    fn next(&mut self) -> Option<ModifiableEntry<'a, K, V>> {
        if self.cur.is_start() { None } else {
            let out = unsafe { transmute(Some(ModifiableEntry { val: self.cur.entry(), cache: self.cache })) };
            self.cur = self.cur.next();
            out
        }
    }
}

/// An iterator going from least to most recently used that yields removable references.
pub struct LTMModifyEntries<'a, K: 'a, V: 'a> { cur: &'a ListNode<K, V>, cache: &'a mut LruCache<K, V>, }
impl<'a, K: Ord, V> Iterator for LTMModifyEntries<'a, K, V> {
    type Item = ModifiableEntry<'a, K, V>;
    fn next(&mut self) -> Option<ModifiableEntry<'a, K, V>> {
        if self.cur.is_start() { None } else {
            let out = unsafe { transmute(Some(ModifiableEntry { val: self.cur.entry(), cache: self.cache })) };
            self.cur = self.cur.prev();
            out
        }
    }
}

pub type ModifyEntries<'a, K, V> = LTMModifyEntries<'a, K, V>;
pub type LTMRemoveEntries<'a, K, V> = iter::Map<LTMModifyEntries<'a, K, V>, fn(ModifiableEntry<'a, K, V>) -> (K, V)>;
pub type MTLRemoveEntries<'a, K, V> = iter::Map<MTLModifyEntries<'a, K, V>, fn(ModifiableEntry<'a, K, V>) -> (K, V)>;
pub type RemoveEntries<'a, K, V> = LTMRemoveEntries<'a, K, V>;
pub type Entries<'a, K, V> = LTMEntries<'a, K, V>;
pub type MutEntries<'a, K, V> = LTMMutEntries<'a, K, V>;
pub type LTMKeys<'a, K, V> = iter::Map<LTMEntries<'a, K, V>, fn((&'a K, &'a V)) -> &'a K>;
pub type MTLKeys<'a, K, V> = iter::Map<MTLEntries<'a, K, V>, fn((&'a K, &'a V)) -> &'a K>;
pub type Keys<'a, K, V> = iter::Map<Entries<'a, K, V>, fn((&'a K, &'a V)) -> &'a K>;
pub type LTMValues<'a, K, V> = iter::Map<LTMEntries<'a, K, V>, fn((&'a K, &'a V)) -> &'a V>;
pub type MTLValues<'a, K, V> = iter::Map<MTLEntries<'a, K, V>, fn((&'a K, &'a V)) -> &'a V>;
pub type Values<'a, K, V> = iter::Map<Entries<'a, K, V>, fn((&'a K, &'a V)) -> &'a V>;

impl<K: Ord, V> LruCache<K, V> {
    pub fn new() -> Allocation<LruCache<K, V>> {
        Ok(LruCache {
            map: try!(alloc!(try BTreeMap::new())),
            ptr: try!(alloc!(try ListNode::initial())),
        })
    }

    /// Remove items until we are at most 'len' items long. We will remove the least recently used
    /// items first.
    pub fn trim_to(&mut self, len: usize) {
        while self.len() > len { self.pop_lru(); }
    }

    /// Remove the 'cnt' least recently used items from this cache.
    pub fn trim_off(&mut self, cnt: usize) {
        for _ in 0..cnt { self.pop_lru(); }
    }

    /// Remove the most recently used item and return it, along with it's key.
    pub fn pop_mru<'a>(&'a mut self) -> Option<(K, V)> {
        let mru : &'a ListNode<K, V> = unsafe { transmute(self.ptr.next()) };
        if mru.is_start() { return None; }
        self.pop_entry(mru.key())
    }

    /// Remove the least recently used item and return it, along with its key.
    pub fn pop_lru<'a>(&'a mut self) -> Option<(K, V)> {
        let lru : &'a ListNode<K, V> = unsafe { transmute(self.ptr.prev()) };
        if lru.is_start() { return None; }
        self.pop_entry(lru.key())
    }

    /// Return the number of elements in the map.
    pub fn len(&self) -> usize { self.map.len() }
    /// Return true if the map contains no elements.
    pub fn is_empty(&self) -> bool { self.len() == 0 }
    /// Returns true if the map contains a value for the specified key.
    /// This does not affect the ranking of the value at that key. For that see touch_value
    pub fn contains_key(&self, key: &K) -> bool { self.map.contains_key(&KeyRef::new(key)) }

    /// Returns a reference to the value corresponding to the key.
    /// This also marks that key as being the most recently used.
    pub fn get<'a>(&'a self, key: &K) -> Option<&'a V> {
        if let Some(cur) = self.map.get(&KeyRef::new(key)) {
            self.make_mru(&**cur);
            Some(cur.value())
        } else {
            None
        }
    }

    /// Returns a mutable reference to the value corresponding to the key.
    /// This also marks that key as being the most recently used.
    pub fn get_mut<'a>(&'a mut self, key: &K) -> Option<&'a mut V> {
        if let Some(cur) = self.map.get_mut(&KeyRef::new(key)) {
            (&**cur).remove_self();
            (&**cur).insert_after(&*self.ptr);
            Some(cur.value_mut())
        } else {
            None
        }
    }

    /// Returns true if there is a value with that key and makes it the most recently used value.
    pub fn touch_value(&self, key: &K) -> bool { self.get(key).is_some() }

    /// Returns true if there is a value with that key and makes it the least recently used value.
    pub fn curse_value(&self, key: &K) -> bool {
        if let Some(prev) = self.map.get(&KeyRef::new(key)) {
            prev.remove_self();
            prev.insert_after(self.ptr.prev());
            true
        } else { false }
    }

    /// Returns a reference to the value corresponding to the key.
    /// This does not count as a use of the value by the LRU cache and does not affect the values
    /// position in it.
    pub fn get_unused<'a>(&'a self, key: &K) -> Option<&'a V> { self.map.get(&KeyRef::new(key)).map(|x| { x.value() }) }

    /// Returns a mutable reference to the value corresponding to the key.
    /// This does not count as a use of the value by the LRU cache and does not affect the values
    /// position in it.
    pub fn get_unused_mut<'a>(&'a mut self, key: &K) -> Option<&'a mut V> {
        self.map.get_mut(&KeyRef::new(key)).map(|x| { x.value_mut() })
    }

    /// Inserts a key-value pair from the map. If the key already had a value present in the map,
    /// that value is returned. Otherwise, None is returned.
    /// The inserted value is considered to be the most recently used value in the cache.
    pub fn insert(&mut self, key: K, val: V) -> Option<V> {
        let ent = ListNode::new(key, val);
        let kr = KeyRef::new(ent.key());
        self.make_mru(&*ent);
        if let Some(prev) = self.map.insert(kr, ent) {
            prev.remove_self();
            Some(prev.take())
        } else {
            None
        }
    }

    /// Inserts a key-value pair into the map. An existing value for a key is replaced by the new
    /// value. Returns true if the key did not already exist in the map.
    #[inline]
    pub fn swap(&mut self, key: K, val: V) -> Option<V> { self.insert(key, val) }

    /// Removes a key from the map, returning the value at the key if the key was previously in the
    /// map.
    pub fn pop(&mut self, key: &K) -> Option<V> { self.pop_entry(key).map(|(_, v)| v) }

    fn pop_entry(&mut self, key: &K) -> Option<(K, V)> {
        if let Some(prev) = self.map.remove(&KeyRef::new(key)) {
            prev.remove_self();
            Some(prev.take_full())
        } else {
            None
        }
    }

    /// Removes a key-value pair from the map. Returns true if the key was present in the map.
    pub fn remove(&mut self, key: &K) -> bool { self.pop(key).is_some() }

    /// Sets the given ListNode as the most recently used.
    #[inline]
    fn make_mru(&self, ent: &ListNode<K, V>) {
        ent.remove_self();
        ent.insert_after(&*self.ptr);
    }

    fn key<'a>((k,_):(&'a K, &'a V)) -> &'a K { k }
    fn val<'a>((_,v):(&'a K, &'a V)) -> &'a V { v }
    /// Gets a lazy iterator over the keys in the map in an undefined order.
    /// Note this does not count as a use of the keys for the purposes of the cache.
    pub fn keys<'a>(&'a self) -> Keys<'a, K, V> {
        self.iter().map(LruCache::key)
    }

    /// Gets a lazy iterator over the keys in the map in an least to most recently used order.
    /// Note this does not count as a use of the keys for the purposes of the cache.
    pub fn keys_least<'a>(&'a self) -> LTMKeys<'a, K, V> {
        self.iter_least().map(LruCache::key)
    }

    /// Gets a lazy iterator over the keys in the map in an most to least recently used order.
    /// Note this does not count as a use of the keys for the purposes of the cache.
    pub fn keys_most<'a>(&'a self) -> MTLKeys<'a, K, V> {
        self.iter_most().map(LruCache::key)
    }

    /// Gets a lazy iterator over the values in the map in an undefined order.
    /// Note this does not count as a use of the values for the purposes of the cache.
    pub fn values<'a>(&'a self) -> Values<'a, K, V> {
        self.iter().map(LruCache::val)
    }

    /// Gets a lazy iterator over the values in the map in an least to most recently used order.
    /// Note this does not count as a use of the values for the purposes of the cache.
    pub fn values_least<'a>(&'a self) -> LTMValues<'a, K, V> {
        self.iter_least().map(LruCache::val)
    }

    /// Gets a lazy iterator over the values in the map in an most to least recently used order.
    /// Note this does not count as a use of the values for the purposes of the cache.
    pub fn values_most<'a>(&'a self) -> MTLValues<'a, K, V> {
        self.iter_most().map(LruCache::val)
    }

    /// An iterator of the keys and values in the iterator in an undefined order.
    /// Note this does not count as a use of the values for the purposes of the LRU cache.
    pub fn iter<'a>(&'a self) -> Entries<'a, K, V> { self.iter_least() }

    /// An iterator of the keys and mutable values in the iterator in an undefined order.
    /// Note this does not count as a use of the values for the purposes of the LRU cache.
    pub fn iter_mut<'a>(&'a mut self) -> MutEntries<'a, K, V> { self.iter_least_mut() }

    /// An iterator of the keys and values in the iterator in order from most to least recently
    /// used. Note this does not count as a use of the values for the purposes of the LRU cache.
    pub fn iter_most<'a>(&'a self) -> MTLEntries<'a, K, V> { MTLEntries { cur: self.ptr.next(), } }

    /// An iterator of the keys and mutable values in the iterator in order from most to least recently
    /// used. Note this does not count as a use of the values for the purposes of the LRU cache.
    pub fn iter_most_mut<'a>(&'a mut self) -> MTLMutEntries<'a, K, V> { MTLMutEntries { cur: self.ptr.next_mut(), } }

    /// An iterator of the keys and values in the iterator in order from least to most recently
    /// used. Note this does not count as a use of the values for the purposes of the LRU cache.
    pub fn iter_least<'a>(&'a self) -> LTMEntries<'a, K, V> { LTMEntries { cur: self.ptr.prev(), } }

    /// An iterator of the keys and mutable values in the iterator in order from least to most recently
    /// used. Note this does not count as a use of the values for the purposes of the LRU cache.
    pub fn iter_least_mut<'a>(&'a mut self) -> LTMMutEntries<'a, K, V> { LTMMutEntries { cur: self.ptr.prev_mut(), } }

    /// Iterator that removes items from the cache as it goes.
    pub fn iter_remove<'a>(&'a mut self) -> RemoveEntries<'a, K, V> { self.iter_remove_least() }
    /// Iterator that removes items from the cache as it goes. Order is least to most recently used.
    fn do_remove<'a>(v: ModifiableEntry<'a, K, V>) -> (K, V) { v.remove_entry() }
    pub fn iter_remove_least<'a>(&'a mut self) -> LTMRemoveEntries<'a, K, V> { self.iter_modify_least().map(LruCache::do_remove) }
    /// Iterator that removes items from the cache as it goes. Order is most to least recently used.
    pub fn iter_remove_most<'a>(&'a mut self) -> MTLRemoveEntries<'a, K, V> { self.iter_modify_most().map(LruCache::do_remove) }

    /// Iterator where you can choose to remove certain entries from the LRU cache.
    pub fn iter_modify<'a>(&'a mut self) -> ModifyEntries<'a, K, V> { self.iter_modify_least() }
    /// Iterator where you can choose to remove certain entries from the LRU cache. Order is least
    /// to most recently used.
    pub fn iter_modify_least<'a>(&'a mut self) -> LTMModifyEntries<'a, K, V> {
        LTMModifyEntries { cur: self.ptr.prev(), cache: unsafe { (self as *mut LruCache<K, V>).as_mut().expect("") } }
    }
    /// Iterator where you can choose to remove certain entries from the LRU cache. Order is most
    /// to least recently used.
    pub fn iter_modify_most<'a>(&'a mut self) -> MTLModifyEntries<'a, K, V> {
        // NOTE There is no way to express in rust that self.ptr is immutable even if we have a
        // mutable reference so this is safe.
        MTLModifyEntries { cur: self.ptr.next(), cache: unsafe { (self as *mut LruCache<K, V>).as_mut().expect("") } }
    }
}

impl<'b, K: Ord + 'b, V> ops::IndexMut<&'b K> for LruCache<K, V> {
    #[inline]
    fn index_mut<'a>(&'a mut self, i: &'b K) -> &'a mut V { self.get_mut(i).expect("no entry found in lru_cache") }
}

impl<'b, K: Ord + 'b, V> ops::Index<&'b K> for LruCache<K, V> {
    type Output = V;
    #[inline]
    fn index<'a>(&'a self, i: &'b K) -> &'a V { self.get(i).expect("no entry found in lru_cache") }
}

impl<K: Clone + Ord, V: Clone> Clone for LruCache<K, V> {
    fn clone(&self) -> LruCache<K, V> {
        let mut nc = match LruCache::new() { Ok(v) => v, Err(_) => { panic!("Unable to clone an lru cache"); } };
        for (k, v) in self.iter_least() { nc.insert(k.clone(), v.clone()); }
        nc
    }
}

impl<K: Ord, V> Extend<(K, V)> for LruCache<K, V> {
    fn extend<T: IntoIterator<Item = (K, V)>>(&mut self, iter: T) {
        for (k, v) in iter { self.insert(k, v); }
    }
}

impl<K: Ord, V> FromIterator<(K, V)> for LruCache<K, V> {
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> LruCache<K, V> {
        let mut nc = match LruCache::new() { Ok(v) => v, Err(_) => { panic!("Unable to make an lru cache"); } };
        nc.extend(iter);
        nc
    }
}

impl<K: Ord, V> Default for LruCache<K, V> {
    fn default() -> LruCache<K, V> {
        LruCache::new().unwrap_or_else(|_| { panic!("Unable to create lru cache");})
    }
}

impl<K: fmt::Debug + Ord, V: fmt::Debug> fmt::Debug for LruCache<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        try!(write!(f, "LruCache (len: {}) {{", self.len()));
        let mut first = true;
        for (k, v) in self.iter_least() {
            if first { first = false; } else { try!(write!(f, ",")); }
            try!(write!(f, " {:?}: {:?}", k, v));
        }
        write!(f, " }}")
    }
}
