
//! Pframes for Reenix.

use alloc::rc::*;
use core::prelude::*;
use base::errno;
use libc::c_void;
use core::fmt::*;
use mm::{Allocation, AllocError};
use core::mem::size_of;
use mmobj;

pub use pframe::pfstate::PFState;
use util::pinnable_cache::{mod, PinnableCache};

pub type PageNum = uint;

#[deriving(PartialEq, Eq, PartialOrd, Ord, Show, Clone)]
pub struct PFrameId { mmobj: Rc<Box<MMObj>>, page: PageNum, }
impl PFrameId {
    /// Create a pframe id.
    pub fn new(mmo: Rc<Box<MMObj>>, page: PageNum) -> PFrameId { PFrameId { mmobj: mmo, page: page } }
}

impl Make<(Rc<Box<MMObj>>, PageNum)> for PFrameId {
    fn make(v: (Rc<Box<MMObj>>, PageNum)) -> PFrameId { let (mmo, page) = v; PFrameId::new(mmo, page) }
}

static mut PFRAME_CACHE : *mut PinnableCache<PFrameId, PFrame> = 0 as *mut PinnableCache<PFrameId, PFrame>;

pub fn init_stage1() {
    // TODO
    // The allocator for the alloc_list
    pinnable_cache::request_pinnable_cache_allocator::<PFrameId, PFrame>("pframe pinnable cache");
}

pub fn init_stage2() {
    let pfcache : PinnableCache<PFrameId, PFrame> = PinnableCache::new().unwrap();
    unsafe { PFRAME_CACHE = transmute(pfcache); }
    // TODO
}

pub fn init_stage3() {
    // TODO
}

pub mod pfstate {
    bitmask_create!(flags PFState : u8 {
        default NONE,
        DIRTY   = 0,
        BUSY    = 1,
        INITING = 2
    })
}

pub struct PFrame {
    /// A weak reference to the creating mmobj.
    obj     : Weak<Box<MMObj>>,
    pagenum : PageNum,

    page : *mut c_void,

    flags : Cell<PFState>,
    queue : WQueue,
}

pub struct PinGaurd<'a> { pf: &'a PFrame, }
impl<'a> Drop for PinGaurd<'a> { fn drop(&mut self) { self.pf.manual_unpin(); } }

pub enum PFError { Alloc(AllocError), Sys(errno::Errno), }
impl PFrame {
    // TODO pframe_migrate?
    /// Makes a new pframe, also makes sure to allocate memory space for it.
    pub fn create(mmo : Rc<Box<MMObj>>, page_num: uint) -> Result<PFrame,PFError> {
        use PFError::*;
        Ok({
            let mut res = PFrame {
                obj : mmo,
                pagenum : page_num,

                page : try!(alloc!(try page::alloc()).map_err(|v| Err(Alloc(v)))),

                flags : Cell::new(pfstate::NORMAL | pfstate::INITING),
                queue : try!(alloc!(try WQueue::new())),
                pincount : AtomicUint::new(0),
            };
            try!(res.fill(mmo).map_err(|v| Err(Sys(v))))
            return res;
        })
    }

    /**
     * Fills the contents of the page (using the mmobj's fillpage op).
     * Make sure to mark the page busy while it's being filled.
     */
    fn fill(&mut self, obj: &MMObj) -> KResult<()> {
        bassert!(self.flags.get() == pfstate::INITING);
        self.set_busy();
        let res = obj.fill_page(self);
        self.clear_busy();
        if res.is_ok() { self.flags.set(self.flags.get() & !pfstate::INITING); }
        return res;
    }

    fn set_busy(&self) {
        self.flags.set(self.flags.get() | pfstate::BUSY);
    }

    fn clear_busy(&self) {
        self.flags.set(self.flags.get() & !pfstate::BUSY);
        self.queue.signal();
    }

    #[inline]
    pub fn is_busy(&self) -> bool { self.flags.get() & pfstate::BUSY != pfstate::NORMAL }
    #[inline]
    pub fn is_dirty(&self) -> bool { self.flags.get() & pfstate::DIRTY != pfstate::NORMAL }
    #[inline]
    pub fn is_pinned(&self) -> bool { self.pincount.load(SeqCst) != 0 }

    /// Wait for the given pframe to stop being busy. This procedure will block if the pframe is
    /// currently busy.
    pub fn wait_busy(&self) -> Result<(),()> {
        while self.flags.get() & pfstate::BUSY != pfstate::NORMAL {
            try!(self.queue.wait());
        }
        Ok(())
    }

    /**
     * Indicates that a page is about to be modified. This should be called on a
     * page before any attempt to modify its contents. This marks the page dirty
     * (so that pageoutd knows to clean it before reclaiming the page frame)
     * and calls the dirtypage mmobj entry point.
     * The given page must not be busy.
     *
     * This routine can block at the mmobj operation level.
     */
    pub fn dirty(&self) -> Result<(),errno::Errno> {
        assert!(!self.is_busy());
        self.set_busy();
        let ret = self.obj.dirty_page(self);
        if let Ok(_) = ret { self.flags.set(self.flags.get() | pfstate::DIRTY); }
        self.clear_busy();
        ret
    }

    /**
     * Clean a dirty page by writing it back to disk. Removes the dirty
     * bit of the page and updates the MMU entry.
     * The page must be dirty but unpinned.
     *
     * This routine can block at the mmobj operation level.
     */
    pub fn clean(&self) -> Result<(), errno::Errno> {
        // TODO Not sure if this is enough
        assert!(self.is_dirty(), "attempt to clean a non-dirty page!");
        assert!(!self.is_pinned(), "we are trying to pin a cleaned page!");
        dbg!(debug::PFRAME, "cleaning {}", self);

        self.flags.set(self.flags.get() & !pfstate::DIRTY);
        /* Make sure a future write to the page will fault (and hence dirty it) */
        tlb::flush(self.page);
        self.obj.remove_from_tables(self);

        self.set_busy();
        let ret = self.obj.clean_page(self);
        if let Err(_) = ret {
            self.flags.set(self.flags.get() | pfstate::DIRTY);
        }
        self.clear_busy();
        return ret;
    }
}

impl Wait<PFState,()> for PFrame {
    fn wait(&self) -> Result<PFState, ()> {
        let out = queue.wait();
        if out.is_ok() {
            Ok(self.flags.get())
        } else {
            Err(())
        }
    }
}


impl Drop for PFrame {
    fn drop(&mut self) {
        assert!(!self.is_pinned());
        assert!(!self.is_busy());
        // TODO Not sure if this is good enough.
        dbg!(debug::PFRAME, "uncaching {}", self);
        // We have already been removed from pagetables.
        tlb::flush(self.page);
        page::free(self.page);
    }
}

impl Show for PFrame {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "PFrame {{ page: {}, pincount: {}, flags: {}, obj: {} }}",
               self.pagenum, self.pincount.load(SeqCst), self.flags.get(), self.obj)
    }
}
