
//! The mmobj definitions.

use core::fmt;
use core::cell::*;
use core::ptr::*;
use core::prelude::*;
use util::cacheable::*;
use base::errno::*;
use pframe;
use alloc::rc::*;
use util::pinnable_cache::*;
use alloc::boxed::*;
use base::devices::*;
use traits::VNode;

// Cheating to get a uuid by just incrementing a counter. This is not really good in general but we
// have 48 bits, which means we will probably never really run out...
// There has got to be a better way but this is just easier for now.
#[deriving(Copy, Eq, PartialEq, Show)]
pub struct MMObjId(DeviceId, u32);
const FAKE_DEVICE : DeviceId = DeviceId_static!(0xFF,0x00);
static mut NEXT_ID : MMObjId = MMObjId(FAKE_DEVICE,0);

impl MMObjId {
    pub fn new(dev: DeviceId, n: u32) -> MMObjId { MMObjId(dev, n) }
}

impl PartialOrd for MMObjId { fn partial_cmp(&self, other: &MMObjId) -> Option<Ordering> { Some(self.cmp(other)) } }
impl Ord for MMObjId {
    fn cmp(&self, other: &MMObjId) -> Ordering {
        let &MMObjId(mdev, mpiece) = self;
        let &MMObjId(odev, opiece) = other;
        match mdev.cmp(&odev) {
            Equal => mpiece.cmp(&opiece),
            Less => Less,
            Greater => Greater,
        }
    }
}

/// An mmobj that needs interior mutability. This is used just like a regular mmobj through the use
/// of cells.
pub trait MMObjMut {
    /// Return an MMObjId for this object.
    fn get_id(&self) -> MMObjId;

    /**
     * Fill the given page frame with the data that should be in it.
     */
    fn fill_page(&mut self, pf: &mut pframe::PFrame) -> KResult<()>;

    /**
     * A hook; called when a request is made to dirty a non-dirty page.
     * Perform any necessary actions that must take place in order for it
     * to be possible to dirty (write to) the provided page. (For example,
     * if this page corresponds to a sparse block of a file that belongs to
     * an S5 filesystem, it would be necessary/desirable to allocate a
     * block in the fs before allowing a write to the block to proceed).
     * This may block.
     */
    fn dirty_page(&mut self, pf: &pframe::PFrame) -> KResult<()>;

    /**
     * Write the contents of the page frame starting at address
     * vp->vp_paddr to the page identified by vp->vp_obj and
     * vp->vp_pagenum.
     * This may block.
     * Return 0 on success and -errno otherwise.
     */
    fn clean_page(&mut self, pf: &pframe::PFrame) -> KResult<()>;

    fn show(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "mmobj_mut for {}", self.get_id()) }
}

pub trait MMObj {
    /// Return an MMObjId for this object.
    fn get_id(&self) -> MMObjId;

    /**
     * Finds the correct page frame from a high-level perspective
     * for performing the given operation on an area backed by
     * the given pagenum of the given object. If "forwrite" is
     * specified then the pframe should be suitable for writing;
     * otherwise, it is permitted not to support writes. In
     * either case, it must correctly support reads.
     *
     * Most objects will simply return a page from their
     * own list of pages, but objects such as shadow objects
     * may need to perform more complicated operations to find
     * the appropriate page.
     * This may block.
     */
    // TODO This isn't the best interface Maybe a holder that will unpin when we leave, might be
    // better. Using this stuff is annoying.
    fn lookup_page(this: Rc<T>, pagenum: uint, _writable: bool) -> KResult<PinnedValue<'static, pframe::PFrameId, pframe::PFrame>> {
        pframe::PFrame::get(this, pagenum)
    }

    /**
     * Fill the given page frame with the data that should be in it.
     */
    fn fill_page(&self, pf: &mut pframe::PFrame) -> KResult<()>;

    /**
     * A hook; called when a request is made to dirty a non-dirty page.
     * Perform any necessary actions that must take place in order for it
     * to be possible to dirty (write to) the provided page. (For example,
     * if this page corresponds to a sparse block of a file that belongs to
     * an S5 filesystem, it would be necessary/desirable to allocate a
     * block in the fs before allowing a write to the block to proceed).
     * This may block.
     */
    fn dirty_page(&self, pf: &pframe::PFrame) -> KResult<()>;

    /**
     * Write the contents of the page frame starting at address
     * vp->vp_paddr to the page identified by vp->vp_obj and
     * vp->vp_pagenum.
     * This may block.
     * Return 0 on success and -errno otherwise.
     */
    fn clean_page(&self, pf: &pframe::PFrame) -> KResult<()>;

    fn show(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "mmobj for {}", self.get_id()) }
}

impl<'a> fmt::Show  for MMObj + 'a { fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { self.show(f) } }
impl<'a> PartialOrd for MMObj + 'a { fn partial_cmp(&self, o: &MMObj) -> Option<Ordering> { self.get_id().partial_cmp(&o.get_id()) } }
impl<'a> PartialEq  for MMObj + 'a { fn eq(&self, o: &MMObj) -> bool { self.get_id().eq(&o.get_id()) } }
impl<'a> Ord        for MMObj + 'a { fn cmp(&self, o: &MMObj) -> Ordering { self.get_id().cmp(&o.get_id()) } }
impl<'a> Eq         for MMObj + 'a {}


// TODO I might want to replace this with a trait that just lets us do the deref, that would let us
// keep more safety.
impl<T> MMObj for UnsafeCell<T> where T: MMObjMut {
    fn get_id(&self) -> MMObjId { unsafe { self.get().as_ref().expect("cannot be null") }.get_id() }
    fn fill_page(&self, pf: &mut pframe::PFrame) -> KResult<()> { unsafe { self.get().as_mut().expect("cannot be null") }.fill_page(pf) }
    fn dirty_page(&self, pf: &pframe::PFrame) -> KResult<()> { unsafe { self.get().as_mut().expect("cannot be null") }.dirty_page(pf) }
    fn clean_page(&self, pf: &pframe::PFrame) -> KResult<()> { unsafe { self.get().as_mut().expect("cannot be null") }.clean_page(pf) }
    fn show(&self, f: &mut fmt::Formatter) -> fmt::Result { unsafe { self.get().as_ref().expect("cannot be null") }.show(f) }
}
