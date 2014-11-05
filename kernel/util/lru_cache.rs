use alloc::boxed::*;
use collections::*;
use core::cell::Cell;
use core::fmt;
use core::iter;
use core::default::Default;
use mm::alloc::Allocation;
use core::mem::{transmute_copy, transmute, size_of};
use core::prelude::*;
use core::ptr::*;

pub fn init_stage1() {}
pub fn init_stage2() {}
pub fn init_stage3() {}

/// A struct used as the key for our map so we can get the key back out without trouble.
struct KeyRef<K> { k: *const K, }
impl<K> KeyRef<K> {
    pub fn new(v: &K) -> KeyRef<K> { unsafe { KeyRef { k: transmute(v), } } }
    pub fn as_ref<'a>(&'a self) -> &'a K { unsafe { self.k.as_ref().expect("LRU-cache key ref should never be null") } }
}

impl<K: PartialEq>  PartialEq  for KeyRef<K> {
    fn eq(&self, o: &KeyRef<K>)  -> bool { self.as_ref().eq( o.as_ref()) }
}
impl<K: PartialOrd> PartialOrd for KeyRef<K> {
    fn partial_cmp(&self, o: &KeyRef<K>) -> Option<Ordering> { self.as_ref().partial_cmp(o.as_ref()) }
}
impl<K: Eq>  Eq  for KeyRef<K> { }
impl<K: Ord> Ord for KeyRef<K> {
    fn cmp(&self, o: &KeyRef<K>) -> Ordering { self.as_ref().cmp(o.as_ref()) }
}

/// This is a LRU cache implemented by a TreeMap referencing nodes in a linked list.
///
/// A node is considered used when anything finds it. A node is not generally considered to be used
/// by iterating over it. We also provide methods that lets one get a node without using it and to
/// assign nodes to particular places in the cache (see touch_value and curse_value).
pub struct LruCache<K, V> {
    map: TreeMap<KeyRef<K>, Box<LruEntry<K, V>>>,
    ptr: Box<LruEntry<K, V>>,
}

struct LruEntry<K, V> {
    val : Option<(K, V)>,
    next: Cell<*mut LruEntry<K, V>>,
    prev: Cell<*mut LruEntry<K, V>>,
}

/// This can be called to make sure there is an allocator able to efficiently hold be used by a lru
/// cache on the given key-value types.
pub fn request_lru_cache_allocator<K, V>(n: &'static str) {
    use mm::alloc::request_slab_allocator;
    request_slab_allocator(n, size_of::<LruEntry<K, V>>() as u32);
}

impl<K: Ord, V> LruEntry<K, V> {
    pub fn new(k: K, v: V) -> Box<LruEntry<K, V>> {
        let mut res = LruEntry::initial();
        res.val = Some((k, v));
        res
    }

