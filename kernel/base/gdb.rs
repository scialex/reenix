// TODO Copyright Header

//! A bunch of hook definitions used by gdb to tell when we have reached certain stages. These
//! should never contain any code.

/// A function to tell GDB we have booted
#[allow(dead_code)] #[inline(never)] #[no_stack_check]
#[export_name="__py_hook_boot"]
pub extern "C" fn boot_hook() {}

/// A function to tell GDB we have reached the idle proc
#[allow(dead_code)] #[inline(never)] #[no_stack_check]
#[export_name="__py_hook_initialized"]
pub extern "C" fn initialized_hook() {}

/// A function to tell GDB we are shutting down.
#[allow(dead_code)] #[inline(never)] #[no_stack_check]
#[export_name="__py_hook_shutdown"]
pub extern "C" fn shutdown_hook() {}
