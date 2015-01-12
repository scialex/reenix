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
        use core::mem;
        if alloc::is_memory_low() {
            Err(())
        } else {
            let x = $f;
            if alloc::is_memory_low() {
                mem::drop(x);
                Err(())
            } else {
                Ok(x)
            }
        }
    });
    (try_box $e:expr) => ({
        use $crate::alloc;
        use core::mem;
        if alloc::is_memory_low() {
            Err(())
        } else {
            let x = box $e;

            if alloc::is_memory_low() {
                mem::drop(x);
                Err(())
            } else {
                Ok(x)
            }
        }
    });
}
