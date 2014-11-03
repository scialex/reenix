
//! The line discipline

use procs::sync::*;
use core::prelude::*;
use base::errno;
use base::errno::KResult;
use core::cmp;
use RDevice;
use bytedev::tty::TTYLineDiscipline;

pub const LINE_BUF_SIZE : uint = 256;

// All the charecters as strings.
const CHARS : [&'static str, ..256] = [
    "",     "\x01", "\x02", "\x03", "\x04", "\x05", "\x06", "\x07", "\x08", "\x09", "\x0a", "\x0b", "\x0c", "\x0d", "\x0e", "\x0f",
    "\x10", "\x11", "\x12", "\x13", "\x14", "\x15", "\x16", "\x17", "\x18", "\x19", "\x1a", "\x1b", "\x1c", "\x1d", "\x1e", "\x1f",
    "\x20", "\x21", "\x22", "\x23", "\x24", "\x25", "\x26", "\x27", "\x28", "\x29", "\x2a", "\x2b", "\x2c", "\x2d", "\x2e", "\x2f",
    "\x30", "\x31", "\x32", "\x33", "\x34", "\x35", "\x36", "\x37", "\x38", "\x39", "\x3a", "\x3b", "\x3c", "\x3d", "\x3e", "\x3f",
    "\x40", "\x41", "\x42", "\x43", "\x44", "\x45", "\x46", "\x47", "\x48", "\x49", "\x4a", "\x4b", "\x4c", "\x4d", "\x4e", "\x4f",
    "\x50", "\x51", "\x52", "\x53", "\x54", "\x55", "\x56", "\x57", "\x58", "\x59", "\x5a", "\x5b", "\x5c", "\x5d", "\x5e", "\x5f",
    "\x60", "\x61", "\x62", "\x63", "\x64", "\x65", "\x66", "\x67", "\x68", "\x69", "\x6a", "\x6b", "\x6c", "\x6d", "\x6e", "\x6f",
    "\x70", "\x71", "\x72", "\x73", "\x74", "\x75", "\x76", "\x77", "\x78", "\x79", "\x7a", "\x7b", "\x7c", "\x7d", "\x7e", "\x7f",
    "\x80", "\x81", "\x82", "\x83", "\x84", "\x85", "\x86", "\x87", "\x88", "\x89", "\x8a", "\x8b", "\x8c", "\x8d", "\x8e", "\x8f",
    "\x90", "\x91", "\x92", "\x93", "\x94", "\x95", "\x96", "\x97", "\x98", "\x99", "\x9a", "\x9b", "\x9c", "\x9d", "\x9e", "\x9f",
    "\xa0", "\xa1", "\xa2", "\xa3", "\xa4", "\xa5", "\xa6", "\xa7", "\xa8", "\xa9", "\xaa", "\xab", "\xac", "\xad", "\xae", "\xaf",
    "\xb0", "\xb1", "\xb2", "\xb3", "\xb4", "\xb5", "\xb6", "\xb7", "\xb8", "\xb9", "\xba", "\xbb", "\xbc", "\xbd", "\xbe", "\xbf",
    "\xc0", "\xc1", "\xc2", "\xc3", "\xc4", "\xc5", "\xc6", "\xc7", "\xc8", "\xc9", "\xca", "\xcb", "\xcc", "\xcd", "\xce", "\xcf",
    "\xd0", "\xd1", "\xd2", "\xd3", "\xd4", "\xd5", "\xd6", "\xd7", "\xd8", "\xd9", "\xda", "\xdb", "\xdc", "\xdd", "\xde", "\xdf",
    "\xe0", "\xe1", "\xe2", "\xe3", "\xe4", "\xe5", "\xe6", "\xe7", "\xe8", "\xe9", "\xea", "\xeb", "\xec", "\xed", "\xee", "\xef",
    "\xf0", "\xf1", "\xf2", "\xf3", "\xf4", "\xf5", "\xf6", "\xf7", "\xf8", "\xf9", "\xfa", "\xfb", "\xfc", "\xfd", "\xfe", "\xff"
];

