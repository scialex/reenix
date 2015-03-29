
//! The RAMFS


use FileSystem;
use InodeNum;
use base::cell::*;
use base::devices::DeviceId;
use base::errno::{self, KResult};
use mm::alloc::request_rc_slab_allocator;
use mm::page;
use procs::sync::Mutex;
use std::borrow::*;
use std::cell::*;
use std::cmp::min;
use std::collections::HashMap;
use std::mem::{transmute, size_of};
use std::rc::*;
use std::slice::bytes::copy_memory;
use std::{mem, fmt};
use umem::mmobj::{MMObjId, MMObj};
use umem::pframe::PFrame;
use vnode::{self, VNode, Stat, DirEnt};

/// The FS to use for a ramfs
static mut FS : *mut RamFS = 0 as *mut RamFS;

/// The deviceid for ramfs
pub const RAMFS_DEVID : DeviceId = DeviceId_static!(4,0);

/// The max name length of a directory entry.
pub const NAME_LEN : usize = 28;

/// The number of files we will allow.
pub const MAX_INODES : InodeNum = 128;
pub const ROOT_INODE_NUM : InodeNum = MAX_INODES - 1;

const MAX_FILE_LEN : usize = page::SIZE;

pub fn init_stage1() {
    // Rc's have some overhead.
    request_rc_slab_allocator("RVNode", size_of::<RVNode>() as u32);
}

pub fn init_stage2() { }
pub fn init_stage3() {
    unsafe {
        let fs : Box<RamFS> = box mem::uninitialized();
        RamFS::initialize(mem::transmute(&fs));
        FS = mem::transmute(fs);
    }
}

pub fn shutdown() {
}

#[derive(Debug)]
pub enum RVNode {
    Byte(ByteInode),
    Block(BlockInode),
    Regular(RegInode),
    Directory(DirInode),
}

impl RVNode {
    fn get_inner(&self) -> &VNode<Real=RVNode, Res=Rc<RVNode>> {
        use self::RVNode::*;
        match *self { Byte(ref i) => i, Block(ref i) => i, Regular(ref i) => i, Directory(ref i) => i, }
    }
}

impl MMObj for RVNode {
    fn get_id(&self) -> MMObjId { MMObjId::new(RAMFS_DEVID, self.get_number() as u32) }
    fn fill_page(&self,   _pf: &mut PFrame) -> KResult<()> { dbg!(debug::VFS, "ramfs vnode used as mmobj!"); Err(errno::ENOTSUP) }
    fn dirty_page(&self,  _pf: &PFrame)     -> KResult<()> { dbg!(debug::VFS, "ramfs vnode used as mmobj!"); Err(errno::ENOTSUP) }
    fn clean_page(&self,  _pf: &PFrame)     -> KResult<()> { dbg!(debug::VFS, "ramfs vnode used as mmobj!"); Err(errno::ENOTSUP) }
    // TODO The next two maybe should panic?
    //fn dirty_page(&self, _pf: &PFrame)      -> KResult<()> { Ok(()) }
    //fn clean_page(&self, _pf: &PFrame)      -> KResult<()> { Ok(()) }
}

impl VNode for RVNode {
    type Real = RVNode;
    type Res = Rc<RVNode>;
    fn get_mode(&self) -> vnode::Mode {
        use self::RVNode::*;
        match *self {
            Byte(_) => vnode::CharDev,
            Block(_) => vnode::BlockDev,
            Regular(_) => vnode::Regular,
            Directory(_) => vnode::Directory,
        }
    }

    fn get_fs(&self) -> &FileSystem<Real=RVNode, Node=Rc<RVNode>> { self.get_inner().get_fs() }
    fn get_number(&self) -> InodeNum { self.get_inner().get_number() }
    fn stat(&self) -> KResult<Stat> { self.get_inner().stat() }
    fn len(&self) -> KResult<usize> { self.get_inner().len() }
    fn read(&self, off: usize, buf: &mut [u8]) -> KResult<usize> { self.get_inner().read(off, buf) }
    fn write(&self, off: usize, buf: &[u8]) -> KResult<usize> { self.get_inner().write(off, buf) }
    fn truncate(&self, size: usize) -> KResult<usize> { self.get_inner().truncate(size) }

