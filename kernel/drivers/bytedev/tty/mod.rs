// TODO Copyright Header

//! The Reenix tty support module.

use base::cell::*;
use base::errno::KResult;
use RDeviceMut;
use WDeviceMut;

pub fn init_stage1() {
    keyboard::init_stage1();
    virtterm::init_stage1();
    ldisc::init_stage1();
    // TODO TTY INIT
}

pub fn init_stage2() {
    keyboard::init_stage2();
    virtterm::init_stage2();
    ldisc::init_stage2();
    // TODO TTY INIT
    keyboard::get_keyboard().set_handler(handle_keyboard_input);
    create_ttys();
    unsafe { TTYS[CUR_TTY_ID as usize].as_mut().expect("One of the ttys is null").set_active(); }
}

#[allow(unused_must_use)]
pub fn init_stage3() {
    get_current_tty().write_to(0, "REENIX STARTED TTY!\n".as_bytes());
}

#[allow(unused_must_use)]
pub fn shutdown() {
    get_current_tty().write_to(0, "\nREENIX IS SHUTTING DOWN.\nYou may now shut off your computer\n".as_bytes());
}

enum ScrollDirection { UP, DOWN, }

pub trait TTYLineDiscipline: RDeviceMut<u8> {
    /// Store that we recieved the given char and return a string to echo to the tty.
    fn recieve_char(&mut self, chr: u8) -> &'static str;
    /// Process a char that was written to the tty so it is suitable to be outputted to the tty.
    fn process_char(&self, chr: u8) -> &'static str;
}

struct Finalizer { data: usize, func: fn(usize), }
impl Drop for Finalizer {
    fn drop(&mut self) {
        let f = self.func;
        f(self.data);
    }
}

pub trait TTYDriver {
    /// Prints a char out to the device.
    fn provide_char(&mut self, chr: u8);
    /// Return a thunk that will unblock io when it goes out of scope.
    fn block_io(&self) -> Finalizer;

    fn echo(&mut self, s: &str) { for &i in s.as_bytes().iter() { self.provide_char(i); } }
    fn scroll(&mut self, dir: ScrollDirection);

    fn redraw(&self);

    fn set_active(&mut self);
    fn set_inactive(&mut self);
}

const TTY_MAJOR : u8 = 2;
const NUM_TTYS : u8 = 3;
static mut CUR_TTY_ID : u8 = 0;
static mut TTYS : [*mut TTY; (NUM_TTYS as usize)] = [0 as *mut TTY; (NUM_TTYS as usize)];
fn create_ttys() {
    for i in 0..NUM_TTYS {
        let t = box SafeCell::new(TTY::create(box virtterm::VirtualTerminal::create(), box ldisc::LineDiscipline::create()));
        unsafe { TTYS[i as usize] = (&*t.get_ref()) as *const TTY as *mut TTY; }
        super::register(::DeviceId::create(TTY_MAJOR, i), t);
    }
}

fn switch_tty(n: u8) {
    if n >= NUM_TTYS { return; }
    let old = get_current_tty();
    if unsafe { CUR_TTY_ID } != n {
        unsafe { CUR_TTY_ID = n; }
        old.set_inactive();
        get_current_tty().set_active();
    }
}

fn get_current_tty() -> &'static mut TTY {
    let n = unsafe { CUR_TTY_ID };
    unsafe { TTYS[n as usize].as_mut().expect("One of the ttys is null") }
}

struct TTY {
    driver     : Box<TTYDriver + 'static>,
    discipline : Box<TTYLineDiscipline + 'static>,
}

impl TTY {
    pub fn create(driver: Box<TTYDriver + 'static>, disc: Box<TTYLineDiscipline + 'static>) -> TTY {
        TTY { driver: driver, discipline : disc }
    }

    /// This function is called from the interrupt handler to take in the recieved char and echo it
    /// to the driver.
    fn handle_char(&mut self, chr: u8) {
        self.driver.echo(self.discipline.recieve_char(chr));
    }
    /// This function asks the driver to scroll.
    fn scroll(&mut self, dir: ScrollDirection) { self.driver.scroll(dir) }

    fn set_active(&mut self) { self.driver.set_active(); }
    fn set_inactive(&mut self) { self.driver.set_inactive(); }
}



impl RDeviceMut<u8> for TTY {
    #[allow(unused_variables)]
    fn read_from(&mut self, offset : usize, buf: &mut [u8]) -> KResult<usize> {
        let blocker = self.driver.block_io();
        let res = self.discipline.read_from(offset, buf);
        drop(blocker);
        res
    }
}

impl WDeviceMut<u8> for TTY {
    #[allow(unused_variables)]
    fn write_to(&mut self, _: usize, buf: &[u8]) -> KResult<usize> {
        let blocker = self.driver.block_io();
        for _ in buf.iter().map(|&i| { self.driver.echo(self.discipline.process_char(i)); }) {}
        drop(blocker);
        Ok(buf.len())
    }
}

extern "Rust" fn handle_keyboard_input(event: keyboard::Event) {
    let ct = get_current_tty();
    match event {
        keyboard::Event::Normal(chr) => ct.handle_char(chr),
        keyboard::Event::Switch(n) => switch_tty(n),
        keyboard::Event::ScrollUp => ct.scroll(ScrollDirection::UP),
        keyboard::Event::ScrollDown => ct.scroll(ScrollDirection::DOWN),
    }
}

mod ldisc;
mod virtterm;
mod keyboard;
