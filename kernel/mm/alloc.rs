// TODO Copyright Header

//! The reenix allocation support.
///
/// This provides support for malloc and free.
///
/// At the moment it simply goes down to the 'C' slab allocators but this might change in the
/// future if I ever get around to actually implementing a rust allocator.
///
/// We use our own tree here to avoid all the allocations that come with the stdlib one. This
/// should be used here only and nowhere else.

use core::prelude::*;
use core::cmp::min;
use core::intrinsics::transmute;
use core::{fmt, mem, ptr};
use libc::{size_t, c_void, c_int};
use super::utils;

pub static SLAB_REDZONE : u32 = 0xdeadbeef;

/// What is the largest slab size we will allow. 1/2 of page size
pub static MAX_SLAB_SIZE : uint = super::page::SIZE >> 1;

/// The slab from mm/slab.c
#[repr(C)]
struct CSlab {
        next  : *mut CSlab,  /* link on list of slabs */
        inuse : c_int,       /* number of allocated objs */
        free  : *mut c_void, /* head of obj free list */
        addr  : *mut c_void, /* start address */
}

/// The slab allocator from mm/slab.c
#[repr(C)]
struct CSlabAllocator {
        next     : *mut CSlabAllocator, /* link on list of slab allocators */
        name_len : size_t,              /* Length of the name in bytes */
        name     : *const u8,           /* user-provided name */
        objsize  : size_t,              /* object size */
        slabs    : *mut CSlab,          /* head of slab list */
        order    : c_int,               /* npages = (1 << order) */
        nobjs    : c_int,               /* number of objs per slab */
}

#[allow(raw_pointer_deriving)]
#[deriving(Eq,Clone,PartialEq)]
struct SlabAllocator(*mut CSlabAllocator);

extern "C" {
    fn slab_allocators_reclaim(target: c_int) -> c_int;
    fn slab_allocator_create_full(nsize: size_t, cstr: *const u8, size: size_t) -> *mut CSlabAllocator;
    fn slab_obj_alloc(a: *mut CSlabAllocator) -> *mut u8;
    fn slab_obj_free(a: *mut CSlabAllocator, ptr: *mut u8);
}

/// Try to free up as much memory as possible.
pub fn reclaim_memory() {
    let cnt = unsafe { slab_allocators_reclaim(0) };
    dbg!(debug::MM, "reclaimed {} pages from slab allocators", cnt);
}

impl SlabAllocator {
    #[inline]
    pub unsafe fn allocate(&self) -> *mut u8 {
        let &SlabAllocator(csa) = self;
        slab_obj_alloc(csa)
    }

    #[inline]
    pub unsafe fn deallocate(&self, ptr: *mut u8) {
        let &SlabAllocator(csa) = self;
        slab_obj_free(csa, ptr);
    }

    pub fn get_size(&self) -> size_t {
        let &SlabAllocator(csa) = self;
        unsafe { ptr::read(csa as *const CSlabAllocator).objsize }
    }

    pub fn get_name(&self) -> &'static str {
        use core::str::from_utf8;
        use core::raw::Slice;
        use core::ptr;
        let &SlabAllocator(csa) = self;
        let CSlabAllocator {next: _,
                            name_len: len,
                            name: name,
                            objsize: _,
                            slabs: _,
                            order: _,
                            nobjs: _ } = unsafe { ptr::read(csa as *const CSlabAllocator) };
        if len == 0 {
            ""
        } else {
            let s = Slice::<u8>{ data: name, len: len as uint };
            from_utf8(unsafe{ transmute(s) }).unwrap_or("ILLEGAL_NAME")
        }
    }

    pub fn new(name: &'static str, size: size_t) -> SlabAllocator {
        unsafe {
            SlabAllocator(slab_allocator_create_full(name.len() as size_t, name.as_ptr(), size))
        }
    }
}

impl fmt::Show for SlabAllocator {
    fn fmt(&self, w: &mut fmt::Formatter) -> fmt::Result {
        let name  = self.get_name();
        let size  = self.get_size();
        try!(w.write("SlabAllocator { name: '".as_bytes()));
        try!(name.fmt(w));
        try!(w.write("', objsize: ".as_bytes()));
        try!(size.fmt(w));
        try!(w.write("}".as_bytes()));
        Ok(())
    }
}

static mut NODE_ALLOCATOR: SlabAllocator = SlabAllocator(0 as *mut CSlabAllocator);