    pub fn initial() -> Box<LruEntry<K, V>> {
        let res = box LruEntry {
            val : None,
            next: Cell::new(0 as *mut LruEntry<K, V>),
            prev: Cell::new(0 as *mut LruEntry<K, V>),
        };
        unsafe {
            res.next.set(transmute_copy(&res));
            res.prev.set(transmute_copy(&res));
        }
        res
    }
    /// Take this entry out of the list (if it is in one). This needs to take a non-mutable pointer
    /// so we can do this even in cases where we are doing (i.e. find).
    pub fn remove_self(&self) {
        self.next().prev.set(self.prev.get());
        self.prev().next.set(self.next.get());
        unsafe {
            self.next.set(transmute(self));
            self.prev.set(transmute(self));
        }
    }
    /// Marks this one as being used less recently than o, but more recently than o.next()
    pub fn insert_after(&self, o: &LruEntry<K, V>) {
        self.next.set(o.next.get());
        unsafe {
            self.prev.set(transmute(&*o));
            self.prev().next.set(transmute(&*self));
            self.next().prev.set(transmute(&*self));
        }
    }
    /// Get the one used less recently then this one.
    pub fn next_mut<'a>(&'a mut self) -> &'a mut LruEntry<K, V> {
        unsafe { self.next.get().as_mut().expect("Bad LRU cache state, no next pointer") }
    }
    /// Get the one that was used more recently than this one.
    pub fn prev_mut<'a>(&'a mut self) -> &'a mut LruEntry<K, V> {
        unsafe { self.prev.get().as_mut().expect("Bad LRU cache state, no prev pointer") }
    }
    pub fn next<'a>(&'a self) -> &'a LruEntry<K, V> {
        unsafe { self.next.get().as_ref().expect("Bad LRU cache state, no next pointer") }
    }
    pub fn prev<'a>(&'a self) -> &'a LruEntry<K, V> {
        unsafe { self.prev.get().as_ref().expect("Bad LRU cache state, no prev pointer") }
    }

    pub fn is_start(&self) -> bool { self.val.is_none() }

    /// Return a ref to the value. Panic if we are not a list element.
    pub fn value<'a>(&'a self) -> &'a V { self.entry().1 }

    /// Return a ref mut to the value. Panic if we are not a list element.
    pub fn value_mut<'a>(&'a mut self) -> &'a mut V { self.entry_mut().1 }

    /// Return a ref to the key. Panic if we are not a list element.
    pub fn key<'a>(&'a self) -> &'a K { self.entry().0 }

    /// Take the value from the element. This destroys the element. The element cannot be in any
    /// cache when this is called.
    pub fn take(self) -> V {
        // Ensure we are not in any list.
        assert!(self.prev.get() == self.next.get() && self.prev().prev.get() == self.prev.get());
        self.take_full().1
    }

    /// Take the value and key from the element. This destroys the element. The element cannot be
    /// in any cache when this is called.
    pub fn take_full(mut self) -> (K, V) {
        assert!(self.prev.get() == self.next.get() && self.prev().prev.get() == self.prev.get());
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

/// An iterator going from least to most recently used that yields mutable references.
pub struct LTMMutEntries<'a, K: 'a, V: 'a> { cur: &'a mut LruEntry<K, V>, }
impl<'a, K: Ord, V> Iterator<(&'a K, &'a mut V)> for LTMMutEntries<'a, K, V> {
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
pub struct MTLMutEntries<'a, K: 'a, V: 'a> { cur: &'a mut LruEntry<K, V>, }
impl<'a, K: Ord, V> Iterator<(&'a K, &'a mut V)> for MTLMutEntries<'a, K, V> {
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
pub struct LTMEntries<'a, K: 'a, V: 'a> { cur: &'a LruEntry<K, V>, }
impl<'a, K: Ord, V> Iterator<(&'a K, &'a V)> for LTMEntries<'a, K, V> {
    fn next(&mut self) -> Option<(&'a K, &'a V)> {
        if self.cur.is_start() { None } else {
            let out = Some((self.cur.key(), self.cur.value()));
            self.cur = self.cur.prev();
            out
        }
    }
}

/// An iterator going from most to least recently used that yields immutable references.
pub struct MTLEntries<'a, K: 'a, V: 'a> { cur: &'a LruEntry<K, V>, }
impl<'a, K: Ord, V> Iterator<(&'a K, &'a V)> for MTLEntries<'a, K, V> {
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
impl<'a, K: Ord, V> Deref<(&'a K, &'a V)> for ModifiableEntry<'a, K, V> {
    fn deref<'b>(&'b self) -> &'b (&'a K, &'a V) { &self.val }
}

/// An iterator going from most to least recently used that yields removable references.
pub struct MTLModifyEntries<'a, K: 'a, V: 'a> { cur: &'a LruEntry<K, V>, cache: &'a mut LruCache<K, V>, }
impl<'a, K: Ord, V> Iterator<ModifiableEntry<'a, K, V>> for MTLModifyEntries<'a, K, V> {
    fn next(&mut self) -> Option<ModifiableEntry<'a, K, V>> {
        if self.cur.is_start() { None } else {
            let out = unsafe { transmute(Some(ModifiableEntry { val: self.cur.entry(), cache: self.cache })) };
            self.cur = self.cur.next();
            out
        }
    }
}

