// TODO Copyright Header

//! Macro's for failable allocation

/// Allocates a value. If it seems that we are likely to run out of memory we will return
/// Err(AllocError), otherwise Ok(val). It is not nessecarially gaurenteed to work.
///
/// NOTE This is super dangerous and very bad. It is only being done because we cannot use the
/// normal method of detecting alloc failures (namely new_task that returns value or fails if it
/// can't).
#[macro_export]
macro_rules! alloc {
    // TODO This is rather wordy and arbitrary.
    (try $f:expr) => ({
        use $crate::alloc;
        use ::std::mem;
        if alloc::is_memory_low() {
            dbg!(debug::CORE|debug::MM, "memory low before: {}", stringify!($f));
            Err(alloc::AllocError)
        } else {
            let x = $f;
            if alloc::is_memory_low() {
                dbg!(debug::CORE|debug::MM, "memory low after: {}", stringify!($f));
                mem::drop(x);
                Err(alloc::AllocError)
            } else {
                Ok(x)
            }
        }
    });
    (try_box $e:expr) => ({
        use $crate::alloc;
        use ::std::mem;
        if alloc::is_memory_low() {
            dbg!(debug::CORE|debug::MM, "memory low before: {}", stringify!($e));
            Err(alloc::AllocError)
        } else {
            let x = box $e;

            if alloc::is_memory_low() {
                dbg!(debug::CORE|debug::MM, "memory low after: {}", stringify!($e));
                mem::drop(x);
                Err(alloc::AllocError)
            } else {
                Ok(x)
            }
        }
    });
}
