
//! The Reenix ATA support.
#![allow(dead_code)]
use mm::page;
use util::Cacheable;
use DeviceId;
use std::cell::*;
use base::{io, kernel};
use blockdev::disk::dma;
use procs::interrupt;
use procs::sync::*;
use base::errno::{KResult, self};
use libc::c_void;
use std::fmt::{self, Formatter, Show};
use umem::mmobj::{MMObjId, MMObjMut};
use umem::pframe::PFrame;
use RDeviceMut;
use WDeviceMut;

mod irqs {
    pub const DISK_PRIMARY   : u16 = 14;
    pub const DISK_SECONDARY : u16 = 15;
}

const BLOCK_SIZE : usize = page::SIZE;
pub const DISK_MAJOR : u8 = 1;
pub const NDISKS : usize = 1;
const IDENT_BUFSIZE : usize = 256;

#[repr(u8)]
enum InterfaceType {
    ATA = 0,
    ATAPI = 1,
}

#[repr(u8)]
enum Type {
    MASTER = 0,
    SLAVE = 1,
}

#[repr(u8)]
enum Operations {
    READ = 0,
    WRITE = 1,
}

mod ports {
    pub const PRIMARY_CTRL : u16 = 0x3f0;
    pub const PRIMARY_CMD  : u16 = 0x1f0;
    pub const SECONDARY_CTRL : u16 = 0x370;
    pub const SECONDARY_CMD  : u16 = 0x170;
}

const NUM_CHANNELS : usize = 2;
const SECTOR_SIZE : usize = 512;

/** Drive/head values (for ATA_REG_DRIVEHEAD) and for CHS/LBA */
mod drivehead {
    pub const MASTER : u8 = 0xa0;
    pub const SLAVE  : u8 = 0xb0;
    /* Drive/head values for CHS / LBA */
    pub const CHS    : u8 = 0x00;
    pub const LBA    : u8 = 0x40;
}

/** Port address offsets for registers */
mod register {
    // TODO Maybe I should make this an enum
    /* Command registers */
    pub const DATA         : u8 = 0x00; /* Data register (read/write address) */
    pub const ERROR        : u8 = 0x01;
    pub const FEATURE      : u8 = 0x01;
    pub const SECCOUNT0    : u8 = 0x02; /* Number of sectors to read/write */
    pub const SECNUM       : u8 = 0x03; /* chs addressing */
    pub const CYLLOW       : u8 = 0x04;
    pub const CYLHIGH      : u8 = 0x05;
    pub const LBA0         : u8 = 0x03; /* lba addressing */
    pub const LBA1         : u8 = 0x04;
    pub const LBA2         : u8 = 0x05;
    pub const DRIVEHEAD    : u8 = 0x06; /* Special drive info (used to set master/slave) */
    pub const COMMAND      : u8 = 0x07; /* Write only */
    pub const STATUS       : u8 = 0x07; /* Read only */
    pub const SECCOUNT1    : u8 = 0x08; /* These four are only used in lba48 */
    pub const LBA3         : u8 = 0x09;
    pub const LBA4         : u8 = 0x0A;
    pub const LBA5         : u8 = 0x0B; /* --- */
    pub const NIEN_CONTROL : u8 = 0x0C;

    /* Control registers */
    pub const CONTROL      : u8 = 0x06; /* Write only */
    /* Like status, but does not imply clear of interrupt */
    pub const ALTSTATUS    : u8 = 0x06; /* Read only */
    pub const DEVADDRESS   : u8 = 0x07;
}

/** Status codes (for ATA_REG_STATUS) */
mod status {
    pub const BSY  : u8 = 0x80; /* Busy */
    pub const DRDY : u8 = 0x40; /* Drive ready */
    pub const DF   : u8 = 0x20; /* Drive write fault */
    pub const DSC  : u8 = 0x10; /* Drive seek complete */
    pub const DRQ  : u8 = 0x08; /* Data request ready */
    pub const CORR : u8 = 0x04; /* Corrected data */
    pub const IDX  : u8 = 0x02; /* inlex */
    pub const ERR  : u8 = 0x01; /* Error */
}

