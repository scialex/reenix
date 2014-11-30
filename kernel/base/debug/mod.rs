// TODO Copyright Header

//! All the debug stuff.

#![macro_escape]

/// Reexport flags.
pub use self::flags::*;
use core::fmt;

pub mod printing;
mod flags;

/// The currently printed debug modes.
static mut DBG_ACTIVE : DbgMode = ALL;

/// Gets the currently printed debug modes.
#[inline]
pub fn get_debug_active() -> DbgMode { unsafe { DBG_ACTIVE } }

#[doc(hidden)]
pub fn setup() {
    unsafe {
        DBG_ACTIVE = flags::DbgMode::get_default();
    }
}

/// Sets a mode as one that should not be printed.
pub fn remove_mode(m: DbgMode) {
    unsafe { DBG_ACTIVE = DBG_ACTIVE - m; }
}

/// Sets a mode as being one that should be printed
pub fn add_mode(m: DbgMode) {
    unsafe { DBG_ACTIVE = DBG_ACTIVE + m; }
}

mod macros;

extern "C" {
    /// A function which can be used to get the current pid number to aid in debuging.
    fn get_dbg_pid() -> &'static fmt::Show;
}
#[doc(hidden)]
pub fn dbg_pid() -> &'static (fmt::Show + 'static) {
    unsafe { get_dbg_pid() }
}

#[no_stack_check]
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

