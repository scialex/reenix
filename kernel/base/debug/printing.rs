// TODO Copyright Header

//! actually printing debug info.

use core::prelude::*;
use io;

/// The struct which can print to the io port we look at for debug information.
#[derive(Clone, Copy)]
pub struct DbgWriter;

pub static PORT : u16 = 0x3f8;
pub static PORT_INTR : u8 = 0x0d;

/// The specific writer we use for debug printing.
pub static mut DBG_WRITER : DbgWriter = DbgWriter;

impl ::core::fmt::Write for DbgWriter {
    #[no_stack_check]
    fn write_str(&mut self, s: &str) -> ::core::fmt::Result {
        for x in s.bytes() {
            unsafe {
                while io::inb(PORT + 5) & 0x20 == 0 {}
                io::outb(PORT, x);
            }
        }
        Ok(())
    }
}

