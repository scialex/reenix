
use core::option::*;
use core::iter::*;

#[no_split_stack]
fn clear_screen(background: u16) {
    for i in range(0u, 80 * 25) {
        unsafe {
            *((0xb8000 + i * 2) as *mut u16) = background << 12;
        }
    }
}

#[no_mangle]
#[no_split_stack]
pub extern "C" fn kmain2() {
    clear_screen(13);
    loop {}
}
