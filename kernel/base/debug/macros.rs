
#![macro_escape]
// TODO At the moment rust lacks any analogue to C's __func__. If there is one added be sure to use
// it.

/// Directly print a formated string to the debug port.
#[macro_export]
macro_rules! dbg_write(
    ($fmt:expr, $($a:expr),*) => ({
        use base::debug::dbg_print;
        format_args!(dbg_print, $fmt, $($a),*)
    })
)

#[macro_export]
macro_rules! dbger(
    ($d:expr, $err:expr, $fmt:expr, $($a:expr),*) => ({
        use base::debug;
        if (debug::get_debug_active() & ($d)) != debug::NONE {
            dbg_write!(concat!("{}{} {}:{:u} <errno:{}> : ", $fmt, "\n"),
                       $d.get_color(), $d, file!(), line!(), $err, $($a),*);
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
            dbg_write!(concat!("{}{} {}:{:u} : ", $fmt, "\n"),
                       $d.get_color(), $d, file!(), line!(), $($a),*);
        }
    });
    ($d:expr, $fmt:expr) => ({
        dbg!($d, "{}", $fmt);
    })
)

#[macro_export]
macro_rules! kpanic(
    ($fmt:expr, $($a:expr),*) => ({
        use base::debug;
        use base::kernel;
        dbg_write!(concat!("{}{} {}:{:u} : ", $fmt, "\n"),
                    debug::PANIC.get_color(), debug::PANIC, file!(), line!(), $($a),*);
        kernel::halt();
    });

    ($fmt:expr) => ({
        kpanic!("{}", $fmt);
    });
)