    // TODO Figure out the contract for mmap.
    //fn mmap(&self, ...) -> KResult<?> { Err(errno::EINVAL) }

    fn create(&self, name: &str) -> KResult<Rc<RVNode>> { self.get_inner().create(name) }
    fn lookup(&self, name: &str) -> KResult<Rc<RVNode>> { self.get_inner().lookup(name) }
    fn mknod(&self, name: &str, devid: DeviceId) -> KResult<()> { self.get_inner().mknod(name, devid) }

    // TODO Maybe this should be &Self for from...
    fn link(&self, from: &Rc<RVNode>, to: &str) -> KResult<()> { self.get_inner().link(from, to) }
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
    fs: &'static RamFS,
    num: InodeNum,
    dev: DeviceId,
}
impl ByteInode {
    fn new(num: InodeNum, dev: DeviceId, fs: &'static RamFS) -> ByteInode {
        ByteInode { num: num, fs: fs, dev: dev }
    }
}
impl VNode for ByteInode {
    type Real = RVNode;
    type Res = Rc<RVNode>;
    fn get_fs(&self) -> &FileSystem<Real=RVNode, Node=Rc<RVNode>> { self.fs }
    fn get_mode(&self) -> vnode::Mode { vnode::CharDev }
    fn get_number(&self) -> InodeNum { self.num }
}

#[derive(Clone, Debug)]
pub struct BlockInode {
    fs: &'static RamFS,
    num: InodeNum,
    dev: DeviceId,
}
impl BlockInode {
    fn new(num: InodeNum, dev: DeviceId, fs: &'static RamFS) -> BlockInode {
        BlockInode { num: num, fs: fs, dev: dev }
    }
}
impl VNode for BlockInode {
    type Real = RVNode;
    type Res = Rc<RVNode>;
    fn get_fs(&self) -> &FileSystem<Real=RVNode, Node=Rc<RVNode>> { self.fs }
    fn get_mode(&self) -> vnode::Mode { vnode::BlockDev }
    fn get_number(&self) -> InodeNum { self.num }
}

pub struct RegInode {
    num: InodeNum,
    fs: &'static RamFS,
    data: SafeCell<Box<[u8;MAX_FILE_LEN]>>,
    len: Cell<usize>,
}

impl fmt::Debug for RegInode {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "RegInode {{ num: {}, fs: {:?}, len: {} }}", self.num, self.fs, self.len.get())
    }
}

impl VNode for RegInode {
    type Real = RVNode;
    type Res = Rc<RVNode>;
    fn get_fs(&self) -> &FileSystem<Real=RVNode, Node=Rc<RVNode>> { self.fs }
    fn get_mode(&self) -> vnode::Mode { vnode::Regular }
    fn get_number(&self) -> InodeNum { self.num }
    fn len(&self) -> KResult<usize> { Ok(self.len.get()) }
    fn read(&self, off: usize, buf: &mut [u8]) -> KResult<usize> {
        let len = try!(self.len());
        if off > len { Err(errno::EFBIG) } else {
            let end = min(off + buf.len(), len);
            copy_memory(buf, &self.data.get_ref()[off..end]);
            Ok(end - off)
        }
    }
    fn write(&self, off: usize, buf: &[u8]) -> KResult<usize> {
        let len = try!(self.len());
        if off >= MAX_FILE_LEN { return Err(errno::ENOSPC); }
        if off > len { self.fill_zeros(off); }
        let buf = &buf[0..min(buf.len(), MAX_FILE_LEN - off)];
        let mut data = self.data.get_mut();
        copy_memory(&mut data[off..], buf);
        Ok(buf.len())
    }
    fn stat(&self) -> KResult<Stat> {
        // TODO
        kpanic!("not implemented");
    }
}

