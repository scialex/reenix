
//! The mmobj definitions.

use collections::TreeMap;
use mm::Allocation;
use drivers::DeviceId;

// Cheating to get a uuid by just incrementing a counter. This is not really good in general but we
// have 128 bits, which means we will probably never really run out...
// There has got to be a better way but this is just easier for now.
#[deriving(Copy, Eq, PartialEq, Show)]
pub struct MMObjId(DeviceId, u64);
static FAKE_DEVICE : DeviceId = DeviceId_static!(0xFF,0x00);
static mut NEXT_ID : MMObjId = MMObjId(FAKE_DEVICE,0);

impl MMObjId {
    pub fn unique() -> MMObjId {
        let out = unsafe { NEXT_ID };
        let MMObjId(dev, cnt) = out;
        unsafe {
            NEXT_ID = if cnt + 1 == 0 {
                MMObjId(DeviceId::new(dev.get_major(), dev.get_minor() + 1), 0)
            } else {
                MMObjId(dev, cnt + 1)
            };
        }
        out
    }
}

impl PartialOrd for MMObjId { fn partial_cmp(&self, other: &MMObjId) -> Option<Ordering> { Some(self.cmp(other)) } }
impl Ord for MMObjId {
    fn cmp(&self, other: &MMObjId) -> Ordering {
        let &MMObjId(mdev, mpiece) = self;
        let &MMObjId(odev, opiece) = other;
        match mdev.cmp(odev) {
            Equal => mpiece.cmp(opiece),
            Less => Less,
            Greater => Greater,
        }
    }
}

pub trait MMObj {
    fn get_id<'a>(&'a self) -> &'a MMObjId;

    /**
     * Get a pframe for the given page in this mmobj if there is one already present. This should
     * never allocate one and should return None if we don't already have the pframe.
     */
    // TODO I almost definitely need to wrap these somehow so they are pinned until we return
    fn get_resident<'a>(&self, pagenum : uint) -> Option<Rc<PFrame>> {
        // TODO
    }

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
    fn lookup_page(&self, pagenum: uint, writable: bool) -> KResult<Rc<Pframe>>;

    /**
     * Fill the given page frame with the data that should be in it.
     */
    fn fill_page(&self, pf: &mut PFrame) -> KResult<()>;

    /**
     * A hook; called when a request is made to dirty a non-dirty page.
     * Perform any necessary actions that must take place in order for it
     * to be possible to dirty (write to) the provided page. (For example,
     * if this page corresponds to a sparse block of a file that belongs to
     * an S5 filesystem, it would be necessary/desirable to allocate a
     * block in the fs before allowing a write to the block to proceed).
     * This may block.
     */
    fn dirty_page(&self, pf: &PFrame) -> KResult<()>;

    /**
     * Write the contents of the page frame starting at address
     * vp->vp_paddr to the page identified by vp->vp_obj and
     * vp->vp_pagenum.
     * This may block.
     * Return 0 on success and -errno otherwise.
     */
    fn clean_page(&self, pf: &PFrame) -> KResult<()>;
}

impl PartialOrd for MMObj { fn partial_cmp(&self, o: &MMObj) -> Option<Ordering> { self.get_id().partial_cmp(o.get_id()) } }
impl PartialEq  for MMObj { fn eq(&self, o: &MMObj) -> bool { self.get_id().eq(o.get_id()) } }
impl Ord        for MMObj { fn cmp(&self, o: &MMObj) -> Ordering { self.get_id().cmp(o.get_id()) } }
impl Eq         for MMObj {}
