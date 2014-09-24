
#![macro_escape]
// TODO At the moment rust lacks any analogue to C's __func__. If there is one added be sure to use
// it.

/// Directly print a formated string to the debug port.
#[macro_export]
macro_rules! dbg_write(
    ($fmt:expr, $($a:expr),*) => ( unsafe {
        use core::result::Err;
        use base::debug::printing::DBG_WRITER;
        use core::fmt::FormatWriter;
        match write!(&mut DBG_WRITER as &mut FormatWriter, $fmt, $($a),*) {
            Err(_) => (),
            _ => (),
        }
    })
)

#[macro_export]
macro_rules! dbger(
    ($d:expr, $err:expr, $fmt:expr, $($a:expr),*) => ({
        use base::debug;
        if (debug::dbg_active & ($d)) != debug::NONE {
            dbg_write!("{}{} {}:{:u} <errno:{}> : ", ($d as debug::dbg_mode).get_color(), ($d as debug::dbg_mode), file!(), line!(), $err);
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
        if (debug::dbg_active & ($d)) != debug::NONE {
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

