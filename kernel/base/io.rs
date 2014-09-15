// TODO Copyright Header
//

//! Very low level io functions

#![feature(asm)]

#[inline] pub unsafe fn outb(port: u16, v: u8) {
    asm!("outb $0, $1" : : "a"(v), "Nd"(port) : : "volatile");
}

#[inline] pub unsafe fn outw(port: u16, v: u16) {
    asm!("outw $0, $1" : : "a"(v), "Nd"(port) : : "volatile");
}

#[inline] pub unsafe fn outl(port: u16, v: u32) {
    asm!("outl $0, $1" : : "a"(v), "Nd"(port) : : "volatile");
}

#[inline] pub unsafe fn inb(port: u16) -> u8 {
    let ret : u8 = 0;
    asm!("inb $1, $0" : "=a"(ret) : "Nd"(port) : : "volatile");
    return ret;
}

#[inline] pub unsafe fn inw(port: u16) -> u16 {
    let ret : u16 = 0;
    asm!("inw $1, $0" : "=a"(ret) : "Nd"(port) : : "volatile");
    return ret;
}

#[inline] pub unsafe fn inl(port: u16) -> u32 {
    let ret : u32 = 0;
    asm!("inl $1, $0" : "=a"(ret) : "Nd"(port) : : "volatile");
    return ret;
}
