
//! The RAMFS

use FileSystem;
use InodeNum;
use vnode::{self, VNode, Stat, DirEnt};
use mm::page;
use base::errno::{self, KResult};
use base::devices::DeviceId;
use std::sync::atomic::AtomicUint;
use std::sync::atomic::Ordering::{SeqCst, Relaxed};
use std::{slice, mem, fmt};
use std::cell::*;
use std::slice::bytes::copy_memory;
use umem::mmobj::{MMObjId, MMObj};

use self::RVNode::*;

/// The FS to use for a ramfs
static mut FS : Option<*mut RamFS> = None;

/// The max name length of a directory entry.
pub const NAME_LEN : usize = 28;

/// The number of files we will allow.
pub const MAX_INODES : usize = 128;

#[deriving(Show)]
pub enum RVNode<'a> {
    Byte(ByteInode<'a>),
    Block(BlockInode<'a>),
    Regular(RegInode<'a>),
    Directory(DirInode<'a>),
}

macro_rules! call_matching {
    ($s:ident : ($($t:ident),+) -> $f:ident ()) => {{
        match $s { $( $t(x) => x.$f(), )+ }
    }};
    ($s:ident : ($($t:ident),+) -> $f:ident ($v:expr)) => {{
        match $s { $( $t(x) => x.$f($v), )+ }
    }};
    ($s:ident : ($($t:ident),+) -> $f:ident ($v:expr, $v2:expr)) => {{
        match $s { $( $t(x) => x.$f($v, $v2), )+ }
    }};
}

impl<'a> RVNode<'a> {
    /// Get the inode that actually represents this file.
    #[inline]
    fn get_inode(&self) -> &'a Inode {
        match *self {
            Byte(ref i)      => i.inode,
            Block(ref i)     => i.inode,
            Regular(ref i)   => i.inode,
            Directory(ref i) => i.inode,
        }
    }
}

impl<'a> fmt::Show for RVNode<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "RVNode {{ type: {}, refs: {}, num {} }}",
               self.get_mode(), self.get_refcount(), self.get_number())
    }
}

impl<'a> MMObj for RVNode<'a> {
    fn get_id(&self) -> MMObjId { MMObjId::new(RAMFS_DEVID, self.get_number()) }
    fn fill_page(&self,  pf: &mut PFrame)  -> KResult<()> { dbg!(debug::VFS, "ramfs vnode used as mmobj!"); Err(errno::ENOTSUP) }
    fn dirty_page(&self,  pf: &mut PFrame)  -> KResult<()> { dbg!(debug::VFS, "ramfs vnode used as mmobj!"); Err(errno::ENOTSUP) }
    fn clean_page(&self,  pf: &mut PFrame)  -> KResult<()> { dbg!(debug::VFS, "ramfs vnode used as mmobj!"); Err(errno::ENOTSUP) }
    // TODO The next two maybe should panic?
    fn dirty_page(&self, _pf: &PFrame)      -> KResult<()> { Ok(()) }
    fn clean_page(&self, _pf: &PFrame)      -> KResult<()> { Ok(()) }
}
impl<'a> VNode for RVNode<'a> {
    fn get_mode(&self) -> vnode::Mode {
        match *self {
            Byte(_) => vnode::CharDev,
            Block(_) => vnode::BlockDev,
            Regular(_) => vnode::Regular,
            Directory(_) => vnode::Directory,
        }
    }

    fn get_number(&self) -> InodeNum {
        call_matching!(self : (Byte, Block, Regular, Directory) -> get_number())
    }

    fn stat(&self) -> KResult<Stat> { call_matching!(self : (Byte, Block, Regular, Directory) -> stat()) }
    fn len(&self) -> KResult<usize> { call_matching!(self : (Byte, Block, Regular, Directory) -> len()) }

    fn read(&self, off: usize, buf: &mut [u8]) -> KResult<usize> {
        match *self {
            Byte(x) => x.read(off, buf),
            Regular(x) => x.read(off, buf),
            s => { dbg!(debug::VFS, "unable to read on {}", s); Err(errno::ENOTSUP) }
        }
    }

    fn write(&self, off: usize, buf: &[u8]) -> KResult<usize> {
        match *self {
            Byte(x) => x.write(off, buf),
            Regular(x) => x.write(off, buf),
            s => { dbg!(debug::VFS, "unable to write on {}", s); Err(errno::ENOTSUP) }
        }
    }

