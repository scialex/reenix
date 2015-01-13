
//! Pframes for Reenix.

use core::fmt;
use core::cell::*;
use procs::sync::*;
use util::cacheable::*;
use alloc::rc::*;
use alloc::boxed::*;
use core::prelude::*;
use base::errno::{self, KResult, Errno};
use base::make::*;
use libc::c_void;
use mm::{AllocError, page, tlb};
use mmobj::*;

pub use pframe::pfstate::PFState;
use util::pinnable_cache::{self, PinnableCache, InsertError, PinnedValue};

pub type PageNum = usize;

#[derive(PartialEq, Eq, PartialOrd, Ord, Show, Clone)]
pub struct PFrameId { mmobj: Rc<Box<MMObj + 'static>>, page: PageNum, }
impl PFrameId {
    /// Create a pframe id.
    pub fn new(mmo: Rc<Box<MMObj + 'static >>, page: PageNum) -> PFrameId { PFrameId { mmobj: mmo, page: page } }
}

impl Make<(Rc<Box<MMObj + 'static >>, PageNum)> for PFrameId {
    fn make(v: (Rc<Box<MMObj + 'static>>, PageNum)) -> PFrameId {
        PFrameId::new(v.0.clone(), v.1)
    }
}

static mut PFRAME_CACHE : *mut PinnableCache<PFrameId, PFrame> = 0 as *mut PinnableCache<PFrameId, PFrame>;

pub fn init_stage1() {
    // TODO
    // The allocator for the alloc_list
    pinnable_cache::request_pinnable_cache_allocator::<PFrameId, PFrame>("pframe pinnable cache");
}

pub fn init_stage2() {
    use core::mem::transmute;
    let pfcache : Box<PinnableCache<PFrameId, PFrame>> = box PinnableCache::new().unwrap();
    unsafe { PFRAME_CACHE = transmute(pfcache); }
    pageout::init_pageoutd();
}

pub fn init_stage3() {
    // TODO
}

/// Module holding the pageoutd stuff.
pub mod pageout {
    use libc::c_void;
    use procs::sync::*;
    use core::prelude::*;
    use super::get_cache;
    use alloc::boxed::*;
    use core::mem::transmute;

    pub fn init_pageoutd() {
        let pd : Box<PageOutD> = box PageOutD { queue: WQueue::new() };
        unsafe { PAGEOUTD = transmute(pd); }
    }

    /// the pageoutd's queue.
    struct PageOutD { pub queue: WQueue, }
    /// The pagetoutd
    static mut PAGEOUTD : *mut PageOutD = 0 as *mut PageOutD;
    /// Get the Pageoutd
    fn get_pageoutd() -> &'static PageOutD { unsafe { PAGEOUTD.as_ref().expect("pageoutd is null!") } }

    /// Wakeup the pageoutd.
    pub fn pageoutd_wakeup() { dbg!(debug::PCACHE, "pageoutd being signaled by {:?}", current_thread!()); get_pageoutd().queue.signal(); }
    pub extern "C" fn pageoutd_run(_: i32, _: *mut c_void) -> *mut c_void {
        // TODO This might be totally bad.
        while !(current_thread!()).cancelled {
            if let Err(_) = get_pageoutd().queue.wait() { break; }
            if (current_thread!()).cancelled { break; }
            dbg!(debug::PCACHE, "pageoutd woken up!");
            let removed = get_cache().clean_unpinned();
            dbg!(debug::PCACHE, "Removed {:?} items from page cache", removed);
            if removed == 0 {
                // TODO Should I do this?
                get_cache().clear_unpinned();
            }
        }
        0 as *mut c_void
    }
}

/// Get the pframe cache
fn get_cache() -> &'static mut PinnableCache<PFrameId, PFrame> {
    unsafe { PFRAME_CACHE.as_mut().expect("pframe cache should not be null") }
}

/// The different states a pframe can be in as it is being used.
pub mod pfstate {
    use core::fmt;
    use core::prelude::*;
    bitmask_create!(
        #[doc = "The different states a pframe can be in"]
        flags PFState : u8 {
            #[doc = "Nothing is being done to this pframe"]
            default NORMAL,
            #[doc = "this pframe is dirty, meaning it has been modified"]
            DIRTY   = 0,
            #[doc = "this pframe is busy, meaning it is currently being modified"]
            BUSY    = 1,
            #[doc = "this pframe is still being initialized. No pframe should ever have this state after being returned."]
            INITING = 2
        }
    );
}

/// A page frame structure used to manage memory in the kernel.
pub struct PFrame {
    /// A weak reference to the creating mmobj.
    obj     : Weak<Box<MMObj + 'static >>,