impl RegInode {
    fn new(num: InodeNum, fs: &'static RamFS) -> RegInode {
        RegInode { num: num, fs: fs, data: SafeCell::new(box [0;MAX_FILE_LEN]), len: Cell::new(0) }
    }
    fn fill_zeros(&self, end: usize) {
        bassert!(end < MAX_FILE_LEN);
        let mut data = self.data.get_mut();
        let len = self.len().unwrap();
        for i in len..end {
            data[i] = 0;
        }
        self.len.set(end);
    }
}

pub struct DirInode {
    num: InodeNum,
    fs: &'static RamFS,
    parent: Option<InodeNum>,
    data: Mutex<HashMap<String, Rc<RVNode>>>,
}

impl fmt::Debug for DirInode {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        // TODO Have it print contents?
        write!(f, "DirInode {{ num: {}, fs: {:?} }}", self.num, self.fs)
    }
}


impl DirInode {
    fn new(num: InodeNum, parent: Option<InodeNum>, fs: &'static RamFS) -> DirInode {
        DirInode { num: num, parent: parent, data: Mutex::new("dir inode mutex", HashMap::new()), fs: fs }
    }
}

impl VNode for DirInode {
    type Real = RVNode;
    type Res = Rc<RVNode>;

    fn get_fs(&self) -> &FileSystem<Real=RVNode, Node=Rc<RVNode>> { self.fs }
    fn get_mode(&self) -> vnode::Mode { vnode::Directory }
    fn get_number(&self) -> InodeNum { self.num }
    fn len(&self) -> KResult<usize> { let c = try!(self.data.lock().map_err(|_| errno::EDEADLK)); Ok(c.len() + 2) }

    fn stat(&self) -> KResult<Stat> {
        // TODO
        kpanic!("not implemented");
    }

    fn create(&self, name: &str) -> KResult<Rc<RVNode>> {
        if name == "." { return Err(errno::EEXIST); }
        let mut l = try!(self.data.lock().map_err(|_| errno::EDEADLK));
        let mut d = &mut *l;
        if d.contains_key(name) { return Err(errno::EEXIST); }
        let new_node = dbg_try!(self.fs.alloc_reg(), debug::VFS, "Unable to get new file inode for file {}", name);
        d.insert(name.to_owned(), new_node.clone());
        Ok(new_node)
    }

    fn link(&self, from: &Rc<RVNode>, name: &str ) -> KResult<()> {
        if from.get_mode() & vnode::Directory != vnode::Unused { return Err(errno::EISDIR); }
        if name == "." { return Err(errno::EEXIST); }
        let mut l = try!(self.data.lock().map_err(|_| errno::EDEADLK));
        let d = &mut *l;
        if d.contains_key(name) {
            dbg!(debug::VFS, "Could not create {} in {} because another vnode has that name", name, self);
            return Err(errno::EEXIST);
        }
        d.insert(name.to_owned(), from.clone());
        return Ok(());
    }

    fn mkdir(&self, name: &str) -> KResult<()> {
        if name == "." { return Err(errno::EEXIST); }
        let mut l = try!(self.data.lock().map_err(|_| errno::EDEADLK));
        let mut d = &mut *l;
        if d.contains_key(name) {
            dbg!(debug::VFS, "Could not mkdir {} in {} because another vnode has that name", name, self);
            return Err(errno::EEXIST);
        }
        let new_node = dbg_try!(self.fs.alloc_dir(self.get_number()),
                                debug::VFS, "Unable to create directory node for {} in {}", name, self);
        // TODO Link ..
        d.insert(name.to_owned(), new_node);
        Ok(())
    }

    fn mknod(&self, _name: &str, _devid: DeviceId) -> KResult<()> {
        // TODO How to tell byte and block apart.
        not_yet_implemented!("mknod");
        Err(errno::ENOTSUP)
    }

    fn lookup(&self, name: &str) -> KResult<Rc<RVNode>> {
        match name {
            "." => self.fs.get_vnode(self.get_number()),
            ".." => self.fs.get_vnode(self.parent.unwrap_or(self.get_number())),
            _ => {
                let d = try!(self.data.lock().map_err(|_| errno::EDEADLK));
                d.get(name).map(|x| x.clone()).ok_or(errno::ENOENT)
            }
        }
    }

