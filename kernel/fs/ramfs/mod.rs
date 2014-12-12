
//! The RAMFS

use FileSystem;
use InodeNum;
use vnode::{mod, VNode, Stat, DirEnt};

use self::RVNode::*;

/// The max name length of a directory entry.
pub const NAME_LEN : uint = 28;

#[deriving(Show)]
pub enum RVNode<'a> {
    Byte(ByteInode<'a>),
    Block(BlockInode<'a>),
    Regular(RegInode<'a>),
    Directory(DirInode<'a>),
}

macro_rules! call_matching(
    ($s:expr : ($($t:ident),+) -> $f:ident ($($v),*)) => {{
        match $s {
            $(
                $t(x) => x.$f($($v),*),
             )+
        }
    }}
)
impl<'a> VNode<'a> for RVNode<'a> {
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

    fn fill_page(&self, _: uint, _: &mut[u8,..page::SIZE]) -> KResult<()> { Err(errno::ENOTSUP); }
    fn dirty_page(&self, _: uint, _: &[u8,..page::SIZE]) -> KResult<()> { Err(errno::ENOTSUP); }
    fn clean_page(&self, _: uint, _: &[u8,..page::SIZE]) -> KResult<()> { Err(errno::ENOTSUP); }

    fn stat(&self) -> KResult<Stat> { call_matching!(self : (Byte, Block, Regular, Directory) -> stat()) }
    fn len(&self) -> KResult<uint> { call_matching!(self : (Byte, Block, Regular, Directory) -> len()) }

    fn read(&self, off: uint, buf: &mut [u8]) -> KResult<uint> {
        match *self {
            Byte(x) => x.read(off, buf),
            Regular(x) => x.read(off, buf),
            s => { dbg!(debug::VFS, "unable to read on {}", s); Err(errno::ENOTSUP) }
        }
    }

    fn write(&self, off: uint, buf: &[u8]) -> KResult<uint> {
        match *self {
            Byte(x) => x.write(off, buf),
            Regular(x) => x.write(off, buf),
            s => { dbg!(debug::VFS, "unable to write on {}", s); Err(errno::ENOTSUP) }
        }
    }

    fn truncate(&self, size: uint) -> KResult<uint> {
        match *self {
            Regular(x) => x.write(off, buf),
            s => { dbg!(debug::VFS, "unable to truncate {}", s); Err(errno::ENOTSUP) }
        }
    }

    // TODO Figure out the contract for mmap.
    //fn mmap(&self, ...) -> KResult<?> { Err(errno::EINVAL) }

    fn create(&self, name: &str) -> KResult<Self> { Err(errno::ENOTDIR) }
    fn lookup(&self, name: &str) -> KResult<Self> { Err(errno::ENOTDIR) }

    fn mknod(&self, name: &str, devid: DeviceId) -> KResult<()> { Err(errno::ENOTDIR) }
    // TODO Maybe this should be &Self for from...
    fn link(&self, from: &Self, to: &str) -> KResult<()> { Err(errno::ENOTDIR) }
    fn unlink(&self, to: &str) -> KResult<()> { Err(errno::ENOTDIR) }
    fn mkdir(&self, to: &str) -> KResult<()> { Err(errno::ENOTDIR) }
    fn rmdir(&self, to: &str) -> KResult<()> { Err(errno::ENOTDIR) }
    /// Given offset into directory returns the size of the dirent in the directory structure and
    /// the given dirent. If it returns EOK then we have read the whole directory. To read the next
    /// entry add the returned length to the offset.
    fn readdir(&self, off: uint) -> KResult<(uint, DirEnt)> { Err(errno::ENOTDIR) }
}

#[deriving(Show)]
struct Inode {
    size: AtomicUint,
    num: InodeNum,
    /// Page sized buffer containing inodes contents
    mem: *mut u8,
    mode: vnode::Mode,
    links: AtomicUint,
}

impl Inode {
    fn incr(&self) {
        self.links.fetch_add(1, Relaxed);
    }
    fn decr(&self) {
        bassert!(self.links.fetch_sub(1, Relaxed) != 0, "decr called when no links present");
    }
}

struct ByteInode<'a> {
    inode: &'a Inode,
    fs: &'a RamFS,
}
impl<'a> ByteInode<'a> { fn create(inode: &'a inode, fs: &'a RamFS) -> ByteInode<'a> { bassert!(inode.mode == vnode::CharDev); inode.incr(); ByteInode { inode, fs } } }
impl<'a> Drop for ByteInode<'a> { fn drop(&mut self) { self.inode.decr() } }
impl<'a> Clone for ByteInode<'a> { fn clone(&self) -> ByteInode<'a> { self.inode.incr(); ByteInode { inode: self.inode, fs: self.fs } } }
struct BlockInode<'a> {
    inode: &'a Inode,
    fs: &'a RamFS,
}
impl<'a> BlockInode<'a> { fn create(inode: &'a inode, fs: &'a RamFS) -> BlockInode<'a> { bassert!(inode.mode == vnode::BlockDev); inode.incr(); BlockInode { inode, fs } } }
impl<'a> Drop for BlockInode<'a> { fn drop(&mut self) { self.inode.decr() } }
impl<'a> Clone for BlockInode<'a> { fn clone(&self) -> BlockInode<'a> { self.inode.incr(); BlockInode { inode: self.inode, fs: self.fs } } }
struct RegInode<'a> {
    inode: &'a Inode,
    fs: &'a RamFS,
}
impl<'a> RegInode<'a> { fn create(inode: &'a inode, fs: &'a RamFS) -> RegInode<'a> { bassert!(inode.mode == vnode::Regular); inode.incr(); RegInode { inode, fs } } }
impl<'a> Drop for RegInode<'a> { fn drop(&mut self) { self.inode.decr() } }
impl<'a> Clone for RegInode<'a> { fn clone(&self) -> RegInode<'a> { self.inode.incr(); RegInode { inode: self.inode, fs: self.fs } } }

struct DirInode<'a> {
    inode: &'a Inode,
    fs: &'a RamFS,
}
impl<'a> DirInode<'a> { fn create(inode: &'a inode, fs: &'a RamFS) -> DirInode<'a> { bassert!(inode.mode == vnode::Directory); inode.incr(); DirInode { inode, fs } } }
impl<'a> Clone for DirInode<'a> { fn clone(&self) -> DirInode<'a> { self.inode.incr(); DirInode { inode: self.inode, fs: self.fs } } }
impl<'a> Drop for DirInode<'a> { fn drop(&mut self) { self.inode.decr() } }

struct RDirEnt {
    inode: InodeNum,
    name: [u8, ..MAX_NAME_LEN],
}

impl RDirEnt {
    fn get_name(&self) -> &str {
        use core::str::from_utf8;
        let nseg = self.name.split(|x| x == 0).nth(0).expect("ramfs dirent must have one null");
        return from_utf8(nseg);
    }
}

impl<'a> DirInode<'a> {
    fn inner_lookup(&self, name: &str) -> KResult<&'a Inode> {
        bassert!(self.inode.mode == vnode::Directory, "is not a directory");
        assert!(!name.is_empty());
        assert!(name.find('/').map_or(true, |v| { v == name.len() - 1 }));
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
    fn inner_readdir(&self, off: uint) -> Option<(uint, &RDirEnt)> {
        let step = mem::size_of::<RDirEnt>();
        let max_dirent = page::SIZE/step;
        bassert!(self.inode.mode == vnode::Directory, "is not a directory");
        if off >= self.inode.size.load(Relaxed) { return Err(errno::EOK); }
        if off % step != 0 {
            dbg!(debug::VFS, "{} is not a valid offset into a ramfs directory", off);
            return Err(errno::EINVAL);
        }
        let real_off = off/step;
        let mem = unsafe { from_raw_buf::<'static, RDirEnt>(self.inode.mem as *const RDirEnt, max_dirent) }.slice_from(real_off);
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
}

impl<'a> DirInode<'a> {
    fn get_mode(&self) -> vnode::Mode {
        bassert!(self.0.mode == vnode::Directory);
        vnode::Directory
    }

    fn stat(&self) -> KResult<Stat> {
        // TODO
        kpanic!("not implemented");
    }

    fn create(&self, mut name: &str) -> KResult<Box<VNode<'a>>> {
        bassert!(self.inode.mode == vnode::Directory, "is not a directory");
        let step = mem::size_of::<RDirEnt>();
        let max_dirent = page::SIZE/step;
        bassert!(self.inner_lookup(name) == Err(errno::ENOENT), "existing file with that name");
        if name.len() >= MAX_NAME_LEN {
            name = name.slice_to(MAX_NAME_LEN - 1);
        }
        let mut nsize = step;
        let entries = unsafe { from_raw_mut_buf::<'static, RDirEnt>(self.0.mem, max_dirent) };
        for i in entries.iter_mut() {
            if i.name[0] == 0 {
                let new_inode = RegInode::create(try!(self.fs.alloc_inode(vnode::Regular)), self.fs);
                i.inode = new_inode.get_number();
                i.name.set_memory(0);
                // Update size
                if self.inode.size.load(Relaxed) < nsize { self.inode.size.store(nsize, Relaxed); }
                for (idx, chr) in name.iter().enumerate() { i.name[idx] = chr; }
                return Ok(box new_inode);
            } else {
                nsize += step;
            }
        }
        Err(errno::ENOSPC)
    }

    fn link(&self, from: &VNode, to: &str ) -> KResult<()> {
        // TODO
    }

    fn lookup(&self, name: &str) -> KResult<Box<VNode<'a>>> {
        let ind = try!(self.inner_lookup(name));
        match ind.mode {
            vnode::Regular => Ok(box RegInode::create(ind, self.fs))    as KResult<Box<VNode<'a>>>
            vnode::Directory => Ok(box DirInode::create(ind, self.fs))  as KResult<Box<VNode<'a>>>
            vnode::CharDev => Ok(box ByteInode::create(ind, self.fs))   as KResult<Box<VNode<'a>>>
            vnode::BlockDev => Ok(box BlockInode::create(ind, self.fs)) as KResult<Box<VNode<'a>>>
            _ => { panic!("unknown inode type in ramfs {}", ind.mode); }
        }
    }

    fn readdir(&self, off: uint) -> KResult<(uint, DirEnt)> {
        self.inner_readdir(off).map(|(v,d)| {
            (v, DirEnt { inode: d.inode, offset: 0, name: unsafe { d.get_name().to_string() } })
        }).ok_or(errno::EOK)
    }
}

pub struct RamFS {
    inodes: UnsafeCell<[Inode,..MAX_INODES]>,
}

impl RamFS {
    fn alloc_inode(&self, mode: vnode::Mode) -> KResult<&Inode> {
        let inds = unsafe { &mut *self.inodes.get() };
        for i in inds.iter_mut() {
            if i.links.compare_and_swap(0, 1, SeqCst) == 0 {
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
        return inds.
    }
}
