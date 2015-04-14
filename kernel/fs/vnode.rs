//! The base of what vnodes are and their id's and such. Placed here more for dependency purposes
//! than any real organizational reason. Some crates need this but don't really need to know much
//! more about drivers.

use vfs::FileSystem;
use base::devices::*;
use ::InodeNum;
use std::fmt;
use base::errno::{KResult, Errno};
use std::borrow::Borrow;

pub use self::_Mode::*;
#[allow(non_upper_case_globals)]
#[allow(non_snake_case)]
mod _Mode {
    bitmask_create!(
        #[doc = "The different types of fs objects"]
        flags Mode : u8 {
            #[doc = "an unused inode"]
            default Unused,
            #[doc = "A charecter special device"]
            CharDev = 0,
            #[doc = "A directory"]
            Directory = 1,
            #[doc = "A block device"]
            BlockDev = 2,
            #[doc = "A regular file"]
            Regular = 3,
            #[doc = "A symbolic link"]
            Link = 4,
            #[doc = "A fifo pipe"]
            Pipe = 5
        }
    );
}

impl Mode {
    #[inline] fn stat_err(self) -> Errno { Errno::ENOTSUP }
    #[inline] fn len_err(self) -> Errno { Errno::ENOTSUP }
    #[inline] fn read_err(self) -> Errno { if self & Directory != Unused { Errno::EISDIR } else { Errno::ENOTSUP } }
    #[inline] fn write_err(self) -> Errno { self.read_err() }
    #[inline] fn truncate_err(self) -> Errno { self.read_err() }
    #[inline] fn create_err(self) -> Errno { if self & Directory == Unused { Errno::ENOTDIR } else { Errno::ENOTSUP } }
    #[inline] fn lookup_err(self) -> Errno { self.create_err() }
    #[inline] fn mknod_err(self) -> Errno { if self & Directory != Unused { Errno::ENOTDIR } else { Errno::ENODEV } }
    #[inline] fn link_err(self) -> Errno { self.create_err() }
    #[inline] fn unlink_err(self) -> Errno { self.create_err() }
    #[inline] fn mkdir_err(self) -> Errno { self.create_err() }
    #[inline] fn rmdir_err(self) -> Errno { self.create_err() }
    #[inline] fn readdir_err(self) -> Errno { self.create_err() }
}

pub trait VNode : fmt::Debug {
    /// This is only here so that the type system works out. Needed b/c no HKT
    type Real: VNode;
    /// What type of vnode operations will create/get. This must be clone.
    /// We want to say it is borrow so that we can have this be a wrapper that deals with
    /// ref-counting.
    type Res: Borrow<Self::Real> + Clone;
    fn get_fs(&self) -> &FileSystem<Real=Self::Real, Node=Self::Res>;
    fn get_mode(&self) -> Mode;
    fn get_number(&self) -> InodeNum;
    fn stat(&self) -> KResult<Stat> { Err(self.get_mode().stat_err()) }
    fn len(&self) -> KResult<usize> { Err(self.get_mode().len_err()) }

    fn read(&self, _off: usize, _buf: &mut [u8]) -> KResult<usize> { Err(self.get_mode().read_err()) }
    fn write(&self, _off: usize, _buf: &[u8]) -> KResult<usize> { Err(self.get_mode().write_err()) }
    fn truncate(&self, _size: usize) -> KResult<usize> { Err(self.get_mode().truncate_err()) }
    // TODO Figure out the contract for mmap.
    //fn mmap(&self; .) -> KResult<?> { Err(errno::EINVAL) }

    fn create(&self, _name: &str) -> KResult<Self::Res> { Err(self.get_mode().create_err()) }
    fn lookup(&self, _name: &str) -> KResult<Self::Res> { Err(self.get_mode().lookup_err()) }

    fn mknod(&self, _name: &str, _devid: DeviceId) -> KResult<()> { Err(self.get_mode().mknod_err()) }
    // TODO Maybe this should be &Self for from...
    fn link(&self, _from: &Self::Res, _to: &str) -> KResult<()> { Err(self.get_mode().link_err()) }
    fn unlink(&self, _to: &str) -> KResult<()> { Err(self.get_mode().unlink_err()) }
    fn mkdir(&self, _to: &str) -> KResult<()> { Err(self.get_mode().mkdir_err()) }
    fn rmdir(&self, _to: &str) -> KResult<()> { Err(self.get_mode().rmdir_err()) }
    /// Given offset into directory returns the size of the dirent in the directory structure and
    /// the given dirent. If it returns EOK then we have read the whole directory. To read the next
    /// entry add the returned length to the offset.
    fn readdir(&self, _off: usize) -> KResult<(usize, DirEnt)> { Err(self.get_mode().readdir_err()) }

//    fn fill_page(&self, pagenum: usize, page: &mut [u8; page::SIZE]) -> KResult<()>;
//    fn clean_page(&self, pagenum: usize, page: &[u8; page::SIZE]) -> KResult<()>;
//    fn dirty_page(&self, pagenum: usize, page: &[u8; page::SIZE]) -> KResult<()>;
}

pub struct DirEnt {
    pub inode: InodeNum,
    pub offset: usize,
    pub name: String,
}

// TODO
pub struct Stat {
    pub dev: DeviceId,
    pub inode: InodeNum,
    pub rdev: u32,
    pub nlink: u32,
    pub uid: u32,
    pub gid: u32,
    pub size: u32,
    pub atime: u32,
    pub mtime: u32,
    pub ctime: u32,
    pub blksize: u32,
    pub blocks: u32,
}

/*
pub struct Stat {
    pub dev,
    pub inode: inodenum;
    pub rdev,
    pub nlink,
    pub uid,
    pub gid,
    pub size,
    pub atime,
    pub mtime,
    pub ctime,
    pub blksize,
    pub blocks,
}
*/


