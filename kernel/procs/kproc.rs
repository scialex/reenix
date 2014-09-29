// TODO Copyright Header

use core::prelude::*;
use collections::string::String;
use collections::vec::Vec;
use collections::bitv::Bitv;
use collections::treemap::TreeMap;
use alloc::rc::{Rc,Weak};
use core::mem;
use core::cell::*;

#[deriving(Clone, Eq, PartialEq, Show, PartialOrd, Ord)]
pub struct ProcId(uint);


extern "rust-intrinsic" {
    fn offset<T>(dst: *const T, offset: int) -> *const T;
}
#[no_mangle]
pub unsafe extern "C" fn memmove(dest: *mut u8, src: *const u8,
                                 n: uint) -> *mut u8 {
    if src < dest as *const u8 { // copy from end
        let mut i = n;
        while i != 0 {
            i -= 1;
            *(offset(dest as *const u8, i as int) as *mut u8) =
                *offset(src, i as int);
        }
    } else { // copy from beginning
        let mut i = 0;
        while i < n {
            *(offset(dest as *const u8, i as int) as *mut u8) =
                *offset(src, i as int);
            i += 1;
        }
    }
    return dest;
}

static INIT_PID : ProcId = ProcId(1);

static mut PID_BITV : *mut Bitv = 0 as *mut Bitv;

impl ProcId {
    pub fn new() -> Option<ProcId> {
        None
    }
}

pub enum ProcState { Running, Dead }
pub type ProcStatus = int;

pub struct KProc {
    pid      : ProcId,                      /* Our pid */
    command  : String,                      /* Process Name */
    //threads  : Vec<Rc<kthread::KThread>>,   /* Our threads */
    children : TreeMap<ProcId, Rc<RefCell<KProc>>>, /* Our children */
    status   : ProcStatus,                  /* Our exit status */
    state    : ProcState,                   /* running/sleeping/etc. */
    parent   : Weak<RefCell<KProc>>,        /* Our parent */

    // TODO ktqueue.
    //wait : WaitQueue,

    // TODO For VFS
    // files : [Option<KFile>, ..NFILES],
    // cwd   : RC<VNode>,

    // TODO For VM
    // brk : uint,
    // start_brk : uint,
    // vmmap : Vec<VMArea>,
}

static mut current_proc : *mut KProc = 0 as *mut KProc;

pub fn init_stage1() {
    use mm::alloc::request_slab_allocator;
    use core::intrinsics::size_of;
    request_slab_allocator("RefCell<KProc> allocator", unsafe { size_of::<RefCell<KProc>>() as u32 });
}

pub fn init_stage2() {
    use core::intrinsics::transmute;
    unsafe {
        let x = box Bitv::with_capacity(100, false);
        PID_BITV = transmute(&*x);
        mem::forget(x);
    }
}
