
//! An LRU cache where we can 'pin' items.

use core::mem::size_of;
use core::cell::*;
use collections::*;
use base::make::*;
use base::errno::Errno;
use core::atomic::*;
use core::prelude::*;
use lru_cache::*;
use key_ref::*;
use alloc::boxed::*;
use core::mem::transmute;
use core::fmt;
use mm::{Allocation, AllocError};
use cacheable::*;

/// An item in a pinnable cache.
struct CacheItem<K, V> {
    /// The key/value pair in this cache.
    val : (K, V),
    /// The pin count for the value this is holding.
    pcnt: AtomicUint,
}

impl<'a, K, V: 'a> Make<K> for Allocation<Box<CacheItem<K, V>>> where K: Ord, V: RefMake<'a, K> + Cacheable {
    fn make(k: K) -> Allocation<Box<CacheItem<K, V>>> {
        let val = RefMake::make_from(&k);
        CacheItem::new(k, val)
    }
}

impl<K: Ord, V: Cacheable> CacheItem<K, V> {
    pub fn new(key: K, val: V) -> Allocation<Box<CacheItem<K,V>>> {
        alloc!(try_box CacheItem { val: (key, val), pcnt: AtomicUint::new(1), })
    }
    /// increment the pincount
    pub fn pin(&self) {
        let old_val = self.pcnt.fetch_add(1, SeqCst);
        assert!(old_val != -1);
    }

    /// decrement the pincount and notify the cache if nessecary.
    pub fn unpin(&self, owner: &PinnableCache<K, V>) {
        let old_val = self.pcnt.fetch_sub(1, SeqCst);
        assert!(old_val != 0, "Unpin called on an already unpinned value!");
        if old_val == 1 {
            owner.notify_unpinned(self);
        }
    }

    /// Get the value associated with this item
    pub fn value<'b>(&'b self) -> &'b V { &self.val.1 }
    /// Get the key associated with this item
    pub fn key<'b>(&'b self)   -> &'b K { &self.val.0 }

    /// Gets the pin count of this item.
    #[inline]
    pub fn pin_count(&self) -> uint { self.pcnt.load(SeqCst) }
    // TODO
}

impl<K: fmt::Show, V: fmt::Show> fmt::Show for CacheItem<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "{{ pinned: {}, val: {} }}", self.pcnt.load(SeqCst), self.val) }
}

impl<K, V> Deref<(K, V)> for CacheItem<K, V> {
    fn deref<'a>(&'a self) -> &'a (K, V) { &self.val }
}

impl<K: Ord, V:Cacheable> Cacheable for CacheItem<K, V> {
    #[inline]
    fn is_still_useful(&self) -> bool { self.value().is_still_useful() }
}

/// The states a value in the cache can have.
#[deriving(Eq, PartialEq)]
pub enum State {
    Pinned(uint),
    Unpinned,
    NotFound
}

/// The errors that can happen when we try to insert a value into the cache.
#[deriving(Show)]
pub enum InsertError {
    /// There is already a key with that value in the cache.
    KeyPresent,
    /// There was an error allocating memory.
    MemoryError(AllocError),
    /// Some other error occured, which might be described by the errno.
    SysError(Option<Errno>),
}

/// This can be called to make sure there is an allocator able to efficiently hold be used by a
/// pinnable cache on the given key-value types.
pub fn request_pinnable_cache_allocator<K, V>(n: &'static str) {
    use mm::alloc::request_slab_allocator;
    use lru_cache::request_lru_cache_allocator;
    request_slab_allocator(n, size_of::<CacheItem<K, V>>() as u32);
    request_lru_cache_allocator::<KeyRef<K>,Box<CacheItem<K, V>>>(n);
}

/// A pinnable cache is a cache where data can be in one of 2 states, either `pinned` where we are
/// referenced by something and cannot be removed, or `unpinned` where we are not currently being
/// used by anything and might be deleted at any time.
///
/// Although this only works if we gaurentee that multiple threads do not use this cache
/// concurrently. This shouldn't be a problem as long as we do not have Kernel Preemption.
pub struct PinnableCache<K: Ord, V> {
    /// The map of unpinned values, which are eligible for deletion if we start to run out of space.
    unpinned : UnsafeCell<LruCache<KeyRef<K>, Box<CacheItem<K, V>>>>,
    /// The map of pinned values that may not be deleted.
    pinned   : UnsafeCell<TreeMap <KeyRef<K>, Box<CacheItem<K, V>>>>,
}