    fn truncate(&self, size: usize) -> KResult<usize> {
        match *self {
            Regular(x) => x.truncate(size),
            s => { dbg!(debug::VFS, "unable to truncate {}", s); Err(errno::ENOTSUP) }
        }
    }

    // TODO Figure out the contract for mmap.
    //fn mmap(&self, ...) -> KResult<?> { Err(errno::EINVAL) }

    fn create(&self, name: &str) -> KResult<Self> {
        match *self {
            Directory(d) => Regular(d.create(name)),
            s => { dbg!(debug::VFS, "unable to create in {}", s); Err(errno::ENOTDIR) }
        }
    }

    fn lookup(&self, name: &str) -> KResult<Self> {
        match *self {
            Directory(d) => d.lookup(name),
            s => { dbg!(debug::VFS, "unable to create in {}", s); Err(errno::ENOTDIR) }
        }
    }

    fn mknod(&self, name: &str, devid: DeviceId) -> KResult<()> {
        match *self {
            Directory(d) => d.mknod(name),
            s => { dbg!(debug::VFS, "unable to mknod in {}", s); Err(errno::ENOTDIR) }
        }
    }

    // TODO Maybe this should be &Self for from...
    fn link(&self, to: &str) -> KResult<()> { if let Directory(d) = *self { d.link(to) } else { Err(errno::ENOTDIR) } }
    fn unlink(&self, to: &str) -> KResult<()> { if let Directory(d) = *self { d.unlink(to) } else { Err(errno::ENOTDIR) } }
    fn mkdir(&self, to: &str) -> KResult<()> { if let Directory(d) = *self { d.mkdir(to) } else { Err(errno::ENOTDIR) } }
    fn rmdir(&self, to: &str) -> KResult<()> { if let Directory(d) = *self { d.rmdir(to) } else { Err(errno::ENOTDIR) } }
    /// Given offset into directory returns the size of the dirent in the directory structure and
    /// the given dirent. If it returns EOK then we have read the whole directory. To read the next
    /// entry add the returned length to the offset.
    fn readdir(&self, off: usize) -> KResult<(usize, DirEnt)> { if let Directory(d) = *self { d.readdir(off) } else { Err(errno::ENOTDIR) } }
}

#[deriving(Show)]
struct Inode {
    size: AtomicUint,
    num: InodeNum,
    /// Page sized buffer containing inodes contents
    mem: *mut u8,
    mode: vnode::Mode,
    links: AtomicUint,
    devid: Option<DeviceId>,
}

impl Inode {
    fn incr(&self) {
        self.links.fetch_add(1, Relaxed);
    }
    fn get_size(&self) -> usize { self.size.load(Relaxed) }
    fn decr(&self) {
        bassert!(self.links.fetch_sub(1, Relaxed) != 0, "decr called when no links present");
    }
    fn get_link_count(&self) -> usize { self.links.load(Relaxed) }
    fn new(n: InodeNum) -> Inode {
        Inode {
            size: AtomicUint::new(0),
            num: n,
            mem: page::alloc().unwrap(),
            mode: vnode::Unused,
            links: AtomicUint::new(0),
            devid: None,
        }
    }
}

struct ByteInode<'a> {
    inode: &'a Inode,
    fs: &'a RamFS,
}
impl<'a> ByteInode<'a> {
    fn new(inode: &'a Inode, fs: &'a RamFS) -> ByteInode<'a> {
        bassert!(inode.mode == vnode::CharDev);
        inode.incr();
        ByteInode { inode: inode, fs: fs }
    }
}
impl<'a> Drop for ByteInode<'a> { fn drop(&mut self) { self.inode.decr() } }
impl<'a> Clone for ByteInode<'a> {
    fn clone(&self) -> ByteInode<'a> {
        self.inode.incr();
        ByteInode { inode: self.inode, fs: self.fs }
    }
}
struct BlockInode<'a> {
    inode: &'a Inode,
    fs: &'a RamFS,
}
impl<'a> BlockInode<'a> {
    fn new(inode: &'a Inode, fs: &'a RamFS) -> BlockInode<'a> {
        bassert!(inode.mode == vnode::BlockDev);
        inode.incr();
        BlockInode { inode: inode, fs: fs }
    }
}
impl<'a> Drop for BlockInode<'a> { fn drop(&mut self) { self.inode.decr() } }
impl<'a> Clone for BlockInode<'a> {
    fn clone(&self) -> BlockInode<'a> {
        self.inode.incr();
        BlockInode { inode: self.inode, fs: self.fs }
    }
}
struct RegInode<'a> {
    inode: &'a Inode,
    fs: &'a RamFS,
}
impl<'a> RegInode<'a> {
    fn new(inode: &'a Inode, fs: &'a RamFS) -> RegInode<'a> {
        bassert!(inode.mode == vnode::Regular);
        inode.incr();
        RegInode { inode: inode, fs: fs }
    }

    fn read(&self, off: usize, buf: &mut [u8]) -> KResult<usize> {
        use std::cmp::{max, min};
        use std::slice::{bytes, from_raw_buf};
        let ret = max(0, min(buf.len() as usize, self.inode.get_size() - off));
        unsafe { 
            let src =  from_raw_buf(self.inode.mem, page::SIZE);
            bytes::copy_memory(buf, src[off..off + ret]);
        }
        Ok(ret)
    }
}
impl<'a> Drop for RegInode<'a> { fn drop(&mut self) { self.inode.decr() } }
impl<'a> Clone for RegInode<'a> {
    fn clone(&self) -> RegInode<'a> {
        self.inode.incr();
        RegInode { inode: self.inode, fs: self.fs }
    }
}

