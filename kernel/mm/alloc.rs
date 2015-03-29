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
use slabmap::{SlabMap, DEFAULT_SLAB_MAP};
use backup::{BackupAllocator, DEFAULT_BACKUP_ALLOCATOR};
use page;

const RC_OVERHEAD : size_t = 12;

/// An allocator is a type that can allocate memory. It is currently a wrapper around C functions.
struct Allocator {
    slabs : SlabMap,
    pages : PageAllocator,
    backup : BackupAllocator,
}

/// A type that can allocate pages. Currently is just a shim to the C page allocator.
struct PageAllocator;
impl PageAllocator {
    pub unsafe fn alloc_n(&self, n: u32) -> *mut u8 {
        use super::page;
        page::c_alloc_n(n) as *mut u8
    }
    pub unsafe fn free_n(&self, ptr: *mut u8, n : u32) {
        use libc::c_void;
        use super::page;
        page::free_n(ptr as *mut c_void, n);
    }
}

const DEFAULT_PAGE_ALLOCATOR : PageAllocator = PageAllocator;

static mut BASE_ALLOCATOR : Allocator = Allocator {
    slabs : DEFAULT_SLAB_MAP,
    pages : DEFAULT_PAGE_ALLOCATOR,
    backup : DEFAULT_BACKUP_ALLOCATOR,
};

/// A type representing that we had an error allocating. We might put more in this eventually.
#[derive(Copy)]
pub struct AllocError;
impl fmt::Debug for AllocError {
    fn fmt(&self, w: &mut fmt::Formatter) -> fmt::Result {
        write!(w, "AllocError")
    }
}
/// The result of an allocation.
pub type Allocation<T> = Result<T, AllocError>;

pub static SLAB_REDZONE : u32 = 0xdeadbeef;

/// What is the largest slab size we will allow. 1/2 of page size
pub static MAX_SLAB_SIZE : usize = super::page::SIZE >> 1;

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

#[allow(raw_pointer_derive)]
#[derive(Eq,Clone,PartialEq, Copy)]
pub struct SlabAllocator(*mut CSlabAllocator);

extern "C" {
    fn slab_allocators_reclaim(target: c_int) -> c_int;
    fn slab_allocator_create_full(nsize: size_t, cstr: *const u8, size: size_t) -> *mut CSlabAllocator;
    fn slab_obj_alloc(a: *mut CSlabAllocator) -> *mut u8;
    fn slab_obj_free(a: *mut CSlabAllocator, ptr: *mut u8);
    fn slab_obj_num_allocated(a: *mut CSlabAllocator) -> u32;
}

/// Try to free up as much memory as possible.
#[invariant="requests_closed()"]
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
        unsafe { ptr::read(csa as *const CSlabAllocator).objsize - (2 * mem::size_of::<usize>()) as u32 }
    }

    pub fn get_name(&self) -> &'static str {
        use core::str::from_utf8;
        use core::raw::Slice;
        use core::ptr;
        let &SlabAllocator(csa) = self;
        let CSlabAllocator {next: _,
                            name_len: len,
                            name,
                            objsize: _,
                            slabs: _,
                            order: _,
                            nobjs: _ } = unsafe { ptr::read(csa as *const CSlabAllocator) };
        if len == 0 {
            ""
        } else {
            let s = Slice::<u8>{ data: name, len: len as usize };
            from_utf8(unsafe{ transmute(s) }).unwrap_or("ILLEGAL_NAME")
        }
    }

    pub fn num_allocated(&self) -> usize {
        let &SlabAllocator(csa) = self;
        unsafe { slab_obj_num_allocated(csa) as usize }
    }

    pub fn new(name: &'static str, size: size_t) -> SlabAllocator {
        unsafe {
            let v = slab_allocator_create_full(name.len() as size_t, name.as_ptr(), size);
            assert!(v != 0 as *mut CSlabAllocator);
            SlabAllocator(v)
        }
    }
}

impl fmt::Debug for SlabAllocator {
    fn fmt(&self, w: &mut fmt::Formatter) -> fmt::Result {
        let name  = self.get_name();
        let size  = self.get_size();
        write!(w, "SlabAllocator {{ name: '{}', objsize: {}, inuse: {} }}", name, size, self.num_allocated())
    }
}

