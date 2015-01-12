
//! A string format! implementation

use core::prelude::*;
use core::fmt;
use collections::String;

/// format a string
#[macro_export]
macro_rules! format {
    ($($arg:tt)*) => ({
        use util;
        let f = util::format::mk_string_formatter();
        match write!(f, $($arg)*) {
            Err(_) => String::from_str("*** FORMAT ERROR ***"),
            Ok(_)  => f.0,
        }
    })
}

/// A string formatter used internally to format a string
#[doc(hidden)]
pub struct StringFormatter(String);

#[doc(hidden)]
pub fn mk_string_formatter() -> StringFormatter { StringFormatter(String::with_capacity(256)) }

impl fmt::Writer for StringFormatter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.push_str(s); Ok(())
    }
}
