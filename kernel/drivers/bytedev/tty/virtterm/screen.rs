
//! The Reenix screen writing code.

use std::ptr::write;
use base::io::outb;
use mm::pagetable;
use libc::uintptr_t;

pub fn init_stage1() {}
pub fn init_stage2() {
    unsafe { SCREEN.ram = pagetable::phys_perm_map(VIDEO_RAM, 1) as *mut u16; }
}

pub const DEFAULT_CHAR: u8 = 0x20;
// NOTE The const-expr propagator can't deal with casts!?
pub const UINT_DISPLAY_HEIGHT : usize = 25;
pub const UINT_DISPLAY_WIDTH  : usize = 80;
pub const DISPLAY_HEIGHT : u8 = 25;
pub const DISPLAY_WIDTH  : u8 = 80;
const VIDEO_RAM : uintptr_t = 0xb8000;
const CRT_CONTROL_ADDR : u16 = 0x3d4;
const CRT_CONTROL_DATA : u16 = 0x3d5;
const CURSOR_HIGH : u8 = 0x0e;
const CURSOR_LOW  : u8 = 0x0f;

const DEFAULT_ATTR : u8 = 0x0f;

static mut SCREEN : Screen = Screen { ram: 0 as *mut u16 };

#[inline]
pub fn get_screen() -> &'static Screen { unsafe { &SCREEN } }

pub struct Screen { ram: *mut u16, }

impl Screen {
    #[inline]
    pub unsafe fn move_cursor(&self, x: u8, y: u8) {
        assert!(x < DISPLAY_WIDTH && y < DISPLAY_HEIGHT);
        let pos : u16 = (y as u16) * (DISPLAY_WIDTH as u16) + (x as u16);

        outb(CRT_CONTROL_ADDR, CURSOR_HIGH);
        outb(CRT_CONTROL_DATA, (pos >> 8) as u8);

        /*  Output address being modified */
        outb(CRT_CONTROL_ADDR, CURSOR_LOW);
        /* New position of cursor */
        outb(CRT_CONTROL_DATA, (pos & 0xff) as u8);
    }

    #[inline]
    pub unsafe fn put_char(&self, c: u8, x: u8, y : u8) {
        self.put_char_with_attr(c,x,y,DEFAULT_ATTR)
    }

    #[inline]
    pub unsafe fn put_char_with_attr(&self, c: u8, x: u8, y: u8, attrib: u8) {
        write(self.ram.offset(xy_to_offset(x,y) as isize), (c as u16) | ((attrib as u16) << 8))
    }

    #[allow(dead_code)]
    pub unsafe fn put_buf(&self, buf: &[[u8; UINT_DISPLAY_WIDTH]; UINT_DISPLAY_HEIGHT]) {
        for y in range(0, DISPLAY_HEIGHT) {
            for x in range(0, DISPLAY_WIDTH) {
                self.put_char(buf[y as usize][x as usize], x, y);
            }
        }
    }

    pub unsafe fn put_line(&self, l: &[u8; UINT_DISPLAY_WIDTH], off: u8) {
        assert!(off < DISPLAY_HEIGHT);
        for x in range(0, DISPLAY_WIDTH) {
            self.put_char(l[x as usize], x, off);
        }
    }

    pub unsafe fn put_lines<'a>(&self, lines: &[[u8; UINT_DISPLAY_WIDTH]], mut start: u8) {
        for l in lines.iter() {
            self.put_line(l, start);
            start += 1;
        }
    }

    #[allow(dead_code)]
    pub unsafe fn clear(&self) {
        for y in range(0, DISPLAY_HEIGHT) {
            for x in range(0, DISPLAY_WIDTH) {
                self.put_char(DEFAULT_CHAR, x, y);
            }
        }
    }
}

#[inline]
fn xy_to_offset(x: u8, y: u8) -> usize { (y as usize) * (DISPLAY_WIDTH as usize) + (x as usize) }

