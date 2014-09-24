
#[phase(plugin, link)] extern crate base;
extern crate libc;

use core::iter::*;


#[no_split_stack]
fn clear_screen(background: u16) {
    for i in range(0u, 80 * 25) {
        unsafe {
            *((0xb8000 + i * 2) as *mut u16) = background << 12;
        }
    }
}

extern "C" {
    fn dbg_init();
    fn page_init();
    fn pt_init();
    fn slab_init();
    // TODO I don't think I need this yet, or ever.
    // fn pframe_init();
    fn acpi_init();
    fn apic_init();
    fn gdt_init();
}

unsafe fn run_c_init() {
    dbg_init();
    page_init();
    pt_init();
    slab_init();
    acpi_init();
    apic_init();
    gdt_init();
}

fn run_rust_init() {
    use mm;
    mm::alloc::init();
}

#[no_mangle]
#[no_split_stack]
pub extern "C" fn kmain2() {
    unsafe { run_c_init(); }
    dbg!(debug::MM, "hi {}", "debugging");
    clear_screen(13);
    loop {}
}
