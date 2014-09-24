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
use core::default::Default;
use core::{mem, uint};
use core::ptr::RawMutPtr;
use core::intrinsics::transmute;
use libc::{size_t, c_void, c_int};
use super::utils;

pub static SLAB_REDZONE : u32 = 0xdeadbeef;

/// What is the smallest slab size we will allow.
pub static MIN_SLAB_SIZE : uint = 4 * uint::BYTES;

/// How much bigger smaller must this allocation be then the next best slab for us to create a new
/// slab just for it?
pub static MIN_SIZE_DIFF_FOR_NEW_SLAB : uint = 4 * uint::BYTES;

type c_bool = c_int;

#[repr(C)]
struct Slab {
        next  : *mut Slab,   /* link on list of slabs */
        inuse : c_bool,      /* number of allocated objs */
        free  : *mut c_void, /* head of obj free list */
        addr  : *mut c_void, /* start address */
}


/// The slab allocator from mm/slab.c
#[repr(C)]
struct SlabAllocator {
        next       : *mut SlabAllocator, /* link on list of slab allocators */
        name_len   : size_t,             /* Length of the name in bytes */
        name       : *const u8,          /* user-provided name */
        objsize    : size_t,             /* object size */
        slabs      : *mut Slab,          /* head of slab list */
        order      : c_int,              /* npages = (1 << order) */
        slab_nobjs : c_int,              /* number of objs per slab */
}

extern "C" {
    fn slab_allocators_reclaim(target: c_int) -> c_int;
    fn slab_allocator_create_full(nsize: size_t, cstr: *const u8, size: size_t) -> *mut SlabAllocator;
    fn slab_obj_alloc(a: *mut SlabAllocator) -> *mut u8;
    fn slab_obj_free(a: *mut SlabAllocator, ptr: *mut u8);
}

static mut NODE_ALLOCATOR: *mut SlabAllocator = 0 as *mut SlabAllocator;

unsafe fn create_node() -> *mut utils::LightNode<*mut SlabAllocator> { mem::transmute(slab_obj_alloc(NODE_ALLOCATOR)) }

static mut SLAB_ALLOCATORS: utils::LightMap<*mut SlabAllocator> = utils::LightMap { root: 0 as *mut utils::LightNode<*mut SlabAllocator>, len: 0, alloc: create_node };

/// Do one time startup initialization of the slab allocators and associated machinery.
pub fn init() {
    let node_size = mem::size_of::<super::utils::LightNode<*mut SlabAllocator>>();
    let n = "Map Node Allocator";
    unsafe { NODE_ALLOCATOR = slab_allocator_create_full(n.len() as size_t, n.as_ptr(), node_size as size_t); }
    unsafe { SLAB_ALLOCATORS.add(node_size, NODE_ALLOCATOR); }
}

static mut REQUESTS_CLOSED : bool = false;
fn requests_closed() -> bool { unsafe { REQUESTS_CLOSED } }

pub fn request_slab_allocator(name: &'static str, size: size_t) {
    use base::debug;
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
            dbg!(debug::MM, "Request to make allocator for {} already fullfilled by allocator for {}",
                 name, "'TODO, HOW DO I CONVERT FROM CSTR TO str'");
            return;
        }
    }
    let new_slab = unsafe { slab_allocator_create_full(name.len() as size_t, name.as_ptr(), size as size_t) };
    unsafe { SLAB_ALLOCATORS.add(size as uint, new_slab) };
}

#[inline]
pub unsafe fn allocate(size: uint, align: uint) -> *mut u8 {
    transmute::<uint, *mut u8>(0)
}

#[inline]
pub unsafe fn reallocate(ptr: *mut u8, size: uint, align: uint,
                             _old_size: uint) -> *mut u8 {
    ptr
}

#[inline]
pub unsafe fn reallocate_inplace(ptr: *mut u8, size: uint, align: uint,
                                    _old_size: uint) -> bool {
    false
}
#[inline]
pub unsafe fn deallocate(ptr: *mut u8, size: uint, align: uint) {  }

#[inline]
pub fn usable_size(size: uint, align: uint) -> uint { size }
