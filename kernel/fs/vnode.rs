//! The base of what vnodes are and their id's and such. Placed here more for dependency purposes
//! than any real organizational reason. Some crates need this but don't really need to know much
//! more about drivers.

use core::fmt::Show;
use base::devices::*;
use core::prelude::*;
use umem::mmobj::*;

bitmask_create!(
    flags Mode: u8 {
        #[doc="A charecter special device"]
        CharDev = 0,
        #[doc="A directory"]
        Directory = 1,
        #[doc="A block device"]
        BlockDev = 2,
        #[doc="A regular file"]
        Regular = 3,
        #[doc="A symbolic link"]
        Link = 4,
        #[doc="A fifo pipe"]
        Pipe = 5,
    }
)

pub trait VNode : MMObj + Show {
    fn get_mode(&self) -> Mode;
    fn get_number(&self) -> InodeNum;
    fn stat(&self) -> KResult<Stat>;
    fn len(&self) -> KResult<uint>;

    fn read(&self, off: uint, buf: &mut [u8]) -> KResult<uint> { Err(errno::EISDIR) }
    fn write(&self, off: uint, buf: &[u8]) -> KResult<uint> { Err(errno::EISDIR) }
    fn truncate(&self, size: uint) -> KResult<uint> { Err(errno::EISDIR) }
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

    fn fill_page(&self, pagenum: uint, page: &mut [u8, ..page::SIZE]) -> KResult<()>;
    fn clean_page(&self, pagenum: uint, page: &[u8, ..page::SIZE]) -> KResult<()>;
    fn dirty_page(&self, pagenum: uint, page: &[u8, ..page::SIZE]) -> KResult<()>;
}

pub struct DirEnt {
    pub inode: InodeNum,
    pub offset: uint,
    pub name: String,
}

// TODO
pub struct Stat {
    pub dev: DeviceId,
    pub inode: InodeNum;
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