// All the charecters as strings prepended by a backspace.
const DELCHARS : [&'static str, ..256] = [
    "",         "\x08\x01", "\x08\x02", "\x08\x03", "\x08\x04", "\x08\x05", "\x08\x06", "\x08\x07",
    "\x08\x08", "\x08\x09", "\x08\x0a", "\x08\x0b", "\x08\x0c", "\x08\x0d", "\x08\x0e", "\x08\x0f",
    "\x08\x10", "\x08\x11", "\x08\x12", "\x08\x13", "\x08\x14", "\x08\x15", "\x08\x16", "\x08\x17",
    "\x08\x18", "\x08\x19", "\x08\x1a", "\x08\x1b", "\x08\x1c", "\x08\x1d", "\x08\x1e", "\x08\x1f",
    "\x08\x20", "\x08\x21", "\x08\x22", "\x08\x23", "\x08\x24", "\x08\x25", "\x08\x26", "\x08\x27",
    "\x08\x28", "\x08\x29", "\x08\x2a", "\x08\x2b", "\x08\x2c", "\x08\x2d", "\x08\x2e", "\x08\x2f",
    "\x08\x30", "\x08\x31", "\x08\x32", "\x08\x33", "\x08\x34", "\x08\x35", "\x08\x36", "\x08\x37",
    "\x08\x38", "\x08\x39", "\x08\x3a", "\x08\x3b", "\x08\x3c", "\x08\x3d", "\x08\x3e", "\x08\x3f",
    "\x08\x40", "\x08\x41", "\x08\x42", "\x08\x43", "\x08\x44", "\x08\x45", "\x08\x46", "\x08\x47",
    "\x08\x48", "\x08\x49", "\x08\x4a", "\x08\x4b", "\x08\x4c", "\x08\x4d", "\x08\x4e", "\x08\x4f",
    "\x08\x50", "\x08\x51", "\x08\x52", "\x08\x53", "\x08\x54", "\x08\x55", "\x08\x56", "\x08\x57",
    "\x08\x58", "\x08\x59", "\x08\x5a", "\x08\x5b", "\x08\x5c", "\x08\x5d", "\x08\x5e", "\x08\x5f",
    "\x08\x60", "\x08\x61", "\x08\x62", "\x08\x63", "\x08\x64", "\x08\x65", "\x08\x66", "\x08\x67",
    "\x08\x68", "\x08\x69", "\x08\x6a", "\x08\x6b", "\x08\x6c", "\x08\x6d", "\x08\x6e", "\x08\x6f",
    "\x08\x70", "\x08\x71", "\x08\x72", "\x08\x73", "\x08\x74", "\x08\x75", "\x08\x76", "\x08\x77",
    "\x08\x78", "\x08\x79", "\x08\x7a", "\x08\x7b", "\x08\x7c", "\x08\x7d", "\x08\x7e", "\x08\x7f",
    "\x08\x80", "\x08\x81", "\x08\x82", "\x08\x83", "\x08\x84", "\x08\x85", "\x08\x86", "\x08\x87",
    "\x08\x88", "\x08\x89", "\x08\x8a", "\x08\x8b", "\x08\x8c", "\x08\x8d", "\x08\x8e", "\x08\x8f",
    "\x08\x90", "\x08\x91", "\x08\x92", "\x08\x93", "\x08\x94", "\x08\x95", "\x08\x96", "\x08\x97",
    "\x08\x98", "\x08\x99", "\x08\x9a", "\x08\x9b", "\x08\x9c", "\x08\x9d", "\x08\x9e", "\x08\x9f",
    "\x08\xa0", "\x08\xa1", "\x08\xa2", "\x08\xa3", "\x08\xa4", "\x08\xa5", "\x08\xa6", "\x08\xa7",
    "\x08\xa8", "\x08\xa9", "\x08\xaa", "\x08\xab", "\x08\xac", "\x08\xad", "\x08\xae", "\x08\xaf",
    "\x08\xb0", "\x08\xb1", "\x08\xb2", "\x08\xb3", "\x08\xb4", "\x08\xb5", "\x08\xb6", "\x08\xb7",
    "\x08\xb8", "\x08\xb9", "\x08\xba", "\x08\xbb", "\x08\xbc", "\x08\xbd", "\x08\xbe", "\x08\xbf",
    "\x08\xc0", "\x08\xc1", "\x08\xc2", "\x08\xc3", "\x08\xc4", "\x08\xc5", "\x08\xc6", "\x08\xc7",
    "\x08\xc8", "\x08\xc9", "\x08\xca", "\x08\xcb", "\x08\xcc", "\x08\xcd", "\x08\xce", "\x08\xcf",
    "\x08\xd0", "\x08\xd1", "\x08\xd2", "\x08\xd3", "\x08\xd4", "\x08\xd5", "\x08\xd6", "\x08\xd7",
    "\x08\xd8", "\x08\xd9", "\x08\xda", "\x08\xdb", "\x08\xdc", "\x08\xdd", "\x08\xde", "\x08\xdf",
    "\x08\xe0", "\x08\xe1", "\x08\xe2", "\x08\xe3", "\x08\xe4", "\x08\xe5", "\x08\xe6", "\x08\xe7",
    "\x08\xe8", "\x08\xe9", "\x08\xea", "\x08\xeb", "\x08\xec", "\x08\xed", "\x08\xee", "\x08\xef",
    "\x08\xf0", "\x08\xf1", "\x08\xf2", "\x08\xf3", "\x08\xf4", "\x08\xf5", "\x08\xf6", "\x08\xf7",
    "\x08\xf8", "\x08\xf9", "\x08\xfa", "\x08\xfb", "\x08\xfc", "\x08\xfd", "\x08\xfe", "\x08\xff"
];

