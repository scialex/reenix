
#![macro_escape]
// TODO At the moment rust lacks any analogue to C's __func__. If there is one added be sure to use
// it.

/// Directly print a formated string to the debug port.
#[macro_export]
macro_rules! dbg_write(
    ($fmt:expr, $($a:expr),*) => ({
        use core::result::Err;
        use base::debug::printing::{DBG_WRITER, DbgWriter};
        use core::fmt::FormatWriter;
        #[inline(always)] #[allow(dead_code)] fn get_writer() -> DbgWriter { unsafe { DBG_WRITER } }
        match write!(&mut get_writer(), $fmt, $($a),*) {
            Err(_) => (),
            _ => (),
        }
    })
)

#[macro_export]
macro_rules! dbger(
    ($d:expr, $err:expr, $fmt:expr, $($a:expr),*) => ({
        use base::debug;
        if (debug::get_debug_active() & ($d)) != debug::NONE {
            dbg_write!("{}{} {}:{:u} <errno:{}> : ", $d.get_color(), $d, file!(), line!(), $err);
            dbg_write!($fmt, $($a),*);
            dbg_write!("{}\n", debug::color::NORMAL);
        }
    });
    ($d:expr, $err:expr, $fmt:expr) => ({
        dbger!($d, $err, "{}", $fmt);
    })
)

#[macro_export]
macro_rules! dbg(
    ($d:expr, $fmt:expr, $($a:expr),*) => ({
        use base::debug;
        if (debug::get_debug_active() & ($d)) != debug::NONE {
            dbg_write!("{}{}-{}:{:u} : ", $d.get_color(), $d, file!(), line!());
            dbg_write!($fmt, $($a),*);
            dbg_write!("{}\n", debug::color::NORMAL);
        }
    });
    ($d:expr, $fmt:expr) => ({
        dbg!($d, "{}", $fmt);
    })
)

#[macro_export]
macro_rules! panic(
    ($fmt:expr, $($a:expr),*) => ({
        use base::debug;
        use base::kernel;
        dbg_write!("{}{}-{}:{:u} : ", debug::PANIC.get_color(), debug::PANIC, file!(), line!());
        dbg_write!($fmt, $($a),* );
        dbg_write!("{}\n", debug::color::NORMAL);
        kernel::halt();
    });

    ($fmt:expr) => ({
        panic!("{}", $fmt);
    });
)

