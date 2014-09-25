
#[phase(plugin, link)] extern crate base;
extern crate libc;
extern crate alloc;
use alloc::boxed::*;

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
}

fn run_init() {
    use mm;
    use acpi;
    use apic;
    use gdt;
    unsafe { dbg_init(); }
    mm::init();
    acpi::init();
    apic::init();
    gdt::init();

    mm::alloc::close_requests();
}

#[no_mangle]
#[no_split_stack]
pub extern "C" fn kmain2() {
    /* TODO Export the symbols so I can run this.
    dbg!(debug::CORE, "Kernel binary:\n");
    dbg!(debug::CORE, "  text: 0x%p-0x%p\n", &kernel_start_text, &kernel_end_text);
    dbg!(debug::CORE, "  data: 0x%p-0x%p\n", &kernel_start_data, &kernel_end_data);
    dbg!(debug::CORE, "  bss:  0x%p-0x%p\n", &kernel_start_bss, &kernel_end_bss);
    */
    run_init();
    dbg!(debug::MM, "hi {}", "debugging");
    clear_screen(13);
    loop {}
}