pub fn init_stage1() {}
pub fn init_stage2() {}

pub struct LineDiscipline {
    rlock : SMutex,
    buf   : [u8, ..LINE_BUF_SIZE],
    rhead : uint,
    raw_tail : uint,
    ckd_tail : uint,
}

impl LineDiscipline {
    pub fn create() -> LineDiscipline {
        LineDiscipline {
            rlock : SMutex::new("Line discipline mutex"),
            buf : [0, ..LINE_BUF_SIZE],
            rhead : 0,
            raw_tail : 0,
            ckd_tail : 0,
        }
    }

    /// Return true if we could push the char, false otherwise.
    fn push_char(&mut self, chr: u8) -> bool {
        match self.cbuf_next() {
            None => {
                // We don't have room at the end. Delete the last char and store it there.
                self.buf[cmp::min(self.raw_tail - 1, LINE_BUF_SIZE - 1)] = chr;
                false
            },
            Some(v) => {
                self.buf[self.raw_tail] = chr;
                self.raw_tail = v;
                true
            },
        }
    }

    fn cbuf_next(&self) -> Option<uint> {
        let next = (self.raw_tail + 1) % LINE_BUF_SIZE;
        if next == self.rhead {
            None
        } else {
            Some(next)
        }
    }
    fn cbuf_prev(&self) -> Option<uint> {
        let prev = cmp::min(self.raw_tail - 1, LINE_BUF_SIZE - 1);
        if self.raw_tail == self.ckd_tail {
            None
        } else {
            Some(prev)
        }
    }
}

impl RDevice<u8> for LineDiscipline {
    fn read_from(&mut self, _: uint, b: &mut [u8]) -> KResult<uint> {
        let t = try!(self.rlock.lock().or_else(|_| Err(errno::EINTR)));
        while self.rhead == self.ckd_tail {
            try!(t.wait().or_else(|_| Err(errno::EINTR)));
        }

        for i in range(0, b.len()) {
            if self.rhead == self.ckd_tail { return Ok(i); }

            let c = self.buf[self.rhead];
            self.rhead = (self.rhead + 1) % LINE_BUF_SIZE;

            b[i] = c;
            match c as char {
                // EOL, we should return now.
                '\r' | '\n' | '\x04' => { return Ok(i + 1); },
                _ => {},
            }
        }
        return Ok(b.len());
    }
}

impl TTYLineDiscipline for LineDiscipline {
    /// Store that we recieved the given char and return a string to echo to the tty.
    fn recieve_char(&mut self, chr: u8) -> &'static str {
        assert!(self.ckd_tail < LINE_BUF_SIZE);
        assert!(self.raw_tail < LINE_BUF_SIZE);
        assert!(self.rhead    < LINE_BUF_SIZE);
        assert!(self.rhead == self.ckd_tail || {
                    let prev = cmp::min(self.ckd_tail - 1, LINE_BUF_SIZE - 1);
                    (self.buf[prev] as char) == '\n' || (self.buf[prev] as char) == '\r' || (self.buf[prev] as char) == '\x04'
                });
        match chr as char {
            '\x7f' | '\x08' => { if let Some(prev) = self.cbuf_prev() { self.raw_tail = prev; "\x08 \x08" } else { "" } },
            '\0' => "",
            '\t' => {
                if !self.push_char(' ' as u8) { "\x08 "
                } else if !self.push_char(' ' as u8) { " "
                } else if !self.push_char(' ' as u8) { "  "
                } else if !self.push_char(' ' as u8) { "   "
                } else { "    " }
            },
            '\r' | '\n' | '\x04' => {
                let res = self.push_char(chr);
                self.ckd_tail = self.raw_tail;
                self.rlock.signal();
                if res { CHARS[chr as uint] } else { DELCHARS[chr as uint] }
            },
            _ => { if self.push_char(chr) { CHARS[chr as uint] } else { DELCHARS[chr as uint] } },
        }
    }

    /// Process a char that was written to the tty so it is suitable to be outputted to the tty.
    fn process_char(&self, chr: u8) -> &'static str {
        match chr as char {
            '\n' => "\n",
            '\r' => "\n",
            '\t' => "    ",
            '\x7f' | '\x08' => "\x08 \x08",
            _ => CHARS[chr as uint],
        }
    }
}