    /// The pagenumber of this page-frame.
    ///
    /// This and the object serve to uniquely identify this pframe.
    pagenum : PageNum,

    /// The actual memory this pageframe holds.
    page : *mut [u8; page::SIZE],

    /// The current state of the pframe, holding whether we are dirty and other data.
    flags : Cell<PFState>,

    /// Queue used to wait for the frame to stop being busy.
    queue : WQueue,
}

#[derive(Copy)]
pub enum PFError { Alloc(AllocError), Sys(errno::Errno), }
impl PFrame {
    /**
     * Get a pframe for the given page in this mmobj if there is one already present. This should
     * never allocate one and should return None if we don't already have the pframe.
     */
    pub fn get_resident(mmo: Rc<Box<MMObj + 'static>>, page_num: usize) -> Option<PinnedValue<'static, PFrameId, PFrame>> {
        get_cache().get(&PFrameId::new(mmo.clone(), page_num))
    }

    pub fn get(mmo: Rc<Box<MMObj + 'static>>, pagenum: usize) -> KResult<PinnedValue<'static, PFrameId, PFrame>> {
        let key = &PFrameId::new(mmo.clone(), pagenum);
        get_cache().add_or_get(key.clone()).map_err(|e| {
            match e {
                InsertError::MemoryError(_)     => { dbg!(debug::PFRAME, "Unable to add {:?} to cache, oom", key); errno::ENOMEM },
                InsertError::SysError(Some(er)) => { dbg!(debug::PFRAME, "unable to add {:?} to cache because of {:?}", key, er); er },
                InsertError::KeyPresent         => { kpanic!("illegal state of pframe cache, concurrency error!"); },
                _                               => { kpanic!("unknown SysError occured!"); }
            }
        })
    }

    /// Gets a mutable view of the page. This can ONLY be called from within an `MMObj`'s
    /// `fill_page` function since that is the only place where a mutable pframe can be obtained
    pub fn get_page_mut(&mut self) -> &mut [u8; page::SIZE] { unsafe { self.page.as_mut().expect("cannot be null") } }
    /// Gets a read only view of this page. Use `PFrame::dirty` to get a read-write view.
    pub fn get_page(&self) -> &[u8; page::SIZE] { unsafe { self.page.as_ref().expect("cannot be null") } }

    // TODO pframe_migrate?
    /// Makes a new pframe, also makes sure to allocate memory space for it.
    fn create(mmo : Rc<Box<MMObj + 'static>>, page_num: usize) -> Result<PFrame,PFError> {
        use pframe::PFError::*;
        Ok({
            let mut res = PFrame {
                obj : mmo.downgrade(),
                pagenum : page_num,

                page : try!(unsafe { page::alloc::<[u8; page::SIZE]>().map_err(|v| Alloc(v)) }),

                flags : Cell::new(pfstate::NORMAL | pfstate::INITING),
                queue : WQueue::new(),
            };
            try!(res.fill(&**mmo).map_err(|v| Sys(v)));
            res
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

    #[inline]
    fn set_busy(&self) {
        self.flags.set(self.flags.get() | pfstate::BUSY);
    }

    #[inline]
    fn clear_busy(&self) {
        self.flags.set(self.flags.get() & !pfstate::BUSY);
        self.queue.signal();
    }

    /// returns whether or not the pframe is marked as busy. If it is users should use `wait_busy`
    /// to wait for it to stop being busy.
    #[inline]
    pub fn is_busy(&self) -> bool { self.flags.get() & pfstate::BUSY != pfstate::NORMAL }
    /// Returns whether or not the pframe has been dirtied.
    #[inline]
    pub fn is_dirty(&self) -> bool { self.flags.get() & pfstate::DIRTY != pfstate::NORMAL }

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
    pub fn dirty(&self) -> Result<&mut [u8; page::SIZE],errno::Errno> {
        assert!(!self.is_busy());
        self.set_busy();
        match self.get_mmo().dirty_page(self) {
            Ok(_) => {
                self.flags.set(self.flags.get() | pfstate::DIRTY);
                self.clear_busy();
                unsafe { Ok(self.page.as_mut().expect("pframe cannot have an empty page")) }
            },
            Err(e) => {
                self.clear_busy();
                Err(e)
            }
        }
    }


    #[inline]
    pub fn get_pagenum(&self) -> PageNum { self.pagenum }

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
        dbg!(debug::PFRAME, "cleaning {:?}", self);

        self.flags.set(self.flags.get() & !pfstate::DIRTY);
        /* Make sure a future write to the page will fault (and hence dirty it) */
        unsafe { tlb::flush(self.page as *mut c_void) };
        self.remove_from_pts();

        self.set_busy();
        let ret = self.get_mmo().clean_page(self);
        if let Err(_) = ret {
            self.flags.set(self.flags.get() | pfstate::DIRTY);
        }
        self.clear_busy();
        return ret;
    }

    fn get_mmo(&self) -> Rc<Box<MMObj + 'static>> { self.obj.upgrade().expect("mmobj shouldn't be destroyed while pframes still present") }

    /// Remove this pframe from the page frame tables of all the procs it is loaded in.
    fn remove_from_pts(&self) {
        // TODO figure out how to do this.
        kpanic!("not yet implemented remove from pts called");
    }
}

impl Cacheable for PFrame {
    fn is_still_useful(&self) -> bool {
        // TODO Fix this.
        //self.get_mmo().deref().is_still_useful() || self.is_dirty()
        self.is_dirty()
    }
}

#[doc(hidden)]
impl TryMake<PFrameId, Errno> for PFrame {
    fn try_make(a: PFrameId) -> Result<PFrame, Errno> {
        let PFrameId { mmobj, page } = a.clone();
        PFrame::create(mmobj, page).map_err(|e| {
            match e {
                PFError::Alloc(_) => { dbg!(debug::PFRAME, "Unable to allocate memory for {:?}", a); Errno::ENOMEM },
                PFError::Sys(e)   => { dbg!(debug::PFRAME, "unable to create {:?} because of {:?}", a, e); e },
            }
        })
    }
}

impl Wait<PFState,()> for PFrame {
    fn wait(&self) -> Result<PFState, ()> {
        let out = self.queue.wait();
        if out.is_ok() {
            Ok(self.flags.get())
        } else {
            Err(())
        }
    }
}

#[unsafe_destructor]
impl Drop for PFrame {
    fn drop(&mut self) {
        assert!(!self.is_busy());
        // TODO Not sure if this is good enough.
        dbg!(debug::PFRAME, "uncaching {:?}", self);
        // We have already been removed from pagetables.
        unsafe { tlb::flush(self.page as *mut c_void) };
        unsafe { page::free(self.page as *mut c_void) };
    }
}

impl fmt::Show for PFrame {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "PFrame {{ page: {}, flags: {:?}, obj: {:?} }}",
               self.pagenum, self.flags.get(), self.get_mmo())
    }
}
