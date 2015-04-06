// TODO Copyright Header

#![crate_name="util"]
#![crate_type="rlib"]

#![feature(asm, unsafe_destructor, plugin, box_syntax, core, alloc)]
#![doc(html_logo_url = "https://avatars.io/gravatar/d0ad9c6f37bb5aceac2d7ac95ba82607?size=large",
       html_favicon_url="https://avatars.io/gravatar/d0ad9c6f37bb5aceac2d7ac95ba82607?size=small")]

#[macro_use] extern crate base;
#[macro_use] extern crate mm;

pub use cacheable::Cacheable;

pub fn init_stage1() {}
pub fn init_stage2() {}
pub fn init_stage3() {}

//pub mod serialize;
pub mod uid;
pub mod lru_cache;
pub mod pinnable_cache;
pub mod cacheable;

mod list_node;

/// A module containing a key_ref, an unsafe reference to a value, used so we can have maps where
/// the key is recoverable. This is very unsafe.
mod key_ref {
    use std::mem::transmute;
    use std::cmp::Ordering;

    /// A struct used as the key for our map so we can get the key back out without trouble.
    /// Does not require lifetime bounds because the checker wouldn't be able to verify it since it
    /// depends on the map's values never being mutated.
    pub struct KeyRef<K> { k: *const K, }
    impl<K> KeyRef<K> {
        pub fn new(v: &K) -> KeyRef<K> { unsafe { KeyRef { k: transmute(v), } } }
        /// Get the key as a reference. Very unsafe.
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
}
