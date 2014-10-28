// TODO Copyright Header
//

//! Very low level io functions

#[inline]
pub unsafe fn outb(port: u16, v: u8) {
    asm!("outb %al, %dx" : : "{al}"(v), "{dx}"(port) : : "volatile");
}

#[inline]
pub unsafe fn outw(port: u16, v: u16) {
    asm!("outw %ax, %dx" : : "{ax}"(v), "{dx}"(port) : : "volatile");
}

#[inline]
pub unsafe fn outl(port: u16, v: u32) {
    asm!("outl %eax, %dx" : : "{eax}"(v), "{dx}"(port) : : "volatile");
}

// NOTE For some reason LLVM doesn't understand (or can't handle) the standard register
// specifiers. We need to use these exact specifiers.
#[inline]
pub unsafe fn inb(port: u16) -> u8 {
    let ret : u8;
    asm!("inb $1, $0" : "={al}"(ret) : "{dx}"(port) : "eax", "edx" : "volatile");
    return ret;
}

#[inline]
pub unsafe fn inw(port: u16) -> u16 {
    let ret : u16;
    asm!("inw $1, $0" : "={ax}"(ret) : "{dx}"(port) : "eax", "edx" : "volatile");
    return ret;
}

#[inline]
pub unsafe fn inl(port: u16) -> u32 {
    let ret : u32;
    asm!("inl $1, $0" : "={eax}"(ret) : "{dx}"(port) : "eax", "edx" : "volatile");
    return ret;
}
