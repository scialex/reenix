// TODO Copyright Header

//! All the debug stuff.

#![macro_escape]

/// Reexport flags.
pub use self::flags::*;

pub mod printing;
mod flags;

static mut DBG_ACTIVE : DbgMode = ALL;
pub fn get_debug_active() -> DbgMode {
    unsafe { DBG_ACTIVE }
}

pub fn setup() {
    unsafe {
        DBG_ACTIVE = flags::DbgMode::get_default();
    }
}

pub fn remove_mode(m: DbgMode) {
    unsafe { DBG_ACTIVE = DBG_ACTIVE - m; }
}

pub fn add_mode(m: DbgMode) {
    unsafe { DBG_ACTIVE = DBG_ACTIVE + m; }
}

mod macros;

//mod langs;