unsafe fn create_node() -> *mut utils::LightNode<SlabAllocator> { mem::transmute(NODE_ALLOCATOR.allocate()) }

static mut SLAB_ALLOCATORS: utils::LightMap<SlabAllocator> =
    utils::LightMap { root: 0 as *mut utils::LightNode<SlabAllocator>, len: 0, alloc: create_node };

extern "C" {
    fn slab_init();
}

/// Do one time startup initialization of the slab allocators and associated machinery.
#[deny(dead_code)]
pub fn init() {
    unsafe { slab_init(); }
    let node_size = mem::size_of::<super::utils::LightNode<*mut SlabAllocator>>();
    unsafe { NODE_ALLOCATOR = SlabAllocator::new("Map Node Allocator", node_size as size_t); }
    add_kmalloc_slabs();
    unsafe { SLAB_ALLOCATORS.add(node_size, NODE_ALLOCATOR); }
    dbg!(debug::MM, "Allocator tree is {}", SLAB_ALLOCATORS );
}

/// are we done with the memory management setup?
static mut REQUESTS_CLOSED : bool = false;

/// True when we have finished setting up allocators.
pub fn requests_closed() -> bool { unsafe { REQUESTS_CLOSED } }

/// Note that we have finished creating all needed allocators, we can start using liballoc once
/// this is called.
pub fn close_requests() { unsafe { REQUESTS_CLOSED = true; } }

/// Request that a slab allocator be made to service requests of the given size.
///
/// One should do this if it is known that there will be a lot of requests for a specific object.
/// Other sizes will also be created using the same slab allocators, though these might get fragmented.
pub fn request_slab_allocator(name: &'static str, size: size_t) {
    if requests_closed() {
        dbg!(debug::MM, "New Allocator requested after requests closed. ignoring.");
        return;
    }
    let cur = unsafe { SLAB_ALLOCATORS.find(size as uint) };
    match cur {
        None => {},
        Some(sa) => {
            // NOTE Rust strings not being (gaurenteed to be) null terminated is extreemly annoying
            //let r = unsafe { sa.as_ref().expect("Found a null slab allocator") };
            dbg!(debug::MM, "Request to make allocator '{}' for {} bytes already fullfilled by {}",
                 name, size as uint, sa);
            return;
        }
    }
    let new_slab = SlabAllocator::new(name, size as size_t);
    unsafe { SLAB_ALLOCATORS.add(size as uint, new_slab) };
    dbg!(debug::MM, "Added allocator called '{}' for a size of {}", name, size);
}

fn add_kmalloc_slabs() {
    // Done in this order so the tree will be somewhat balanced.
    maybe_add_kmalloc_slab(3);
    maybe_add_kmalloc_slab(1);
    maybe_add_kmalloc_slab(5);
    maybe_add_kmalloc_slab(0);
    maybe_add_kmalloc_slab(2);
    maybe_add_kmalloc_slab(4);
    maybe_add_kmalloc_slab(6);
    let mut i = 7;
    while maybe_add_kmalloc_slab(i) { i += 1 }
}

extern "C" {
    #[link_name="get_kmalloc_allocator"]
    fn get_alloc(i: c_int) -> *mut CSlabAllocator;
}


fn maybe_add_kmalloc_slab(i: c_int) -> bool {
    let sa = unsafe { get_alloc(i) };
    if sa.is_null() {
        dbg!(debug::MM,  "There was no kmalloc object {}, recieved null", i);
        return false;
    }
    let new_slab = SlabAllocator(sa);
    if new_slab.get_size() > super::page::SIZE as size_t {
        dbg!(debug::MM, "kmalloc object {} was larger than largest size we will use slab objs for", new_slab);
        false
    } else {
        unsafe { SLAB_ALLOCATORS.add(new_slab.get_size() as uint, new_slab); }
        dbg!(debug::MM, "Added kmalloc object {} {}", i, new_slab);
        true
    }
}

