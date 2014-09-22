// TODO Copyright Header

//! All the debug stuff.

#![macro_escape]

/// Reexport flags.
pub use self::flags::*;

pub mod printing;
mod flags;

pub static mut dbg_active : DbgMode = ALL;

mod macros;

//mod langs;

