
#![macro_escape]
// TODO At the moment rust lacks any analogue to C's __func__. If there is one added be sure to use
// it.

/// Directly print a formated string to the debug port.
#[macro_export]
macro_rules! dbg_write(
    ($fmt:expr, $($a:expr),*) => ( unsafe {
        use core::fmt::FormatWriter;
        write!(&mut base::debug::printing::DBG_WRITER as &mut FormatWriter, $fmt, $($a),*);
    })
)

#[macro_export]
macro_rules! dbger(
    ($d:expr, $err:expr, $fmt:expr, $($a:expr),*) => ({
        if (base::debug::dbg_active & ($d)) != base::debug::NONE {
            dbg_write!("{}{} {}:{:u} <errno:{}> : ", ($d as base::debug::dbg_mode).get_color(), ($d as base::debug::dbg_mode), file!(), line!(), $err);
            dbg_write!($fmt, $($a),*);
            dbg_write!("{}\n", base::debug::color::NORMAL);
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
        if (debug::dbg_active & ($d)) != base::debug::NONE {
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
        dbg_write!("{}{}-{}:{:u} : ", base::debug::PANIC.get_color(), base::debug::PANIC, file!(), line!());
        dbg_write!($fmt, $($a),* );
        dbg_write!("{}\n", base::debug::color::NORMAL);
        base::kernel::halt();
    });

    ($fmt:expr) => ({
        panic!("{}", $fmt);
    });
)

