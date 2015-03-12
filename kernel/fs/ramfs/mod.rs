
//! The RAMFS

use FileSystem;
use InodeNum;
use vnode::{self, VNode, Stat, DirEnt};
use mm::page;
use umem::pframe::*;
use base::errno::{self, KResult};
use base::devices::DeviceId;
use std::sync::atomic::AtomicUint;
use std::sync::atomic::Ordering::{SeqCst, Relaxed};
use std::{slice, mem, fmt};
use std::cell::*;
use base::cell::*;
use std::slice::bytes::copy_memory;
use umem::mmobj::{MMObjId, MMObj};
use std::cmp::min;
use std::rc::*;
use std::borrow::*;
use procs::sync::Mutex;
use std::collections::HashMap;
use base::errno::Errno;

use self::RVNode::*;

/// The FS to use for a ramfs
static mut FS : Option<*mut RamFS> = None;

/// The deviceid for ramfs
pub const RAMFS_DEVID : DeviceId = DeviceId_static!(4,0);

/// The max name length of a directory entry.
pub const NAME_LEN : usize = 28;

/// The number of files we will allow.
pub const MAX_INODES : usize = 128;

const MAX_FILE_LEN : usize = page::SIZE;

#[derive(Clone, Debug)]
pub enum RVNode {
    Byte(Rc<ByteInode>),
    Block(Rc<BlockInode>),
    Regular(Rc<RegInode>),
    Directory(Rc<DirInode>),
}

impl RVNode {
    fn get_inner(&self) -> &VNode<Res=RVNode> {
        match *self { Byte(i) => &*i, Block(i) => &*i, Regular(i) => &*i, Directory(i) => &*i, }
    }
}

impl MMObj for RVNode {
    fn get_id(&self) -> MMObjId { MMObjId::new(RAMFS_DEVID, self.get_number()) }
    fn fill_page(&self,   pf: &mut PFrame) -> KResult<()> { dbg!(debug::VFS, "ramfs vnode used as mmobj!"); Err(errno::ENOTSUP) }
    fn dirty_page(&self,  pf: &PFrame)     -> KResult<()> { dbg!(debug::VFS, "ramfs vnode used as mmobj!"); Err(errno::ENOTSUP) }
    fn clean_page(&self,  pf: &PFrame)     -> KResult<()> { dbg!(debug::VFS, "ramfs vnode used as mmobj!"); Err(errno::ENOTSUP) }
    // TODO The next two maybe should panic?
    //fn dirty_page(&self, _pf: &PFrame)      -> KResult<()> { Ok(()) }
    //fn clean_page(&self, _pf: &PFrame)      -> KResult<()> { Ok(()) }
}

impl VNode for RVNode {
    type Res = RVNode;
    fn get_mode(&self) -> vnode::Mode {
        match *self {
            Byte(_) => vnode::CharDev,
            Block(_) => vnode::BlockDev,
            Regular(_) => vnode::Regular,
            Directory(_) => vnode::Directory,
        }
    }

    fn get_number(&self) -> InodeNum { self.get_inner().get_number() }
    fn stat(&self) -> KResult<Stat> { self.get_inner().stat() }
    fn len(&self) -> KResult<usize> { self.get_inner().len() }
    fn read(&self, off: usize, buf: &mut [u8]) -> KResult<usize> { self.get_inner().read(off, buf) }
    fn write(&self, off: usize, buf: &[u8]) -> KResult<usize> { self.get_inner().write(off, buf) }
    fn truncate(&self, size: usize) -> KResult<usize> { self.get_inner().truncate(size) }

    // TODO Figure out the contract for mmap.
    //fn mmap(&self, ...) -> KResult<?> { Err(errno::EINVAL) }

    fn create(&self, name: &str) -> KResult<RVNode> { self.get_inner().create(name) }
    fn lookup(&self, name: &str) -> KResult<RVNode> { self.get_inner().lookup(name) }
    fn mknod(&self, name: &str, devid: DeviceId) -> KResult<()> { self.get_inner().mknod(name, devid) }