/** Error codes (for ATA_REG_ERROR) */
mod error {
    pub const BBK   : u8 = 0x80; /* Bad sector */
    pub const UNC   : u8 = 0x40; /* Uncorrectable data */
    pub const MC    : u8 = 0x20; /* No media */
    pub const IDNF  : u8 = 0x10; /* ID mark not found */
    pub const MCR   : u8 = 0x08; /* No media */
    pub const ABRT  : u8 = 0x04; /* Command aborted */
    pub const TK0NF : u8 = 0x02; /* Track 0 not found */
    pub const AMNF  : u8 = 0x01; /* No address mark */
}

/** Commands (for ATA_REG_COMMAND) */
mod command {
    pub const READ_PIO         : u8 = 0x20;
    pub const READ_PIO_EXT     : u8 = 0x24;
    pub const READ_DMA         : u8 = 0xC8;
    pub const READ_DMA_EXT     : u8 = 0x25;
    pub const WRITE_PIO        : u8 = 0x30;
    pub const WRITE_PIO_EXT    : u8 = 0x34;
    pub const WRITE_DMA        : u8 = 0xCA;
    pub const WRITE_DMA_EXT    : u8 = 0x35;
    pub const CACHE_FLUSH      : u8 = 0xE7;
    pub const CACHE_FLUSH_EXT  : u8 = 0xEA;
    pub const PACKET           : u8 = 0xA0;
    pub const IDENTIFY_PACKET  : u8 = 0xA1;
    pub const IDENTIFY         : u8 = 0xEC;
}

const IDENT_MAX_LBA : usize = 30;

#[derive(Show, Clone)]
struct Channel {
    cmd : u16,
    ctrl: u16,
    intr: u8,
    dev : DeviceId,
    busmaster : u16,
}

impl Channel {
    /// Writes to command registers
    #[inline] unsafe fn outb(&self, reg: u8, data: u8)  { io::outb(self.cmd + (reg as u16), data) }
    #[inline] unsafe fn outw(&self, reg: u8, data: u16) { io::outw(self.cmd + (reg as u16), data) }
    #[inline] unsafe fn outl(&self, reg: u8, data: u32) { io::outl(self.cmd + (reg as u16), data) }
    /// Reads from the command registers, NOT the control registers
    #[inline] unsafe fn inb(&self, reg: u8) -> u8  { io::inb(self.cmd + (reg as u16)) }
    #[inline] unsafe fn inw(&self, reg: u8) -> u16 { io::inw(self.cmd + (reg as u16)) }
    #[inline] unsafe fn inl(&self, reg: u8) -> u32 { io::inl(self.cmd + (reg as u16)) }
    /// Helpful for delaying, etc.
    #[inline] unsafe fn altstatus(&self) -> u8 { io::inb(self.ctrl + (register::ALTSTATUS as u16)) }
    #[inline] unsafe fn sync(&self) { self.altstatus(); }
    #[inline] unsafe fn pause(&self) { self.sync(); kernel::ndelay(400); }
    /// Get the channel number so we can get the DMA information.
    #[inline] fn get_channel_num(&self) -> u8 { self.dev.get_minor() }
}

pub fn init_stage1() {
}

