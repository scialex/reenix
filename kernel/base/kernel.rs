// TODO Copyright Header

//! The Reenix base kernel-CPU Interface stuff


use libc::c_void;

/// The linker script will initialize these symbols. Note
/// that the linker does not actually allocate any space
/// for these variables (thus the void type) it only sets
/// the address that the symbol points to. So for example
/// the address where the kernel ends is &kernel_end,
/// NOT kernel_end.
#[allow(dead_code)]
extern "C" {
    #[link_name="kernel_start"]
    pub static start : *const c_void;
    #[link_name="kernel_start_text"]
    pub static start_text : *const c_void;
    #[link_name="kernel_start_data"]
    pub static start_data : *const c_void;
    #[link_name="kernel_start_bss"]
    pub static start_bss : *const c_void;
    #[link_name="kernel_start_init"]
    pub static start_init: *const c_void;

    #[link_name="kernel_end"]
    pub static end : *const c_void;
    #[link_name="kernel_end_text"]
    pub static end_text : *const c_void;
    #[link_name="kernel_end_data"]
    pub static end_data : *const c_void;
    #[link_name="kernel_end_bss"]
    pub static end_bss : *const c_void;
    #[link_name="kernel_end_init"]
    pub static end_init: *const c_void;
}

// TODO I maybe should move this to a different module.
/// This stops everything.
#[no_stack_check]
#[inline]
#[export_name="hard_shutdown"]
pub fn halt() -> ! {
    ::gdb::shutdown_hook();
    unsafe {
        asm!("cli; hlt");
    }
    loop {}
}

extern "C" {
    #[link_name="do_c_ndelay"]
    pub fn ndelay(n: u32);
}