extern "C" {
    fn slab_init();
}

/// Do one time startup initialization of the slab allocators and associated machinery.
#[deny(dead_code)]
#[invariant = "!requests_closed()"]
pub fn init_stage1() {
    unsafe { slab_init(); }
    let ba = unsafe { &mut BASE_ALLOCATOR };
    ba.add_kmalloc_slabs();
    request_slab_allocator("MAX SIZE SLAB", MAX_SLAB_SIZE as u32);
}

#[invariant = "requests_closed()"]
pub fn init_stage2() {}

/// are we done with the memory management setup?
static mut REQUESTS_CLOSED : bool = false;

/// True when we have finished setting up allocators.
pub fn requests_closed() -> bool { unsafe { REQUESTS_CLOSED } }

/// Note that we have finished creating all needed allocators, we can start using liballoc once
/// this is called.
#[precond  = "!requests_closed()"]
#[postcond = "requests_closed()"]
pub fn close_requests() { dbg!(debug::MM, "Requests closed"); unsafe { REQUESTS_CLOSED = true; (&mut BASE_ALLOCATOR).finish() } }

#[inline]
pub fn request_rc_slab_allocator(name: &'static str, size: size_t) {
    request_slab_allocator(name, size + RC_OVERHEAD)
}
/// Request that a slab allocator be made to service requests of the given size.
///
/// One should do this if it is known that there will be a lot of requests for a specific object.
/// Other sizes will also be created using the same slab allocators, though these might get fragmented.
#[debug_invariant = "!requests_closed()"]
pub fn request_slab_allocator(name: &'static str, size: size_t) {
    if requests_closed() {
        dbg!(debug::MM, "New Allocator requested after requests closed. ignoring.");
        return;
    }
    let ba = unsafe { &mut BASE_ALLOCATOR };
    ba.request_slab_allocator(name, size);
}

impl fmt::Debug for Allocator {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        try!(writeln!(f, "Weenix allocator"));
        try!(writeln!(f, "{:?}", self.slabs));
        try!(writeln!(f, "free pages: {:?}", unsafe { page::free_count()} ));
        writeln!(f, "{:?}", self.backup)
    }
}

impl Allocator {
    pub fn finish(&mut self) {
        self.slabs.finish();
        self.backup.finish();
    }
    pub fn request_slab_allocator(&mut self, name: &'static str, size: size_t) {
        let cur = self.slabs.find(size as usize);
        match cur {
            None => {},
            Some(sa) => {
                // NOTE Rust strings not being (gaurenteed to be) null terminated is extreemly annoying
                //let r = unsafe { sa.as_ref().expect("Found a null slab allocator") };
                dbg!(debug::MM, "Request to make allocator '{:?}' for {:?} bytes already fullfilled by {:?}",
                    name, size as usize, sa);
                return;
            }
        }
        let new_slab = SlabAllocator::new(name, size as size_t);
        self.slabs.add(new_slab);
        dbg!(debug::MM, "Added allocator called '{}' for a size of {}", name, size);
    }

    pub fn add_kmalloc_slabs(&mut self) {
        let mut i = 0;
        while self.maybe_add_kmalloc_slab(i) { i += 1 }
    }

    fn maybe_add_kmalloc_slab(&mut self, i: c_int) -> bool {
        extern "C" {
            #[link_name="get_kmalloc_allocator"]
            fn get_alloc(i: c_int) -> *mut CSlabAllocator;
        }

        let sa = unsafe { get_alloc(i) };
        if sa.is_null() {
            dbg!(debug::MM,  "There was no kmalloc object {}, recieved null", i);
            return false;
        }
        let new_slab = SlabAllocator(sa);
        if new_slab.get_size() > (MAX_SLAB_SIZE as u32) + 1 {
            dbg!(debug::MM, "kmalloc object {:?} was larger than largest size we will use slab objs for", new_slab);
            false
        } else {
            self.slabs.add(new_slab);
            dbg!(debug::MM, "Added kmalloc object {:?} {:?}", i, new_slab);
            true
        }
    }