    // TODO Maybe this should be &Self for from...
    fn link(&self, from: &RVNode, to: &str) -> KResult<()> { self.get_inner().link(from, to) }
    fn unlink(&self, to: &str) -> KResult<()> { self.get_inner().unlink(to) }
    fn mkdir(&self, to: &str) -> KResult<()> { self.get_inner().mkdir(to) }
    fn rmdir(&self, to: &str) -> KResult<()> { self.get_inner().rmdir(to) }
    /// Given offset into directory returns the size of the dirent in the directory structure and
    /// the given dirent. If it returns EOK then we have read the whole directory. To read the next
    /// entry add the returned length to the offset.
    fn readdir(&self, off: usize) -> KResult<(usize, DirEnt)> { self.get_inner().readdir(off) }
}

#[derive(Clone, Debug)]
pub struct ByteInode {
    fs: Rc<RamFS>,
    num: InodeNum,
    dev: DeviceId,
}
impl ByteInode {
    fn new(num: InodeNum, dev: DeviceId, fs: Rc<RamFS>) -> ByteInode {
        ByteInode { num: num, fs: fs, dev: dev }
    }
}
impl VNode for ByteInode {
    type Res = RVNode;
    fn get_mode(&self) -> vnode::Mode { vnode::CharDev }
    fn get_number(&self) -> InodeNum { self.num }
}

#[derive(Clone, Debug)]
pub struct BlockInode {
    fs: Rc<RamFS>,
    num: InodeNum,
    dev: DeviceId,
}
impl BlockInode {
    fn new(num: InodeNum, dev: DeviceId, fs: Rc<RamFS>) -> BlockInode {
        BlockInode { num: num, fs: fs, dev: dev }
    }
}
impl VNode for BlockInode {
    type Res = RVNode;
    fn get_mode(&self) -> vnode::Mode { vnode::BlockDev }
    fn get_number(&self) -> InodeNum { self.num }
}

pub struct RegInode {
    num: InodeNum,
    fs: Rc<RamFS>,
    data: SafeCell<Box<[u8;MAX_FILE_LEN]>>,
    len: Cell<usize>,
}

impl fmt::Debug for RegInode {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "RegInode {{ num: {}, fs: {:?}, len: {} }}", self.num, self.fs, self.len.get())
    }
}

impl VNode for RegInode {
    type Res = RVNode;
    fn get_mode(&self) -> vnode::Mode { vnode::Regular }
    fn get_number(&self) -> InodeNum { self.num }
    fn len(&self) -> KResult<usize> { self.len() }
    fn read(&self, off: usize, buf: &mut [u8]) -> KResult<usize> {
        let len = try!(self.len());
        if off > len { Err(errno::EFBIG) } else {
            let end = min(off + buf.len(), len);
            copy_memory(buf, &self.data.get_ref()[off..end]);
            Ok(end - off)
        }
    }
    fn write(&self, off: usize, mut buf: &[u8]) -> KResult<usize> {
        use std::slice::{bytes, from_raw_mut_buf};
        let len = try!(self.len());
        if off >= MAX_FILE_LEN { return Err(errno::ENOSPC); }
        if off > len { self.fill_zeros(off); }
        let buf = &buf[0..min(buf.len(), MAX_FILE_LEN - off)];
        let data = self.data.get_mut();
        copy_memory(&mut data[off..], buf);
        Ok(buf.len())
    }
    fn stat(&self) -> KResult<Stat> {
        // TODO
        kpanic!("not implemented");
    }
}

impl RegInode {
    fn new(num: InodeNum, fs: Rc<RamFS>) -> RegInode {
        RegInode { num: num, fs: fs, data: SafeCell::new(box [0;MAX_FILE_LEN]), len: Cell::new(0) }
    }
    fn fill_zeros(&self, end: usize) {
        bassert!(end < MAX_FILE_LEN);
        let data = self.data.get_mut();
        let len = self.len().unwrap();
        for i in len..end {
            data[i] = 0;
        }
        self.len.set(end);
    }
}