impl<K: Ord, V: Cacheable> PinnableCache<K, V> {
    pub fn new() -> Allocation<PinnableCache<K, V>> {
        Ok(PinnableCache {
            unpinned : UnsafeCell::new(try!(LruCache::new())),
            pinned   : UnsafeCell::new(TreeMap::new()),
        })
    }

    fn notify_unpinned(&self, it: &CacheItem<K, V>) {
        assert!(it.pin_count() == 0);
        //dbg!(debug::PCACHE, "unpinning {}: {} down to 0", it.key(), it.value());
        let kr = KeyRef::new(it.key());
        let ci = self.pinned_mut().remove(&kr).expect("notify unpinned but not in pinned map");
        if ci.is_still_useful() {
            assert!(self.unpinned_mut().insert(kr, ci).is_none(), "already present unpinned value");
        } else {
            //dbg!(debug::PCACHE, "dropping no longer useful item {}", ci);
        }
    }

    /// The number of items that are currently not pinned by any user.
    #[inline]
    pub fn num_unpinned(&self) -> uint { self.unpinned().len() }

    /// The number of items that are currently pinned by some user.
    #[inline]
    pub fn num_pinned(&self) -> uint { self.pinned().len() }

    /// the total number of pinned and unpinned items in the cache.
    ///
    /// NOTE Validity depends on operation not being interruptable.
    #[inline]
    pub fn len(&self) -> uint { self.num_unpinned() + self.num_pinned() }

    pub fn get<'a>(&'a self, key: &K) -> Option<PinnedValue<'a, K, V>> {
        let kr = KeyRef::new(key);
        if let Some(v) = self.unpinned_mut().pop(&kr) {
            assert!(self.pinned_mut().insert(KeyRef::new(v.key()), v).is_none());
            Some(PinnedValue::create(self, &**(self.pinned().get(&kr).expect("just inserted value not present"))))
        } else { if let Some(v) = self.pinned().get(&kr) { Some(PinnedValue::create(self, &**v)) } else { None } }
    }

    /// Attempts to insert this value into the cache. Returns a Pinned value if we succeed,
    /// otherwise returns an error with failure reason.
    pub fn insert<'a>(&'a self, key: K, val: V) -> Result<PinnedValue<'a, K, V>,InsertError> {
        // VERY MUCH RELIES ON MUTUAL EXCLUSION
        if self.contains_key(&key) { return Err(InsertError::KeyPresent); }
        let item = try!(CacheItem::new(key, val).map_err(|x| InsertError::MemoryError(x)));
        let kr  = KeyRef::new(item.key());
        let kr2 = KeyRef::new(item.key());
        assert!(self.pinned_mut().insert(kr, item).is_none());
        let out = self.get(kr2.as_ref()).expect("Item cannot have been deleted");
        // Get rid of the extra pin we start out with.
        unsafe { out.manual_unpin(); }
        Ok(out)
    }

    /// attempts to insert the value into the cache.
    #[inline]
    pub fn insert_unpinned(&self, key: K, val: V) -> Result<(), InsertError> { self.insert(key, val).map(|_| ()) }

    /// Returns true if the key is contained within this cache.
    #[inline]
    pub fn contains_key(&self, key: &K) -> bool { self.get_state(key) != State::NotFound }

    /// Gets the state of the key in this cache
    ///
    /// Returns Pinned if the value is currently pinned, Unpinned if the value is not currently
    /// pinned but is present, and NotFound if we do not have the value in our cache.
    pub fn get_state(&self, key: &K) -> State {
        let kr = &KeyRef::new(key);
        if let Some(v) = self.pinned().get(kr) {
            State::Pinned(v.pin_count())
        } else if self.unpinned().contains_key(kr) {
            State::Unpinned
        } else {
            State::NotFound
        }
    }

    fn pinned_mut(&self) -> &mut TreeMap<KeyRef<K>, Box<CacheItem<K, V>>> { unsafe { transmute(self.pinned.get()) } }
    fn pinned(&self) -> &TreeMap<KeyRef<K>, Box<CacheItem<K, V>>> { unsafe { transmute(self.pinned.get()) } }

    fn unpinned_mut(&self) -> &mut LruCache<KeyRef<K>, Box<CacheItem<K, V>>> { unsafe { transmute(self.unpinned.get()) } }
    fn unpinned(&self) -> &LruCache<KeyRef<K>, Box<CacheItem<K, V>>> { unsafe { transmute(self.unpinned.get()) } }

    pub fn clear_unpinned(&mut self) {
        for (_, v) in self.unpinned_mut().iter_remove_least() {
            assert!(v.pin_count() == 0);
            drop(v);
        }
    }

    /// Remove all unpinned values that say they are not useful at this time. Returns the number of
    /// values removed from the cache.
    pub fn clean_unpinned(&mut self) -> uint {
        let mut cnt = 0;
        for m in self.unpinned_mut().iter_modify_least() {
            let &(_, v) = m.deref();
            if !v.is_still_useful() {
                drop(m);
                cnt += 1;
            }
        }
        return cnt;
    }

    /// Removes the least-recently-used unpinned value and free's it. Returns true if a value was
    /// destroyed.
    pub fn pop_unpinned(&mut self) -> bool {
        if let Some((_, v)) = self.unpinned_mut().pop_lru() {
            assert!(v.pin_count() == 0);
            drop(v);
            true
        } else {
            false
        }
    }
}

