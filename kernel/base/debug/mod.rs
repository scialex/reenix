// TODO Copyright Header

//! All the debug stuff.

#![macro_escape]

/// Reexport flags.
pub use self::flags::*;
use core::fmt;

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

#[doc(hidden)]
#[inline(never)]
pub fn dbg_print(msg: &fmt::Arguments) {
    use core::result::Err;
    use debug::printing::DBG_WRITER;
    match fmt::write(unsafe { &mut DBG_WRITER }, msg) {
        Err(_) => (),
        _ => (),
    }
}

//mod langs;