#[allow(unused_variables)]
pub fn init_stage2() {
    interrupt::map(irqs::DISK_PRIMARY, interrupt::DISK_PRIMARY);
    interrupt::map(irqs::DISK_SECONDARY, interrupt::DISK_SECONDARY);

    let ipl = interrupt::temporary_ipl(interrupt::DISK_PRIMARY);

    // totally stolen from weenix
    unsafe {
        assert!(NDISKS <= NUM_CHANNELS);
        for i in range(0, NDISKS) {
            // TODO
            let mut c = DEFAULT_CHANNELS[i].clone();
            // Choose drive. In this case always master since no slave support.
            c.outb(register::DRIVEHEAD, drivehead::MASTER | drivehead::LBA);
            // Set the sector count register to be 0
            c.outb(register::SECCOUNT0, 0);
            // Set the LBA0/1/2 registers to be 0
            c.outb(register::LBA0, 0);
            c.outb(register::LBA1, 0);
            c.outb(register::LBA2, 0);

            // Disable IRQs for master (stolen from OS-dev)
            io::outb(ports::PRIMARY_CTRL + (register::CONTROL as u16), 0x02);

            // Tell drive to get ready to in identification space.
            c.outb(register::COMMAND, command::IDENTIFY);
            // Wait some for the drive to process.
            c.pause();

            // If status register is 0x00 drive does not exist.
            if 0 == c.inb(register::STATUS) {
                dbg!(debug::DISK | debug::CORE, "Drive {:?} does not exist! status is 0b{:08b}", c, c.inb(register::STATUS));
                continue;
            }
            // Poll until the bsy bit clears.
            loop {
                let cur_status = c.inb(register::STATUS);
                if cur_status & status::BSY == 0 { break; }
                c.pause();
            }
            // Now the drive is no longer busy. Poll until the error bit is set or drq is set
            loop {
                let cur_status = c.inb(register::STATUS);
                if cur_status & status::ERR != 0 {
                    kpanic!("Error setting up ATA drive {:?}, status is 0b{:08b}", c, cur_status);
                }
                if cur_status & status::DRQ != 0 { break; } else { c.pause(); }
            }

            // Now clear the command register
            io::outb(ports::PRIMARY_CTRL + (register::CONTROL as u16), 0x00);

            let mut id_buf : [u32; IDENT_BUFSIZE] = [0; IDENT_BUFSIZE];
            // Get the meta data of the disk.
            for i in id_buf.iter_mut() {
                *i = c.inl(register::DATA);
            }
            extern "C" {
                // TODO This should really be in Rust. I would need to (at very the least) copy the
                // TODO structs for PCI to rust and that would be annoying
                fn ata_setup_busmaster_simple(c: u8) -> u16;
            }
            // Set the busmaster.
            c.busmaster = ata_setup_busmaster_simple(c.get_channel_num());

            // Allocate the new disk.
            let disk = box UnsafeCell::new(ATADisk::create(c, true, (id_buf[IDENT_MAX_LBA] as usize),
                                                           BLOCK_SIZE / SECTOR_SIZE));
            let rd = disk.get().as_ref().expect("should not be null");
            // TODO Doing this is somewhat bad but there is no way (At the moment) to remove disks
            // TODO so it is at least safe. Idealy we would not need this and just do dynamic_cast
            // TODO to TTY in interrupt handler.
            DISKS[rd.channel.get_channel_num() as usize] = disk.get();
            interrupt::register(rd.channel.intr, ata_intr_handler);
            dbg!(debug::DISK, "Registering disk {:?}", rd);
            ::blockdev::register(rd.channel.dev, disk);
        }
    }
}

static mut DISKS : [*mut ATADisk; NDISKS] = [0 as *mut ATADisk; NDISKS];
const DEFAULT_CHANNELS : [Channel; NUM_CHANNELS] = [Channel { cmd : ports::PRIMARY_CMD,
                                                                ctrl: ports::PRIMARY_CTRL,
                                                                intr: interrupt::DISK_PRIMARY,
                                                                dev : DeviceId_static!(DISK_MAJOR, 0),
                                                                busmaster : 0, },
                                                      Channel { cmd : ports::SECONDARY_CMD,
                                                                ctrl: ports::SECONDARY_CTRL,
                                                                intr: interrupt::DISK_SECONDARY,
                                                                dev : DeviceId_static!(DISK_MAJOR, 1),
                                                                busmaster : 0, } ];