/// An iterator going from least to most recently used that yields removable references.
pub struct LTMModifyEntries<'a, K: 'a, V: 'a> { cur: &'a LruEntry<K, V>, cache: &'a mut LruCache<K, V>, }
impl<'a, K: Ord, V> Iterator<ModifiableEntry<'a, K, V>> for LTMModifyEntries<'a, K, V> {
    fn next(&mut self) -> Option<ModifiableEntry<'a, K, V>> {
        if self.cur.is_start() { None } else {
            let out = unsafe { transmute(Some(ModifiableEntry { val: self.cur.entry(), cache: self.cache })) };
            self.cur = self.cur.prev();
            out
        }
    }
}

pub type ModifyEntries<'a, K, V> = LTMModifyEntries<'a, K, V>;
pub type LTMRemoveEntries<'a, K, V> = iter::Map<'static, ModifiableEntry<'a, K, V>, (K, V), LTMModifyEntries<'a, K, V>>;
pub type MTLRemoveEntries<'a, K, V> = iter::Map<'static, ModifiableEntry<'a, K, V>, (K, V), MTLModifyEntries<'a, K, V>>;
pub type RemoveEntries<'a, K, V> = LTMRemoveEntries<'a, K, V>;
pub type Entries<'a, K, V> = LTMEntries<'a, K, V>;
pub type MutEntries<'a, K, V> = LTMMutEntries<'a, K, V>;
pub type LTMKeys<'a, K, V> = iter::Map<'static, (&'a K, &'a V), &'a K, LTMEntries<'a, K, V>>;
pub type MTLKeys<'a, K, V> = iter::Map<'static, (&'a K, &'a V), &'a K, MTLEntries<'a, K, V>>;
pub type Keys<'a, K, V> = iter::Map<'static, (&'a K, &'a V), &'a K, Entries<'a, K, V>>;
pub type LTMValues<'a, K, V> = iter::Map<'static, (&'a K, &'a V), &'a V, LTMEntries<'a, K, V>>;
pub type MTLValues<'a, K, V> = iter::Map<'static, (&'a K, &'a V), &'a V, MTLEntries<'a, K, V>>;
pub type Values<'a, K, V> = iter::Map<'static, (&'a K, &'a V), &'a V, Entries<'a, K, V>>;

impl<K: Ord, V> LruCache<K, V> {
    pub fn new() -> Allocation<LruCache<K, V>> {
        Ok(LruCache {
            map: try!(alloc!(try TreeMap::new())),
            ptr: try!(alloc!(try LruEntry::initial())),
        })
    }

    /// Remove items until we are at most 'len' items long. We will remove the least recently used
    /// items first.
    pub fn trim_to(&mut self, len: uint) {
        while self.len() > len { self.pop_lru(); }
    }

    /// Remove the 'cnt' least recently used items from this cache.
    pub fn trim_off(&mut self, cnt: uint) {
        for _ in range(0, cnt) { self.pop_lru(); }
    }