// TODO I need to make sure to handle calls to this before we lock down from creating new slabs.
// Either we need to disallow any calls before that or we need to create a slab for them whenever
// it happens.
#[allow(unused_unsafe)]
#[inline]
pub unsafe fn allocate(size: uint, _align: uint) -> *mut u8 {
    if !requests_closed() {
        // TODO Decide what I should do here. Panicing might not be best.
        panic!("Attempt to call allocate before we have finished setting up the allocators.");
    }
    if size >= MAX_SLAB_SIZE {
        use super::page;
        let pages = page::addr_to_num(page::const_align_up(size as *const u8));
        dbg!(debug::MM, "Allocating {} pages to satisfy a request for {} bytes", pages, size);
        let res = page::alloc_n(pages as u32) as *mut u8;
        if res.is_null() {
            dbg!(debug::MM, "Allocation of {} pages failed for request of {} bytes. Reclaiming memory and retrying.", pages, size);
            reclaim_memory();
            return page::alloc_n(pages as u32) as *mut u8
        } else {
            return res;
        }
    }
    let alloc = SLAB_ALLOCATORS.find_smallest(size);
    match alloc {
        None => {
            // This should never really happen, truely large ones will get their own page.
            panic!("Unable to find a large enough slab for something that is smaller than a page in length at {} bytes!", size);
        },
        Some((alloc_size, sa)) => {
            dbg!(debug::MM, "Allocating from {} for request for {}", sa, size);
            assert!(alloc_size >= size, "allocator's size {} was less then required size {}", alloc_size, size);
            assert!(sa.get_size() as uint >= size, "{} is not large enough for allocation of {}", sa, size);
            let res = sa.allocate();
            if res.is_null() {
                dbg!(debug::MM, "Allocation from slab {} failed for request of {} bytes. Reclaiming memory and retrying.", sa, size);
                reclaim_memory();
                return sa.allocate();
            } else {
                return res;
            }
        }
    }
}

#[allow(unused_unsafe)]
#[inline]
pub unsafe fn reallocate(ptr: *mut u8, size: uint, align: uint,
                             old_size: uint) -> *mut u8 {
    use core::intrinsics::copy_nonoverlapping_memory;
    if reallocate_inplace(ptr, size, align, old_size) {
        ptr
    } else {
        let new_ptr = allocate(size, align);
        if !new_ptr.is_null() {
            copy_nonoverlapping_memory(new_ptr, ptr as *const u8, min(size, old_size));
            deallocate(ptr, old_size, align);
        } else {
            dbg!(debug::MM, "Unable to allocate memory for realloc of {} from {} to {} bytes", ptr, old_size, size);
        }
        new_ptr
    }
}

#[allow(unused_unsafe)]
#[inline]
pub unsafe fn reallocate_inplace(_ptr: *mut u8, size: uint, _align: uint,
                                    old_size: uint) -> bool {
    use super::page;
    if size >= MAX_SLAB_SIZE && old_size >= MAX_SLAB_SIZE {
        let new_pages = page::addr_to_num(page::const_align_up(size as *const u8));
        let old_pages = page::addr_to_num(page::const_align_up(old_size as *const u8));
        old_pages == new_pages
    } else if size >= MAX_SLAB_SIZE || old_size >= MAX_SLAB_SIZE {
        false
    } else {
        // Check if we have the same allocator
        let (_, new_alloc) = SLAB_ALLOCATORS.find_smallest(size).expect("Unable to find slab allocator that was used.");
        let (_, old_alloc) = SLAB_ALLOCATORS.find_smallest(old_size).expect("Unable to find slab allocator that was used.");
        new_alloc == old_alloc
    }

}

#[allow(unused_unsafe)]
#[inline]
pub unsafe fn deallocate(ptr: *mut u8, size: uint, _align: uint) {
    if size >= MAX_SLAB_SIZE {
        use super::page;
        use libc::c_void;
        let pages = page::addr_to_num(page::const_align_up(size as *const u8));
        dbg!(debug::MM, "Deallocating {} pages used to satisfy a request for {} bytes", pages, size);
        page::free_n(ptr as *mut c_void, pages as u32);
        return;
    }
    let alloc = SLAB_ALLOCATORS.find_smallest(size);
    match alloc {
        None => {
            // This should never really happen, truely large ones will get their own page.
            panic!("Unable to find a large enough slab for something that is smaller than a page in length at {} bytes!", size);
        },
        Some((_, sa)) => {
            sa.deallocate(ptr);
        }
    }
}

#[inline]
pub fn usable_size(size: uint, _align: uint) -> uint {
    if size >= MAX_SLAB_SIZE {
        use super::page;
        use libc::c_void;
        unsafe { page::const_align_up(size as *const u8) as uint }
    } else {
        let alloc = unsafe {SLAB_ALLOCATORS.find_smallest(size)};
        match alloc {
            None => {
                // This should never really happen, truely large ones will get their own page.
                panic!("Unable to find a large enough slab for something that is smaller than a page in length at {} bytes!", size);
            },
            Some((_,sa)) => {
                sa.get_size() as uint
            }
        }
    }
}