impl<'a, K, V:'a> PinnableCache<K, V> where K: Ord + Clone, V: TryMake<K, Errno> + Cacheable {
    pub fn add<'b: 'a>(&'b mut self, k: K) -> Result<PinnedValue<'b, K, V>,InsertError> {
        let val = try!(TryMake::try_make(k.clone()).map_err(|e| InsertError::SysError(Some(e))));
        self.insert(k, val)
    }

    pub fn add_or_get<'b: 'a>(&'b mut self, k: K) -> Result<PinnedValue<'a, K, V>, InsertError> {
        if self.contains_key(&k) {
            Ok(self.get(&k).unwrap())
        } else {
            self.add(k)
        }
    }
}

pub struct PinnedValue<'a, K: Ord + 'a, V: 'a> {
    cache : &'a PinnableCache<K, V>,
    value : &'a CacheItem<K, V>,
    // TODO
}

impl<'a, K: Ord + 'a, V: Cacheable + 'a> PinnedValue<'a, K, V> {
    /// Create a new pinned value. This will increment the items pin count for you.
    fn create<'b: 'a>(cache: &'b PinnableCache<K, V>, val: &'b CacheItem<K, V>) -> PinnedValue<'a, K, V> {
        val.pin();
        PinnedValue { cache: cache, value : val, }
    }

    /// Increase the pincount on this value manually. This has the effect of forcing the value to
    /// stay around and might lead to unfreed memory
    #[allow(unused_unsafe)]
    pub unsafe fn manual_pin(&self) { self.value.pin(); }

    /// Decrease the pin count on this value manually. This can lead to use-after-free if done
    /// incorrectly. Use with caution.
    #[allow(unused_unsafe)]
    pub unsafe fn manual_unpin(&self) { self.value.unpin(self.cache); }

    /// Reduce the pin count on this value. This is the same as dropping it.
    #[inline]
    pub fn unpin(self) { drop(self) }

    /// Increase the pin count on self/clone self.
    pub fn pin(&self) -> PinnedValue<'a, K, V> { PinnedValue::create(self.cache, self.value) }
}

impl<'a, K: Ord, V: Cacheable> Clone for PinnedValue<'a, K, V> {
    #[inline]
    fn clone(&self) -> PinnedValue<'a, K, V> { self.pin() }
}

#[unsafe_destructor]
impl<'a, K: Ord, V: Cacheable> Drop for PinnedValue<'a, K, V> {
    fn drop(&mut self) { self.value.unpin(self.cache); }
}