    /// Remove the most recently used item and return it, along with it's key.
    pub fn pop_mru<'a>(&'a mut self) -> Option<(K, V)> {
        let mru : &'a LruEntry<K, V> = unsafe { transmute(self.ptr.next()) };
        if mru.is_start() { return None; }
        self.pop_entry(mru.key())
    }

    /// Remove the least recently used item and return it, along with its key.
    pub fn pop_lru<'a>(&'a mut self) -> Option<(K, V)> {
        let lru : &'a LruEntry<K, V> = unsafe { transmute(self.ptr.prev()) };
        if lru.is_start() { return None; }
        self.pop_entry(lru.key())
    }

    /// Return the number of elements in the map.
    pub fn len(&self) -> uint { self.map.len() }
    /// Return true if the map contains no elements.
    pub fn is_empty(&self) -> bool { self.len() == 0 }
    /// Returns true if the map contains a value for the specified key.
    /// This does not affect the ranking of the value at that key. For that see touch_value
    pub fn contains_key(&self, key: &K) -> bool { self.map.contains_key(&KeyRef::new(key)) }

    /// Returns a reference to the value corresponding to the key.
    /// This also marks that key as being the most recently used.
    pub fn find<'a>(&'a self, key: &K) -> Option<&'a V> {
        if let Some(cur) = self.map.find(&KeyRef::new(key)) {
            self.make_mru(&**cur);
            Some(cur.value())
        } else {
            None
        }
    }

    /// Returns a mutable reference to the value corresponding to the key.
    /// This also marks that key as being the most recently used.
    pub fn find_mut<'a>(&'a mut self, key: &K) -> Option<&'a mut V> {
        if let Some(cur) = self.map.find_mut(&KeyRef::new(key)) {
            (&**cur).remove_self();
            (&**cur).insert_after(&*self.ptr);
            Some(cur.value_mut())
        } else {
            None
        }
    }

    /// Returns true if there is a value with that key and makes it the most recently used value.
    pub fn touch_value(&self, key: &K) -> bool { self.find(key).is_some() }

    /// Returns true if there is a value with that key and makes it the least recently used value.
    pub fn curse_value(&self, key: &K) -> bool {
        if let Some(prev) = self.map.find(&KeyRef::new(key)) {
            prev.remove_self();
            prev.insert_after(self.ptr.prev());
            true
        } else { false }
    }

    /// Returns a reference to the value corresponding to the key.
    /// This does not count as a use of the value by the LRU cache and does not affect the values
    /// position in it.
    pub fn find_unused<'a>(&'a self, key: &K) -> Option<&'a V> { self.map.find(&KeyRef::new(key)).map(|x| { x.value() }) }

    /// Returns a mutable reference to the value corresponding to the key.
    /// This does not count as a use of the value by the LRU cache and does not affect the values
    /// position in it.
    pub fn find_unused_mut<'a>(&'a mut self, key: &K) -> Option<&'a mut V> {
        self.map.find_mut(&KeyRef::new(key)).map(|x| { x.value_mut() })
    }

    /// Inserts a key-value pair from the map. If the key already had a value present in the map,
    /// that value is returned. Otherwise, None is returned.
    /// The inserted value is considered to be the most recently used value in the cache.
    pub fn swap(&mut self, key: K, val: V) -> Option<V> {
        let ent = LruEntry::new(key, val);
        let kr = KeyRef::new(ent.key());
        self.make_mru(&*ent);
        if let Some(prev) = self.map.swap(kr, ent) {
            prev.remove_self();
            Some(prev.take())
        } else {
            None
        }
    }

    /// Inserts a key-value pair into the map. An existing value for a key is replaced by the new
    /// value. Returns true if the key did not already exist in the map.
    pub fn insert(&mut self, key: K, val: V) -> bool { self.swap(key, val).is_none() }

    /// Removes a key from the map, returning the value at the key if the key was previously in the
    /// map.
    pub fn pop(&mut self, key: &K) -> Option<V> { self.pop_entry(key).map(|(_, v)| v) }

    fn pop_entry(&mut self, key: &K) -> Option<(K, V)> {
        if let Some(prev) = self.map.pop(&KeyRef::new(key)) {
            prev.remove_self();
            Some(prev.take_full())
        } else {
            None
        }
    }

    /// Removes a key-value pair from the map. Returns true if the key was present in the map.
    pub fn remove(&mut self, key: &K) -> bool { self.pop(key).is_some() }

    /// Sets the given LruEntry as the most recently used.
    #[inline]
    fn make_mru(&self, ent: &LruEntry<K, V>) {
        ent.remove_self();
        ent.insert_after(&*self.ptr);
    }

    /// Gets a lazy iterator over the keys in the map in an undefined order.
    /// Note this does not count as a use of the keys for the purposes of the cache.
    pub fn keys<'a>(&'a self) -> Keys<'a, K, V> { self.iter().map(|(k, _)| k) }

    /// Gets a lazy iterator over the keys in the map in an least to most recently used order.
    /// Note this does not count as a use of the keys for the purposes of the cache.
    pub fn keys_least<'a>(&'a self) -> LTMKeys<'a, K, V> { self.iter_least().map(|(k, _)| k) }

    /// Gets a lazy iterator over the keys in the map in an most to least recently used order.
    /// Note this does not count as a use of the keys for the purposes of the cache.
    pub fn keys_most<'a>(&'a self) -> MTLKeys<'a, K, V> { self.iter_most().map(|(k, _)| k) }

    /// Gets a lazy iterator over the values in the map in an undefined order.
    /// Note this does not count as a use of the values for the purposes of the cache.
    pub fn values<'a>(&'a self) -> Values<'a, K, V> { self.iter().map(|(_k, v)| v) }

    /// Gets a lazy iterator over the values in the map in an least to most recently used order.
    /// Note this does not count as a use of the values for the purposes of the cache.
    pub fn values_least<'a>(&'a self) -> LTMValues<'a, K, V> { self.iter_least().map(|(_k, v)| v) }

    /// Gets a lazy iterator over the values in the map in an most to least recently used order.
    /// Note this does not count as a use of the values for the purposes of the cache.
    pub fn values_most<'a>(&'a self) -> MTLValues<'a, K, V> { self.iter_most().map(|(_k, v)| v) }

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
    pub fn iter_remove_least<'a>(&'a mut self) -> LTMRemoveEntries<'a, K, V> { self.iter_modify_least().map(|x| x.remove_entry() ) }
    /// Iterator that removes items from the cache as it goes. Order is most to least recently used.
    pub fn iter_remove_most<'a>(&'a mut self) -> MTLRemoveEntries<'a, K, V> { self.iter_modify_most().map(|x| x.remove_entry() ) }

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

