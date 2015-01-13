// TODO Copyright Header

//! A map which we can use before memory allocation is fully availible.

use core::prelude::*;
use core::fmt;
use core::cmp;
use alloc::SlabAllocator;

pub const NUM_ALLOCATORS : usize = 256;
pub const DEFAULT_SLAB_MAP : SlabMap = SlabMap { vals: [None; NUM_ALLOCATORS], cnt: 0 };

pub struct SlabMap {
    vals: [Option<SlabAllocator>; NUM_ALLOCATORS],
    cnt : usize,
}

impl fmt::Show for SlabMap {
    fn fmt(&self, w: &mut fmt::Formatter) -> fmt::Result {
        try!(write!(w, "SlabMap (size: {}) [", self.len()));
        if self.cnt != 0 {
            try!(write!(w, "{:?}", self.vals[0].expect("shouldn't be null")));
            for &i in self.vals.slice(1, self.cnt).iter() {
                try!(write!(w, ", {:?}", i.expect("shouldn't be null")));
            }
        }
        write!(w, "]")
    }
}

/// Just a basic quick sort.
fn sort(l: &mut [Option<SlabAllocator>], lo: usize, hi: usize) {
    if hi - lo < 2  || lo > hi {
        return;
    }
    let pivot = hi - 1;
    let mut left = lo;
    let mut store = lo;
    let pval = l[pivot].expect("shouldn't be null").get_size();
    while left <= pivot - 1 {
        if l[left].expect("shouldn't be null").get_size() < pval {
            l.swap(left, store);
            store = store + 1;
        }
        left = left + 1;
    }
    l.swap(store, pivot);
    sort(l, lo, store);
    sort(l, store + 1, hi);
}

impl SlabMap {
    pub fn finish(&mut self) {
        sort(&mut self.vals, 0, self.cnt);
        let mut prev = 0;
        for i in range(0, self.len()) {
            let size = self.vals[i].expect("shouldn't be null").get_size();
            if size <= prev {
                kpanic!("repeated items or out of order slab map. prev {}, cur {}, index {}", prev, size, i);
            } else {
                prev = size;
            }
        }
        dbg!(debug::MM, "slab map is {:?}", self);
    }

    pub fn add(&mut self, v: SlabAllocator) {
        if self.cnt == NUM_ALLOCATORS {
            dbg!(debug::MM | debug::CORE, "Ignoring {:?} because we already have too many!", v);
        } else if !self.brute_check(v.get_size() as usize) {
            self.vals[self.cnt] = Some(v);
            self.cnt += 1;
        } else {
            dbg!(debug::MM, "ignoring slab {:?} for already present one.", v);
        }
    }

    fn brute_check(&self, k: usize) -> bool {
        for i in range(0, self.cnt) {
            match self.vals[i] {
                None => { kpanic!("Should not have nulls in allocated region"); },
                Some(sa) => { if sa.get_size() as usize == k { return true; } },
            }
        }
        return false;
    }

    pub fn find(&self, key: usize) -> Option<SlabAllocator> {
        match self.find_smallest(key) {
            None => None,
            Some(sa) => if sa.get_size() as usize == key { Some(sa) } else { None }
        }
    }

    pub fn find_smallest(&self, key: usize) -> Option<SlabAllocator> {
        match self.vals.slice_to(self.cnt).binary_search_by(|&:v| -> cmp::Ordering { (v.expect("should have value").get_size() as usize).cmp(&key) }) {
            Ok(v)  => Some(self.vals[v].expect("should have value")),
            Err(v) => if v == self.cnt { None } else { Some(self.vals[v].expect("should have value")) },
        }
    }

    #[inline]
    pub fn len(&self) -> usize { self.cnt }
}
