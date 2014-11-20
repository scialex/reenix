// TODO Copyright Header

//! actually printing debug info.

use core;
use core::result::Ok;
use core::slice::*;
use io;

/// The struct which can print to the io port we look at for debug information.
pub struct DbgWriter;

pub static PORT : u16 = 0x3f8;
pub static PORT_INTR : u8 = 0x0d;

/// The specific writer we use for debug printing.
pub static mut DBG_WRITER : DbgWriter = DbgWriter;

impl core::fmt::FormatWriter for DbgWriter {
    #[no_stack_check]
    fn write(&mut self, data: &[u8]) -> core::fmt::Result {
        for &x in data.iter() {
            unsafe {
                while io::inb(PORT + 5) & 0x20 == 0 {}
                io::outb(PORT, x);
            }
        }
        Ok(())
    }
}