struct DirInode<'a> {
    inode: &'a Inode,
    fs: &'a RamFS,
}
impl<'a> DirInode<'a> {
    fn new(inode: &'a Inode, fs: &'a RamFS) -> DirInode<'a> {
        bassert!(inode.mode == vnode::Directory);
        inode.incr();
        DirInode { inode: inode, fs: fs }
    }
}
impl<'a> Clone for DirInode<'a> {
    fn clone(&self) -> DirInode<'a> {
        self.inode.incr();
        DirInode { inode: self.inode, fs: self.fs }
    }
}
impl<'a> Drop for DirInode<'a> { fn drop(&mut self) { self.inode.decr() } }

struct RDirEnt {
    inode: InodeNum,
    name: [u8; NAME_LEN],
}

impl RDirEnt {
    fn get_name(&self) -> &str {
        use std::str::from_utf8;
        let nseg = self.name.split(|x| x == 0).nth(0).expect("ramfs dirent must have one null");
        return from_utf8(nseg);
    }
}

impl<'a> DirInode<'a> {
    fn inner_lookup(&self, name: &str) -> KResult<&'a Inode> {
        bassert!(self.inode.mode == vnode::Directory, "is not a directory!");
        assert!(!name.is_empty());
        assert!(name.find('/').map_or(true, |v| { v == name.len() - 1 }),
                "'{}' contains a non-terminal '/' this should have been dealt with in VFS!");
        let mut off = 0;
        loop {
            if let Some((nxt, de)) = self.inner_readdir(off) {
                if de.get_name() == name {
                    return Ok(self.fs.get_inode(de.inode))
                }
            } else {
                return Err(errno::ENOENT);
            }
        }
    }

    fn inner_readdir(&self, off: usize) -> Option<(usize, &RDirEnt)> {
        let step = mem::size_of::<RDirEnt>();
        let max_dirent = page::SIZE/step;
        bassert!(self.inode.mode == vnode::Directory, "is not a directory");
        if off >= self.inode.size.load(Relaxed) { return Err(errno::EOK); }
        if off % step != 0 {
            dbg!(debug::VFS, "{} is not a valid offset into a ramfs directory", off);
            return Err(errno::EINVAL);
        }
        let real_off = off/step;
        let mem = unsafe { slice::from_raw_buf::<'static, RDirEnt>(self.inode.mem as *const RDirEnt, max_dirent) }.slice_from(real_off);
        let mut disp = step;
        for x in mem.iter() {
            if x.name[0] != 0 {
                return Some((disp, x));
            } else {
                disp += step;
            }
        }
        None
    }

    fn get_mode(&self) -> vnode::Mode {
        bassert!((self.0).mode == vnode::Directory);
        vnode::Directory
    }

    fn stat(&self) -> KResult<Stat> {
        // TODO
        kpanic!("not implemented");
    }

    fn create(&self, name: &str) -> KResult<RVNode<'a>> {
        bassert!(self.inode.mode == vnode::Directory, "is not a directory");
        // If we do not get a positive return value the only link will be from this inode which
        // will be destroyed
        let new_inode = RegInode::new(try!(self.fs.alloc_inode(vnode::Regular)), self.fs);
        return self.link(new_inode, name).map(|_| new_inode);
    }

    fn link(&self, from: &RVNode<'a>, name: &str ) -> KResult<()> {
        assert!(from.get_inode().links.load(Relaxed) != 0);
        bassert!(self.inode.mode == vnode::Directory, "is not a directory");
        let step = mem::size_of::<RDirEnt>();
        let max_dirent = page::SIZE/step;
        bassert!(self.inner_lookup(name) == Err(errno::ENOENT), "existing file with that name");
        if name.len() >= NAME_LEN {
            return Err(errno::ENAMETOOLONG);
        }
        let mut nsize = step;
        let entries = unsafe { slice::from_raw_mut_buf::<'static, RDirEnt>(self.0.mem, max_dirent) };
        for i in entries.iter_mut() {
            if i.name[0] == 0 {
                i.inode = from.get_number();
                i.name.set_memory(0);
                // Update size
                if self.inode.size.load(Relaxed) < nsize { self.inode.size.store(nsize, Relaxed); }

                // Incr reference count on self.
                from.get_inode().incr();
                self.inode.incr();

                for (idx, chr) in name.iter().enumerate() { i.name[idx] = chr; }

                return Ok();
            } else {
                nsize += step;
            }
        }
        Err(errno::ENOSPC)
    }

    fn mkdir(&self, name: &str) -> KResult<RVNode<'a>> {
        let cur_links = self.inode.get_link_count();

        let new_inode = DirInode::new(try!(self.fs.alloc_inode(vnode::Directory)), self.fs);

        // Link to '.';
        try!(new_inode.link(new_inode, "."));
        // We need to make sure the new_inode only has 1 reference if it is empty, to make rmdir
        // work easily.
        new_inode.inode.decr();
        bassert!(new_inode.inode.links.load(Relaxed) == 1);

        let ret = Directory(new_inode);

        // If we fail the only link to new_inode is the immediate in this function so it will get
        // destroyed. We are good.
        try!(self.link(&ret, name));
        bassert!(ret.get_inode().get_link_count() == 2);
        bassert!(self.inode.get_link_count() == cur_links + 1);

        // Link to '..'. This also increments the current link count for this directory.
        // TODO Do I need to decr self?
        try!(ret.link(&Directory(self.clone()), ".."));
        ret.get_inode().decr();
        self.inode.decr();
        bassert!(ret.get_inode().get_link_count() == 1);
        bassert!(self.inode.get_link_count() == cur_links + 1);

        Ok(ret)
    }

    fn mknod(&self, name: &str, devid: DeviceId) -> KResult<RVNode<'a>> {
        // TODO How to tell byte and block apart.
        Err(errno::ENOTSUP)
    }

    fn lookup(&self, name: &str) -> KResult<RVNode<'a>> {
        let ind = try!(self.inner_lookup(name));
        match ind.mode {
            vnode::Regular => Ok(Regular(RegInode::new(ind, self.fs))),
            vnode::Directory => Ok(Directory(DirInode::new(ind, self.fs))),
            vnode::CharDev => Ok(Byte(ByteInode::new(ind, self.fs))),
            vnode::BlockDev => Ok(Block(BlockInode::new(ind, self.fs))),
            _ => { panic!("unknown inode type in ramfs {}", ind.mode); }
        }
    }

    fn readdir(&self, off: usize) -> KResult<(usize, DirEnt)> {
        self.inner_readdir(off).map(|(v,d)| {
            (v, DirEnt { inode: d.inode, offset: 0, name: unsafe { d.get_name().to_string() } })
        }).ok_or(errno::EOK)
    }
}

pub struct RamFS {
    inodes: UnsafeCell<[Inode; MAX_INODES]>,
}

impl RamFS {
    fn create() -> RamFS {
        let inodes : [Inode; MAX_INODES] = mem::uninitialized();
        //let memory : *mut u8 = 

        for i in range(0, MAX_INODES) {
            inodes[i] = Inode::new();
        }

        RamFS { inodes: UnsafeCell::new(inodes) }
    }
    /// gets one an unlinked inode, does not link it. Must link it to something before going to
    /// sleep.
    fn alloc_inode(&self, mode: vnode::Mode) -> KResult<&Inode> {
        let inds = unsafe { &mut *self.inodes.get() };
        for i in inds.iter_mut() {
            if i.links.load(SeqCst) == 0 {
                i.devid = None;
                i.mode = mode;
                i.size.set(0, Relaxed);
                return Ok(&*i);
            }
        }
        return Err(errno::ENOSPC);
    }
    fn get_inode(&self, id: InodeNum) -> &Inode {
        bassert!(id < MAX_INODES);
        let inds = unsafe { &*self.inodes.get() };
        return inds[id];
    }
}

