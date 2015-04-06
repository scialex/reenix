
//! The Reenix DMA stuff

use libc::c_void;
use base::io;

mod register {
    pub const COMMAND: u8 = 0;
    pub const STATUS : u8 = 0x2;
    pub const PRD    : u8 = 0x4;
}

#[repr(C, packed)]
pub struct Prd {
    pub addr: u32,
    pub count: u16,
    pub last : u16,
    pub buf : [u8; 128],
}

pub fn init_stage1() { }
pub fn init_stage2() { }

impl Prd {
    pub fn load(&mut self, start: *const c_void, count: u16) {
        self.addr = (current_proc!()).get_pagedir().virt_to_phys(start as usize) as u32;
        self.count = count;
        self.last = 0x8000;
    }

    pub fn start(&mut self, busmaster_addr: u16, write: bool) {
        use std::intrinsics::copy_nonoverlapping;
        // Set the read/write bit.
        let cmd: u8 = if write { 0b101 } else { 0b001 };
        let pd = (current_proc!()).get_pagedir();
        unsafe {
            // TODO This might be really REALLY bad. If so we will need to do something else.
            // We cannot really be sure of the alignment of self (partly due to redzoning). Lets do this instead.
            // This might well be the ugliest hack I've ever written...
            let mut pbuf = self.buf.as_mut_ptr();
            while (pbuf as usize) % 32 != 0 { pbuf = pbuf.offset(1); }
            copy_nonoverlapping(pbuf as *mut u8, (self as *mut Prd) as *const u8, 8);
            // Set the address of the prd.
            io::outl(busmaster_addr + (register::PRD as u16), pd.virt_to_phys(pbuf as usize) as u32);
            // allow all chanels of dma on this busmaster by setting the status register
            io::outb(busmaster_addr + (register::STATUS as u16), io::inb(busmaster_addr + (register::STATUS as u16)) | 0x60);
            // Set the start/stop bit
            io::outb(busmaster_addr + (register::COMMAND as u16), cmd);
        }
    }

    pub fn reset(&mut self, busmaster_addr : u16) {
        /* to acknowledge the interrupt we need to both
         * read the status register and write 0x64 to it.
         * the 0x64 resets the interrupts somehow while
         * keeping DMA enabled
         */
        unsafe {
            io::inb(busmaster_addr + (register::STATUS as u16));
            io::outb(busmaster_addr + (register::STATUS as u16), 0x64);
            // Also clear the start bit of the command register.
            io::outb(busmaster_addr + (register::COMMAND as u16), 0x00);
            self.buf = [0; 128];
        }
    }
}
