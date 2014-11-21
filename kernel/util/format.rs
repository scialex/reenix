
//! A string format! implementation

use core::prelude::*;
use core::fmt;
use collections::String;
use core::str::from_utf8;

/// format a string
#[macro_export]
macro_rules! format(
    ($($arg:tt)*) => ({
        use util;
        let f = util::format::mk_string_formatter();
        match write!(f, $($arg)*) {
            Err(_) => String::from_str("*** FORMAT ERROR ***"),
            Ok(_)  => f.0,
        }
    })
)

/// A string formatter used internally to format a string
#[doc(hidden)]
pub struct StringFormatter(String);

#[doc(hidden)]
pub fn mk_string_formatter() -> StringFormatter { StringFormatter(String::with_capacity(256)) }

impl fmt::FormatWriter for StringFormatter {
    fn write(&mut self, bytes: &[u8]) -> fmt::Result {
        from_utf8(bytes).map_or(Err(fmt::Error), |st| { self.0.push_str(st); Ok(()) })
    }
}
