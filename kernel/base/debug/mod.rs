// TODO Copyright Header

//! All the debug stuff.

#![macro_escape]

pub use self::flags::*;

pub mod printing;
mod flags;


pub static mut dbg_active : DbgMode = ALL;

// TODO At the moment rust lacks any analogue to C's __func__. If there is one added be sure to use
// it.

/// Directly print a formated string to the debug port.
#[macro_export]
macro_rules! dbg_write(
    ($fmt:expr, $($a:expr),*) => { write!(::base::debug::printing::DEBUG_WRITER, $fmt, $($a),*) }
)

#[macro_export]
macro_rules! dbger(
    ($d:expr, $err:expr, $fmt:expr, $($a:expr),*) => {(
        if (::base::debug::dbg_active & ($d)) != 0 {
            dbg_write!("{s}{s}-{s}:{s} <errno:{s}> : ", ($d as ::base::debug::dbg_mode).get_color(), ($d as ::base::debug::dbg_mode), file!(), line!(), $err);
            dbg_write!($fmt, $($a),*);
            dbg_write!("{s}\n", base::debug::color::NORMAL);
        }
    )}
)

#[macro_export]
macro_rules! dbg(
    ($d:expr, $fmt:expr, $($a:expr),*) => {(
        if (::base::debug::dbg_active & ($d)) != 0 {
            dbg_write!("{s}{s}-{s}:{s} : ", ($d as ::base::debug::dbg_mode).get_color(), ($d as ::base::debug::dbg_mode), file!(), line!());
            dbg_write!($fmt, $($a),*);
            dbg_write!("{s}\n", base::debug::color::NORMAL);
        }
    )}
)

#[macro_export]
macro_rules! panic(
    ($fmt:expr, $($a:expr),*) => {
        dbg_write!("{s}{s}-{s}:{s} : ", ($d as ::base::debug::dbg_mode).get_color(), ($d as ::base::debug::dbg_mode), file!(), line!());
        dbg_write!($fmt, $($a),*);
        dbg_write!("{s}\n", base::debug::color::NORMAL);
        ::base::kernel::halt();
    }
)
