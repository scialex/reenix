
//! The Reenix keyboard support.
#![allow(dead_code)]

use core::prelude::*;
use procs::interrupt;
use base::io::inb;

/* Indicates that one of these is "being held down" */
const SHIFT_MASK : u8 = 0x1;
const CTRL_MASK  : u8 = 0x2;
/* Indicates that an escape code was the previous key received */
const ESC_MASK   : u8 = 0x4;

/* Where to read from to get scancodes */
const KEYBOARD_IN_PORT : u16 = 0x60;
const KEYBOARD_CMD_PORT : u16 = 0x61;

/* Scancodes for special keys */
const LSHIFT  : u8 = 0x2a;
const RSHIFT  : u8 = 0x36;
const CTRL    : u8 = 0x1d;
/* Right ctrl is escaped */
/* Our keyboard driver totally ignores ALT */
const ESC0    : u8 = 0xe0;
const ESC1    : u8 = 0xe1;

/* Special stuff for scrolling (note that these only work when ctrl is held) */
const SCROLL_UP   : u8 = 0x0e;
const SCROLL_DOWN : u8 = 0x1c;

/* Keys for switching virtual terminals - for now F1,F2,...,F10 */
const VT_KEY_LOW  : u8 = 0x3b;
const VT_KEY_HIGH : u8 = 0x44;

/* If the scancode & BREAK_MASK, it's a break code; otherwise, it's a make code */
const BREAK_MASK : u8 = 0x80;

const NORMAL_KEY_HIGH : u8 = 0x39;

/* Some sneaky value to indicate we don't actually pass anything to the terminal */
const NO_CHAR : u8 = 0xff;

/* Scancode tables Based on
 *  http://www.win.tue.nl/~aeb/linux/kbd/scancodes-1.html
 */

/* The scancode table for "normal" scancodes - from 02 to 39 */
/* Unsupported chars are symbolized by \x00 */
const NORMAL_SCANCODES : &'static str = concat!("\x00",             /* Error */
                                                "\x1b",             /* Escape key */
                                                "1234567890-=",     /* Top row */
                                                "\x08",             /* Backspace */
                                                "\tqwertyuiop[]\n", /* Next row - ish */
                                                "\x00",             /* Left ctrl */
                                                "asdfghjkl;'`",
                                                "\x00",             /* Lshift */
                                                "\\",
                                                "zxcvbnm,./",
                                                "\x00\x00\x00",     /* Rshift, prtscrn, Lalt */
                                                " "                 /* Space bar */);

const SHIFT_SCANCODES  : &'static str = "\x00\x1b!@#$%^&*()_+\x08\tQWERTYUIOP{}\n\x00ASDFGHJKL:\"~\x00|ZXCVBNM<>?\x00\x00\x00 ";
// For some reason the parser does not like the much easier to read concat version.
                                      /*concat!("\x00",
                                                "\x1b",
                                                "!@#$%^&*()_+"
                                                "\x08",
                                                "\tQWERTYUIOP{}\n",
                                                "\x00",
                                                "ASDFGHJKL:\"~",
                                                "\x00",
                                                "\\",
                                                "ZXCVBNM<>?",
                                                "\x00\x00\x00",
                                                " ");*/

pub enum KeyboardEvent {
    Normal(u8), Switch(u8), ScrollUp, ScrollDown,
}

const IRQ_KEYBOARD : u16 = 0x1;

pub fn init_stage1() {}

pub fn init_stage2() {
    interrupt::map(IRQ_KEYBOARD, interrupt::KEYBOARD);
    interrupt::register(interrupt::KEYBOARD, keyboard_handler);
}

pub type KeyboardHandler = extern "Rust" fn(KeyboardEvent);

pub struct Keyboard { curmask: u8, handler: KeyboardHandler }

fn default_handler(_: KeyboardEvent) { }
static mut KEYBOARD : Keyboard = Keyboard { curmask : 0, handler: default_handler };

#[inline]
pub fn get_keyboard() -> &'static mut Keyboard { unsafe { &mut KEYBOARD } }

impl Keyboard {
    pub fn set_handler(&mut self, handler: KeyboardHandler) {
        self.handler = handler;
    }

    pub fn handle_interrupt(&mut self) {
        let inp = unsafe { inb(KEYBOARD_IN_PORT) };
        let sc = inp & !BREAK_MASK;
        let c = if inp & BREAK_MASK != 0 {
            /* Most break codes are ignored */
            /* Shift/ctrl release */
            if sc == LSHIFT || sc == RSHIFT {
                self.curmask &= !SHIFT_MASK;
            } else if sc == CTRL {
                self.curmask &= !CTRL_MASK;
            }
            return;
        } else if sc == LSHIFT || sc == RSHIFT {
            self.curmask |= SHIFT_MASK;
            return;
        } else if sc == CTRL {
            self.curmask |= CTRL_MASK;
            return;
        } else if self.curmask & ESC_MASK != 0 {
            /* Escape mask only lasts for one key */
            self.curmask &= !ESC_MASK;
            return;
        } else if sc == ESC0 || sc == ESC1 {
            self.curmask |= ESC_MASK;
            return;
        } else if sc >= VT_KEY_LOW && sc <= VT_KEY_HIGH {
            Switch(sc - VT_KEY_LOW)
        } else if (self.curmask & CTRL_MASK) != 0 && sc == SCROLL_DOWN {
            ScrollDown
        } else if (self.curmask & CTRL_MASK) != 0 && sc == SCROLL_UP {
            ScrollUp
        } else if sc > NORMAL_KEY_HIGH {
            return;
        } else if self.curmask & CTRL_MASK != 0 {
            /* Because of the way ASCII works, the control chars are based on the
             * values of the shifted chars produced without control */
            let c = SHIFT_SCANCODES.as_bytes()[sc as uint];
            /* Range of chars that have corresponding control chars */
            if c >= 0x40 && c < 0x60 {
                Normal(c - 0x40)
            } else {
                return;
            }
        } else if self.curmask & SHIFT_MASK != 0 {
            Normal(SHIFT_SCANCODES.as_bytes()[sc as uint])
        } else {
            Normal(NORMAL_SCANCODES.as_bytes()[sc as uint])
        };
        /* Give the key to the vt system, which passes it to the tty */
        let h = self.handler;
        h(c);
    }
}

extern "Rust" fn keyboard_handler(_: &mut interrupt::Registers) {
    get_keyboard().handle_interrupt();
}