pub struct ATADisk {
    channel           : Channel, // The channel we are on.
    is_master         : bool,             // Master or slave. Currently must be master because no slave support.
    size              : usize,             // Size of disk in sectors.
    sectors_per_block : usize,
    mutex             : SMutex,           // Only one proc can be using the disk at any time.
    queue             : WQueue,           // We need to sleep while holding the lock.
    prd               : dma::Prd,         // The dma information.
}

impl Show for ATADisk {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}-ATADisk {{ channel: {:?}, size: {}k }}",
               if self.is_master { "master" } else {"slave"}, self.channel, ((self.size/self.sectors_per_block)*BLOCK_SIZE)/1024)
    }
}

impl ::blockdev::BlockDevice for UnsafeCell<ATADisk> {}

impl ATADisk {
    fn create(channel: Channel, is_master: bool, size: usize, sectors_per_block: usize) -> ATADisk {
        ATADisk {
            channel : channel,
            is_master : is_master,
            size : size,
            sectors_per_block : sectors_per_block,
            mutex : SMutex::new("ATA disk mutex"),
            queue : WQueue::new(),
            prd   : dma::Prd { addr: 0, count: 0, last : 0, buf: [0; 128] },
        }
    }

    #[inline]
    fn handle_interrupt(&self) { self.queue.signal(); }

    /// This is the function that actually handles all the reads and writes. Due to the fact that
    /// both ops are very similar this has been collapsed into a single function.
    #[allow(unused_variables, unused_must_use)]
    unsafe fn unsafe_do_operation(&mut self, block: usize, buf: *mut u8, to_write: bool) -> KResult<()> {
        // RAII lock. It is freed at end of block and unlocks self.
        let lock = self.mutex.force_lock();
        let sec = block * self.sectors_per_block;
        if sec + self.sectors_per_block >  self.size {
            dbger!(debug::DISK, errno::EIO, "ERROR: request to {} block {} when disk is only {} blocks long",
                   if to_write { "write" } else { "read" }, block, self.size / self.sectors_per_block);
            return Err(errno::EIO);
        }
        // RAII ipl. It resets at the end.
        let ipl = interrupt::temporary_ipl(interrupt::DISK_SECONDARY);

        // Load DMA.
        self.prd.load(buf as *const c_void, BLOCK_SIZE as u16);

        // Write the number of sectors to read.
        self.channel.outb(register::SECCOUNT0, self.sectors_per_block as u8);

        // Enter the start location in little endian order.
        self.channel.outb(register::LBA0, (sec & 0xff) as u8);
        self.channel.outb(register::LBA1, ((sec >> 8) & 0xff) as u8);
        self.channel.outb(register::LBA2, ((sec >> 16) & 0xff) as u8);

        // Tell the system what we want to do.
        self.channel.outb(register::COMMAND, if to_write { command::WRITE_DMA } else { command::READ_DMA });

        // Pause
        self.channel.pause();

        // Start DMA op.
        self.prd.start(self.channel.busmaster, to_write);

        // Wait for the interrupt signal saying we got something.
        self.queue.wait();

        // Check for an error, return EIO if we get one...
        if status::ERR & self.channel.inb(register::STATUS) != 0 {
            self.channel.outb(register::ERROR, 0);
            self.prd.reset(self.channel.busmaster);
            dbger!(debug::DISK, errno::EIO, "Unable to perform {} of block {} because of DMA error",
                   if to_write { "write" } else { "read" }, block);
            Err(errno::EIO)
        } else {
            self.prd.reset(self.channel.busmaster);
            dbg!(debug::DISK, "Successfully {} block {}", if to_write { "wrote" } else { "read" }, block);
            Ok(())
        }
    }