    pub unsafe fn allocate(&self, size: usize, align: usize) -> *mut u8 {
        let res = if is_enabled!(TEST -> LOW_MEMORY) { 0 as *mut u8 } else { self.do_allocate(size, align) };
        if res.is_null() {
            dbg!(debug::BACKUP_MM|debug::MM, "Unable to allocate from normal allocators. Trying to use backup");
            let out = self.backup.allocate(size, align);
            if out.is_null() {
                dbg!(debug::DANGER|debug::MM, "Unable to allocate from backup allocator!");
            } else {
                dbg!(debug::MM, "Allocated {:p} of size {} from backup allocator", out, size);
            }
            out
        } else {
            dbg!(debug::MM, "Allocated {:p} of size {}", res, size);
            res
        }
    }

    #[inline]
    unsafe fn do_allocate(&self, size: usize, _align: usize) -> *mut u8 {
        if size >= MAX_SLAB_SIZE {
            use super::page;
            let pages = page::addr_to_num(page::const_align_up(size as *const u8));
            dbg!(debug::MM, "Allocating {} pages to satisfy a request for {} bytes", pages, size);
            let res = self.pages.alloc_n(pages as u32) as *mut u8;
            if res.is_null() {
                dbg!(debug::MM, "Allocation of {} pages failed for request of {} bytes. Reclaiming memory and retrying.", pages, size);
                reclaim_memory();
                return self.pages.alloc_n(pages as u32) as *mut u8
            } else {
                return res;
            }
        }
        let alloc = self.slabs.find_smallest(size);
        match alloc {
            None => {
                // This should never really happen, truely large ones will get their own page.
                kpanic!("Unable to find a large enough slab for something that is smaller than a page in length at {} bytes!", size);
            },
            Some(sa) => {
                dbg!(debug::MM, "Allocating {:?} from {:?}", size, sa);
                bassert!(sa.get_size() as usize >= size, "allocator's size {:?} was less then required size {:?}", sa.get_size(), size);
                let res = sa.allocate();
                if res.is_null() {
                    dbg!(debug::MM, "Allocation from slab {:?} failed for request of {:?} bytes. Reclaiming memory and retrying.", sa, size);
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
    pub unsafe fn reallocate(&self, ptr: *mut u8, old_size: usize, size: usize, align: usize) -> *mut u8 {
        use core::intrinsics::copy_nonoverlapping_memory;
        dbg!(debug::MM, "reallocating {:p} from size of {} to size {}", ptr, old_size, size);
        if self.can_reallocate_inplace(ptr, old_size, size, align) {
            ptr
        } else {
            dbg!(debug::MM, "manually reallocating {:p} of size {} to size {}", ptr, old_size, size);
            let new_ptr = self.allocate(size, align);
            if !new_ptr.is_null() {
                copy_nonoverlapping_memory(new_ptr, ptr as *const u8, min(size, old_size));
                self.deallocate(ptr, old_size, align);
            } else {
                dbg!(debug::MM, "Unable to allocate memory for realloc of {:p} from {} to {} bytes", ptr, old_size, size);
                return 0 as *mut u8;
            }
            new_ptr
        }
    }

    #[allow(unused_unsafe)]
    #[inline]
    pub unsafe fn can_reallocate_inplace(&self, ptr: *mut u8, old_size: usize, size : usize,
                                        _align: usize) -> bool {
        use super::page;
        if self.backup.contains(ptr) && old_size != size {
            false
        } else if old_size == size {
            true
        } else if size >= MAX_SLAB_SIZE && old_size >= MAX_SLAB_SIZE {
            let new_pages = page::addr_to_num(page::const_align_up(size as *const u8));
            let old_pages = page::addr_to_num(page::const_align_up(old_size as *const u8));
            old_pages == new_pages
        } else if size >= MAX_SLAB_SIZE || old_size >= MAX_SLAB_SIZE {
            false
        } else {
            // Check if we have the same allocator
            let new_alloc = self.slabs.find_smallest(size).expect("Unable to find slab allocator that was used.");
            let old_alloc = self.slabs.find_smallest(old_size).expect("Unable to find slab allocator that was used.");
            if new_alloc != old_alloc {
                dbg!(debug::MM, "posibly reallocating from {:?} (size {}) to {:?} (size {})", old_alloc, old_size, new_alloc, size);
            }
            new_alloc == old_alloc
        }
    }

    #[allow(unused_unsafe)]
    #[inline]
    pub unsafe fn deallocate(&self, ptr: *mut u8, size: usize, _align: usize) {
        if self.backup.contains(ptr) {
            self.backup.deallocate(ptr, size, _align);
            return;
        }
        if size >= MAX_SLAB_SIZE {
            use super::page;
            let pages = page::addr_to_num(page::const_align_up(size as *const u8));
            dbg!(debug::MM, "Deallocating {} pages used to satisfy a request for {} bytes", pages, size);
            self.pages.free_n(ptr, pages as u32);
            return;
        }
        let alloc = self.slabs.find_smallest(size);
        match alloc {
            None => {
                // This should never really happen, truely large ones will get their own page.
                kpanic!("Unable to find a large enough slab for something that is smaller than a page in length at {} bytes!", size);
            },
            Some(sa) => {
                dbg!(debug::MM, "deallocating {:p} with {} bytes from {:?}", ptr, size, sa);
                sa.deallocate(ptr);
            }
        }
    }

    #[inline]
    pub fn usable_size(&self, size: usize, _align: usize) -> usize {
        // TODO This depends on which allocator the pointer is in. Since we cannot know that just
        // from the size we need to say that there is no extra space. If they call realloc we might
        // not move them anyway.
        size
    }

    /// Return's true if we have low memory and have a better then even chance of failing to allocate a
    /// value if asked. Even when true allocations might continue to succeed.
    pub fn is_memory_low(&self) -> bool {
        // We just ask the backup if it feels ok. As long as it does we are good.
        self.backup.is_memory_low()
    }
}

// TODO I need to make sure to handle calls to this before we lock down from creating new slabs.
// Either we need to disallow any calls before that or we need to create a slab for them whenever
// it happens.
#[allow(unused_unsafe)]
#[inline]
#[precond = "requests_closed()"]
pub unsafe fn allocate(size: usize, _align: usize) -> *mut u8 {
    if !requests_closed() {
        // TODO Decide what I should do here. Panicing might not be best.
        kpanic!("Attempt to call allocate before we have finished setting up the allocators.");
    }
    let x = &BASE_ALLOCATOR;
    x.allocate(size, _align)
}

#[allow(unused_unsafe)]
#[inline]
#[precond = "requests_closed()"]
pub unsafe fn reallocate(ptr: *mut u8, old_size: usize, size: usize,
                             align: usize) -> *mut u8 {
    let x = &BASE_ALLOCATOR;
    x.reallocate(ptr, old_size, size, align)
}

#[allow(unused_unsafe)]
#[inline]
#[precond = "requests_closed()"]
pub unsafe fn reallocate_inplace(_ptr: *mut u8, old_size: usize, size : usize,
                                    _align: usize) -> usize {
    let x = &BASE_ALLOCATOR;
    if x.can_reallocate_inplace(_ptr, old_size, size, _align) {
        size
    } else {
        old_size
    }
}

#[allow(unused_unsafe)]
#[inline]
#[precond = "requests_closed()"]
pub unsafe fn deallocate(ptr: *mut u8, size: usize, _align: usize) {
    let x = &BASE_ALLOCATOR;
    x.deallocate(ptr, size, _align)
}

#[inline]
#[precond = "requests_closed()"]
pub fn usable_size(size: usize, _align: usize) -> usize {
    let x = unsafe { &BASE_ALLOCATOR };
    x.usable_size(size, _align)
}

#[precond = "requests_closed()"]
pub fn stats_print() {
    dbg!(debug::MM|debug::CORE, "{:?}", unsafe { &BASE_ALLOCATOR });
}

#[precond = "requests_closed()"]
pub fn get_stats() -> &'static (fmt::Debug + 'static) {
    let x = unsafe { &BASE_ALLOCATOR };
    x as &'static fmt::Debug
}

/// Return's true if we have low memory and have a better then even chance of failing to allocate a
/// value if asked. Even when true allocations might continue to succeed.
#[precond = "requests_closed()"]
pub fn is_memory_low() -> bool {
    (unsafe { &BASE_ALLOCATOR }).is_memory_low()
}

