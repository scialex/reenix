// TODO Copyright Header

//! actually printing debug info.

use core::fmt::*;
use base::io::{inb,outb}

pub struct DbgWriter {}

#[feature(macro_rules)]

pub static PORT : u16 = 0x3f8;
pub static PORT_INTR : u8 = 0x0d;
pub static DBG_WRITER : DbgWriter = DbgWriter {};

impl FormatWriter for DbgWriter {
    fn write(&mut self, data: &[u8]) {
        for x in data.iter() {
            unsafe {
                while (inb(PORT + 5) & 0x20 == 0) {}
                outb(PORT, x);
            }
        }
    }
}