impl<K: Ord, V> IndexMut<K, V> for LruCache<K, V> {
    #[inline]
    fn index_mut<'a>(&'a mut self, i: &K) -> &'a mut V { self.find_mut(i).expect("no entry found in lru_cache") }
}

impl<K: Ord, V> Index<K, V> for LruCache<K, V> {
    #[inline]
    fn index<'a>(&'a self, i: &K) -> &'a V { self.find(i).expect("no entry found in lru_cache") }
}

impl<K: Clone + Ord, V: Clone> Clone for LruCache<K, V> {
    fn clone(&self) -> LruCache<K, V> {
        let mut nc = LruCache::new().unwrap();
        for (k, v) in self.iter_least() { nc.insert(k.clone(), v.clone()); }
        nc
    }
}

impl<K: Ord, V> Extendable<(K, V)> for LruCache<K, V> {
    fn extend<T: Iterator<(K, V)>>(&mut self, mut iter: T) {
        for (k, v) in iter { self.insert(k, v); }
    }
}

impl<K: Ord, V> FromIterator<(K, V)> for LruCache<K, V> {
    fn from_iter<T: Iterator<(K, V)>>(iter: T) -> LruCache<K, V> {
        let mut nc = LruCache::new().unwrap();
        nc.extend(iter);
        nc
    }
}

impl<K: Ord, V> Default for LruCache<K, V> { fn default() -> LruCache<K, V> { LruCache::new().unwrap() } }

impl<K: fmt::Show + Ord, V: fmt::Show> fmt::Show for LruCache<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        try!(write!(f, "LruCache (len: {}) {{", self.len()));
        let mut first = true;
        for (k, v) in self.iter_least() {
            if first { first = false; } else { try!(write!(f, ",")); }
            try!(write!(f, " {}: {}", k, v));
        }
        write!(f, " }}")
    }
}