    fn readdir(&self, off: usize) -> KResult<(usize, DirEnt)> {
        let l = try!(self.data.lock().map_err(|_| errno::EDEADLK));
        if off >= try!(self.len()) {
            Err(errno::EOK)
        } else if off == 0 {
            Ok((1, DirEnt { inode: self.get_number(), offset: off + 1, name: ".".to_string() }))
        } else if off == 1 {
            Ok((1, DirEnt { inode: self.parent.unwrap_or(self.get_number()), offset: off + 1, name: "..".to_string() }))
        } else if let Some((name, vn)) = l.iter().nth(off - 2) {
            Ok((1, DirEnt { inode: vn.get_number(), offset: off + 1, name: name.clone() }))
        } else {
            Err(errno::EOK)
        }
    }
}

pub struct RamFS {
    inodes: Mutex<[Option<Weak<RVNode>>; MAX_INODES - 1]>,
    root_dir: Option<Rc<RVNode>>,
    last: Cell<usize>,
}

impl fmt::Debug for RamFS {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(),fmt::Error> {
        write!(f, "RamFS {{ ... }}")
    }
}

impl RamFS {
    fn get_inode(&self) -> KResult<InodeNum> {
        let l = try!(self.inodes.lock().map_err(|_| errno::EDEADLK));
        let s = self.last.get();
        let mut c = (s + 1) % l.len();
        while c != s {
            if l[c].as_ref().map(|x| x.upgrade()).is_none() {
                return Ok(c);
            }
            c = (c + 1) % l.len();
        }
        Err(errno::ENOSPC)
    }
    fn alloc_reg(&'static self) -> KResult<Rc<RVNode>> {
        let mut l = try!(self.inodes.lock().map_err(|_| errno::EDEADLK));
        let ni = try!(self.get_inode());
        let out = Rc::new(RVNode::Regular(RegInode::new(ni, self)));
        l[ni] = Some(out.downgrade());
        Ok(out)
    }
    fn alloc_dir(&'static self, parent: InodeNum) -> KResult<Rc<RVNode>> {
        let mut l = try!(self.inodes.lock().map_err(|_| errno::EDEADLK));
        if self.get_vnode(parent).is_err() {
            dbg!(debug::VFS, "parent of new directory does not exist");
            return Err(errno::EBADF);
        }
        let ni = dbg_try!(self.get_inode(), debug::VFS, "Unable to allocate inode number!");
        let out = Rc::new(RVNode::Directory(DirInode::new(ni, Some(parent), self)));
        self.last.set(ni);
        l[ni] = Some(out.downgrade());
        Ok(out)
    }

    fn get_vnode(&'static self, num: InodeNum) -> KResult<Rc<RVNode>> {
        if num == ROOT_INODE_NUM {
            self.root_dir.clone().ok_or_else(|| { panic!("Unable to get root dir, is None!"); })
        } else if num < MAX_INODES {
            let l = try!(self.inodes.lock().map_err(|_| errno::EDEADLK));
            l[num].clone().ok_or(errno::EBADF).and_then(|x| x.upgrade().ok_or(errno::EBADF))
        } else { Err(errno::EINVAL) }
    }
    unsafe fn initialize(this: &'static mut RamFS) {
        // Wish specific enum variants could be marked copy.
        let inodes = [None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                      None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                      None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                      None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                      None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                      None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                      None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                      None, None, None, None, None, None, None, None, None, None, None, None, None, None, None];
        let root = Some(Rc::new(RVNode::Directory(DirInode::new(ROOT_INODE_NUM, None, this))));
        mem::replace(this, RamFS { last: Cell::new(ROOT_INODE_NUM - 1), inodes: Mutex::new("ramfs mutex", inodes), root_dir : root });
    }
}

impl FileSystem for RamFS {
    type Real = RVNode;
    type Node = Rc<RVNode>;
    fn get_type(&self) -> &'static str { "RamFS" }
    fn get_fs_root<'a>(&'a self) -> Rc<RVNode> {
        self.root_dir.clone().expect("root is null!")
    }
}
