// TODO Copyright Header

//! All the debug stuff.

#[feature(macro_rules)]
#![macro_escape]

pub use flags::*;
pub mod printing;
pub static mut dbg_active : dbg_modes = ALL;
macro_rules! dbg(
    ($d:expr, $fmt:expr, $($a:expr),*) => {
        if (::base::debug::dbg_active & dbg_modes != 0) {
            write!(::base::debug::printing::DEBUG_WRITER, "{s}{s}-{s}:{s} {s}(): ", (d as ::base::debug::dbg_flags).get_color(), (d as ::base::debug::dbg_flags), file!(), line!());
            write!(::base::debug::printing::DEBUG_WRITER, $fmt, $($a),+);
            write!(::base::debug::printing::DEBUG_WRITER, "{s}\n", base::debug::color::NORMAL);
        }
    }
)