pub struct DirInode {
    num: InodeNum,
    fs: Rc<RamFS>,
    data: Mutex<HashMap<String, RVNode>>,
}

impl fmt::Debug for DirInode {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        // TODO Have it print contents?
        write!(f, "DirInode {{ num: {}, fs: {:?} }}", self.num, self.fs)
    }
}


impl DirInode {
    fn new(num: InodeNum, fs: Rc<RamFS>) -> DirInode {
        DirInode { num: num, data: Mutex::new("dir inode mutex", HashMap::new()), fs: fs }
    }
}

impl VNode for DirInode {
    type Res = RVNode;

    fn get_mode(&self) -> vnode::Mode { vnode::Directory }
    fn get_number(&self) -> InodeNum { self.num }
    fn len(&self) -> KResult<usize> { let c = try!(self.data.lock().map_err(|_| errno::EDEADLK)); Ok(c.len() + 1) }

    fn stat(&self) -> KResult<Stat> {
        // TODO
        kpanic!("not implemented");
    }

    fn create(&self, name: &str) -> KResult<RVNode> {
        if name == "." { return Err(errno::EEXIST); }
        let l = try!(self.data.lock().map_err(|_| errno::EDEADLK));
        let d = &*l;
        if d.contains_key(name) { return Err(errno::EEXIST); }
        let new_node = try!(self.fs.alloc_reg());
        d.insert(name.to_owned(), new_node.clone());
        Ok(new_node)
    }

    fn link(&self, from: &RVNode, name: &str ) -> KResult<()> {
        if from.get_mode() & vnode::Directory != vnode::Unused { return Err(errno::EISDIR); }
        if name == "." { return Err(errno::EEXIST); }
        let l = try!(self.data.lock().map_err(|_| errno::EDEADLK));
        let d = &mut *l;
        if d.contains_key(name) { return Err(errno::EEXIST); }
        d.insert(name.to_owned(), from.clone());
        return Ok(());
    }

    fn mkdir(&self, name: &str) -> KResult<()> {
        if name == "." { return Err(errno::EEXIST); }
        let l = try!(self.data.lock().map_err(|_| errno::EDEADLK));
        let d = &mut *l;
        if d.contains_key(name) { return Err(errno::EEXIST); }
        let new_node = try!(self.fs.alloc_dir());
        // TODO Link ..
        d.insert(name.to_owned(), new_node);
        Ok(())
    }

    fn mknod(&self, name: &str, devid: DeviceId) -> KResult<()> {
        // TODO How to tell byte and block apart.
        not_yet_implemented!("mknod");
        Err(errno::ENOTSUP)
    }

    fn lookup(&self, name: &str) -> KResult<RVNode> {
        // TODO WRONG
        if name == "." { return self.fs.get_vnode(self.get_number()); }
        let d = try!(self.data.lock().map_err(|_| errno::EDEADLK));
        d.get(name).map(|x| x.clone()).ok_or(errno::ENOENT)
    }

    fn readdir(&self, off: usize) -> KResult<(usize, DirEnt)> {
        // TODO
    }
}

pub struct RamFS {
    inodes: Mutex<[Option<RVNode>; MAX_INODES]>,
    root_dir: Option<RVNode>,
}

impl fmt::Debug for RamFS {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(),fmt::Error> {
        write!(f, "RamFS {{ ... }}")
    }
}

impl RamFS {
    fn alloc_reg(&self) -> KResult<RVNode> {
        // TODO
    }
    fn alloc_dir(&self) -> KResult<RVNode> {
        // TODO
    }
    fn get_vnode(&self, num: InodeNum) -> KResult<RVNode> {
        // TODO
    }
    fn create() -> Rc<RamFS> {
        let mut out = RamFS { inodes: Mutex::new("ramfs mutex", inds), root_dir: None }
        let mut inds = [None; MAX_INODES];

        let root = RVNode::Directory(Rc::new)
    }
}

