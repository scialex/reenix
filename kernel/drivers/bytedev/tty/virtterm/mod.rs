
//! The Reenix virtual terminal support.

use super::ScrollDirection;
use super::ScrollDirection::{UP, DOWN};
use bytedev::tty::Finalizer;
use core::prelude::*;
use bytedev::tty::TTYDriver;
use procs::interrupt;
use core::cmp;

mod screen;

pub fn init_stage1() {
    screen::init_stage1();
    real_init_stage1();
}
pub fn init_stage2() {
    screen::init_stage2();
    real_init_stage2();
}

fn real_init_stage1() {}
fn real_init_stage2() {
}

pub const HISTORY_LINES: uint = screen::UINT_DISPLAY_HEIGHT * 6;

pub struct VirtualTerminal {
    // TODO This should really just hold a reference to it's screen and not use a static function
    // to get it.
    buf       : [[u8; screen::UINT_DISPLAY_WIDTH]; HISTORY_LINES],
    cur_line  : uint,
    cur_char  : uint,
    view_line : uint,
    active    : bool
}

impl VirtualTerminal {
    pub fn create() -> VirtualTerminal {
        VirtualTerminal {
            buf       : [[screen::DEFAULT_CHAR; screen::UINT_DISPLAY_WIDTH]; HISTORY_LINES],
            cur_line  : 0,
            cur_char  : 0,
            view_line : 0,
            active    : false,
        }
    }
    /// Return true if we need to redraw everything.
    fn line_feed(&mut self) -> bool {
        let redraw = self.cursor_at_bottom();
        if redraw {
            self.view_line = (self.view_line + 1) % HISTORY_LINES;
        }
        self.cur_line = (self.cur_line + 1) % HISTORY_LINES;
        self.buf[(self.cur_line + screen::UINT_DISPLAY_HEIGHT) % HISTORY_LINES] = [screen::DEFAULT_CHAR; screen::UINT_DISPLAY_WIDTH];
        return redraw;
    }

    fn get_line_y(&self) -> Option<uint> {
        let yval = if self.cur_line < self.view_line {
            (self.cur_line + HISTORY_LINES) - self.view_line
        } else {
            self.cur_line - self.view_line
        };
        if yval >= screen::UINT_DISPLAY_HEIGHT { None } else { Some(yval) }
    }
    fn cursor_at_bottom(&self) -> bool { (self.view_line + (screen::UINT_DISPLAY_HEIGHT - 1)) % HISTORY_LINES == self.cur_line }
    fn cursor_x(&self) -> Option<u8> {
        if self.cur_char == screen::UINT_DISPLAY_WIDTH {
            None
        } else {
            Some(self.cur_char as u8)
        }
    }
}

impl TTYDriver for VirtualTerminal {
    fn scroll(&mut self, dir: ScrollDirection) {
        match dir {
            DOWN => {
                if self.view_line != self.cur_line {
                    self.view_line = (self.view_line + 1) % HISTORY_LINES;
                } else {
                    return;
                }
            },
            UP => {
                let next_view = ((self.view_line + HISTORY_LINES) - 1) % HISTORY_LINES;
                // We keep the DISPLAY_HEIGHT lines after the current line clear so it looks ok
                // when we are at the very bottom.
                if next_view == (self.cur_line + screen::UINT_DISPLAY_HEIGHT) % HISTORY_LINES {
                    return;
                } else {
                    self.view_line = next_view;
                }
            },
        }
        if self.active {
            self.redraw();
        }
    }
    fn provide_char(&mut self, chr: u8) {
        match chr as char {
            '\n' => {
                self.cur_char = 0;
                if self.active {
                    if self.line_feed() {
                        self.redraw();
                    } else if let Some(y) = self.get_line_y() {
                        if let Some(x) = self.cursor_x() {
                            unsafe { screen::get_screen().move_cursor(x, y as u8); }
                        }
                    }
                } else { self.line_feed(); }
                return;
            },
            '\x08' | '\x7f' => {
                if self.cur_char != 0 {
                    self.cur_char -= 1;
                }
                self.buf[self.cur_line][self.cur_char] = screen::DEFAULT_CHAR;
                if self.active {
                    self.redraw();
                }
                return;
            },
            '\r' => {
                self.cur_char = 0;
                if self.active { self.redraw(); }
                return;
            },
            '\t' => {
                self.echo("    ");
                return;
            },
            _ => {},
        }
        let need_redraw = if self.cur_char == screen::UINT_DISPLAY_WIDTH {
            self.cur_char = 0;
            self.line_feed()
        } else { false };
        bassert!(self.cur_line < HISTORY_LINES);
        self.buf[self.cur_line][self.cur_char] = chr;

        let x = self.cur_char;
        self.cur_char += 1;

        if self.active {
            let yval = self.get_line_y();
            if let Some(y) = yval {
                if need_redraw {
                    self.redraw();
                } else {
                    unsafe {
                        screen::get_screen().put_char(chr, x as u8, y as u8);
                        if let Some(cx) = self.cursor_x() {
                            screen::get_screen().move_cursor(cx, y as u8);
                        }
                    }
                }
            }
        }
    }

    /// Return a thunk that will unblock io when it goes out of scope.
    fn block_io(&self) -> Finalizer {
        let cipl = interrupt::get_ipl();
        interrupt::set_ipl(cmp::max(interrupt::KEYBOARD, cipl));
        fn reset(d: uint) { interrupt::set_ipl(d as u8) }
        Finalizer { data: cipl as uint, func: reset }
    }

    fn redraw(&self) {
        let first_end = cmp::min(HISTORY_LINES, self.view_line + screen::UINT_DISPLAY_HEIGHT);
        unsafe { screen::get_screen().put_lines(self.buf.slice(self.view_line, first_end), 0); }

        let second_end = (self.view_line + screen::UINT_DISPLAY_HEIGHT) % HISTORY_LINES;

        if second_end < self.view_line && second_end != 0 {
            bassert!((HISTORY_LINES - self.view_line) + second_end == screen::UINT_DISPLAY_HEIGHT);
            unsafe { screen::get_screen().put_lines(self.buf.slice(0, second_end), (first_end - self.view_line) as u8); }
        } else {
            bassert!(first_end - self.view_line == screen::UINT_DISPLAY_HEIGHT);
        }
        if let Some(y) = self.get_line_y() {
            if let Some(x) = self.cursor_x() {
                unsafe { screen::get_screen().move_cursor(x, y as u8); }
            }
        }
    }

    fn set_active(&mut self) {
        self.redraw();
        self.active = true;
    }

    fn set_inactive(&mut self) { self.active = false; }
}