    fn write_single(&mut self, block: usize, buf: &[u8; page::SIZE]) -> KResult<usize> {
        if !page::aligned(buf.as_ptr()) {
            kpanic!("The given pointer of buf {:p} is not page aligned! This shouldn't be possible with our current memory strategy.");
            // TODO I might want to just copy it if we get this, OTOH It is unlikely to be much of
            // TODO a problem in practice and we might just want to make sure we always stop it.
        }
        unsafe { self.unsafe_do_operation(block, buf.as_ptr() as *mut u8, true).and(Ok(1)) }
    }

    /// Read a single block from the device. This requires that buf be page aligned (although due
    /// to the way weenix handles memory this should almost always be true anyway) and then reads
    /// the block.
    fn read_single(&mut self, block: usize, buf: &mut [u8; page::SIZE]) -> KResult<usize> {
        if !page::aligned(buf.as_ptr()) {
            kpanic!("The given pointer of buf {:p} is not page aligned! This shouldn't be possible with our current memory strategy.");
            // TODO I might want to just copy it if we get this, OTOH It is unlikely to be much of
            // TODO a problem in practice and we might just want to make sure we always stop it.
        }
        unsafe { self.unsafe_do_operation(block, buf.as_ptr() as *mut u8, false).and(Ok(1)) }
    }
}

impl RDeviceMut<[u8; page::SIZE]> for ATADisk {
    /// Read buf.len() objects from the device starting at offset. Returns the number of objects
    /// read from the stream, or errno if it fails.
    fn read_from(&mut self, offset: usize, buf: &mut [[u8; page::SIZE]]) -> KResult<usize> {
        for i in range(0, buf.len()) {
            try!(self.read_single(offset + i, &mut buf[i]));
        }
        Ok(buf.len())
    }
}

impl WDeviceMut<[u8; page::SIZE]> for ATADisk {
    /// Write the buffer to the device, starting at the given offset from the start of the device.
    /// Returns the number of bytes written or errno if an error happens.
    fn write_to(&mut self, offset: usize, buf: &[[u8; page::SIZE]]) -> KResult<usize> {
        for i in range(0, buf.len()) {
            dbg!(debug::DISK, "starting write of page {} to block {}", i, offset + i);
            try!(self.write_single(offset + i, &buf[i]));
        }
        Ok(buf.len())
    }
}

impl MMObjMut for ATADisk {
    // TODO TEST ALL OF THIS

    // TODO I might want to get rid of the MMObjId thing and just use memory location like in
    // TODO weenix, gaurenteed uniqueness
    fn get_id(&self) -> MMObjId { MMObjId::new(self.channel.dev, 0) }

    /**
     * Fill the given page frame with the data that should be in it.
     */
    fn fill_page(&mut self, pf: &mut PFrame) -> KResult<()> {
        use std::slice::mut_ref_slice;
        let pgnum = pf.get_pagenum();
        self.read_from(pgnum, mut_ref_slice(pf.get_page_mut())).map(|_| ())
    }

    /**
     * Since this is just a drive we do nothing.
     */
    fn dirty_page(&mut self, _: &PFrame) -> KResult<()> { Ok(()) }

    /**
     * Write the contents of the page frame
     */
    fn clean_page(&mut self, pf: &PFrame) -> KResult<()> {
        use std::slice::ref_slice;
        let pgnum = pf.get_pagenum();
        self.write_to(pgnum, ref_slice(pf.get_page())).map(|_| ())
    }

    fn show(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "{:?}", self) }
}

impl Cacheable for ATADisk { fn is_still_useful(&self) -> bool { true } }

extern "Rust" fn ata_intr_handler(r: &mut interrupt::Registers) {
    dbg!(debug::DISK, "ATA Interrupt for {}", r.intr);
    unsafe {
        for &i in DISKS.iter() {
            let d = i.as_mut().expect("No disks should be uninitialized");
            if (d.channel.intr as u32) == r.intr {
                d.handle_interrupt();
                d.channel.inb(register::STATUS);
                return;
            }
        }
    }
    kpanic!("Recieved an interrupt for a disk on an unknown channel");
}

