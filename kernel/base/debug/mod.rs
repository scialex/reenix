// TODO Copyright Header

//! All the debug stuff.

/// Reexport flags.
pub use self::flags::*;
use core::prelude::*;
use core::fmt;

#[macro_use]
mod macros;

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
    dbg!(debug::CORE, "removing mode {}", m);
    unsafe { DBG_ACTIVE = DBG_ACTIVE - m; }
}

/// Sets a mode as being one that should be printed
pub fn add_mode(m: DbgMode) {
    dbg!(debug::CORE, "adding mode {}", m);
    unsafe { DBG_ACTIVE = DBG_ACTIVE + m; }
}

#[allow(improper_ctypes)]
extern "C" {
    /// A function which can be used to get the current pid number to aid in debuging.
    fn get_dbg_pid() -> Option<::pid::ProcId>;
}

#[inline]
#[no_stack_check]
#[doc(hidden)]
pub fn dbg_pid() -> MaybePid {
    unsafe { MaybePid(get_dbg_pid()) }
}

#[doc(hidden)]
#[derive(Copy)]
pub struct MaybePid(Option<::pid::ProcId>);

impl fmt::Show for MaybePid {
    #[no_stack_check]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.0 {
            Some(v) => { write!(f, "{:?}", v) }
            None => { write!(f, "\x08") }
        }
    }
}

#[no_stack_check]
#[doc(hidden)]
#[inline(never)]
pub fn dbg_print(msg: fmt::Arguments) {
    use core::result::Result::Err;
    use debug::printing::DBG_WRITER;
    match fmt::write(unsafe { &mut DBG_WRITER }, msg) {
        Err(_) => (),
        _ => (),
    }
}

