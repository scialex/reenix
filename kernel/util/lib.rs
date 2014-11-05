// TODO Copyright Header

#![crate_name="util"]
#![crate_type="rlib"]

#![no_std]
#![feature(phase, globs, macro_rules, asm, if_let, default_type_params, unsafe_destructor, tuple_indexing)]

#[phase(link, plugin)] extern crate core;
#[phase(link, plugin)] extern crate base;
#[phase(link, plugin)] extern crate collections;
#[phase(link, plugin)] extern crate mm;
extern crate alloc;
extern crate libc;

use collections::String;
use core::prelude::*;
use core::fmt;

pub fn init_stage1() {
    lru_cache::init_stage1();
}
pub fn init_stage2() {
    lru_cache::init_stage1();
}
pub fn init_stage3() {
    lru_cache::init_stage1();
    // TODO Mark we now have a pid.
    // TODO Make it so that dbg will use an extern function to get the pid number.
    // TODO (i.e. extern fn() -> &'static Show;) defined as transmute(current_proc!().get_pid())
}

#[macro_export]
macro_rules! format(
    ($($arg:tt)*) => ({
        use util;
        let f = util:::mk_string_formatter();
        match write!(f, $($arg)*) {
            Err(_) => String::from_str("*** FORMAT ERROR ***"),
            Ok(_)  => f.0,
        }
    })
)

pub struct StringFormatter(String);

pub fn mk_string_formatter() -> StringFormatter { StringFormatter(String::with_capacity(256)) }

impl fmt::FormatWriter for StringFormatter {
    fn write(&mut self, bytes: &[u8]) -> fmt::Result {
        use core::str::from_utf8;
        from_utf8(bytes).map_or(Err(fmt::WriteError), |st| { self.0.push_str(st); Ok(()) })
    }
}

pub mod lru_cache;

mod std {
    pub use core::clone;
    pub use core::cmp;
    pub use core::fmt;
    pub use core::num;
    pub use core::option;
    pub use collections::hash;
}
