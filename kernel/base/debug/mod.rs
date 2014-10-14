// TODO Copyright Header

//! All the debug stuff.

#![macro_escape]

/// Reexport flags.
pub use self::flags::*;

pub mod printing;
mod flags;

pub const DBG_ACTIVE : DbgMode = ALL;

mod macros;

//mod langs;

