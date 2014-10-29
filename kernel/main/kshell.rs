// TODO Copyright Header

use procs::kproc;
use alloc::boxed::*;
use libc::c_void;
use core::prelude::*;
use core::ptr::*;
use collections::string;
use core::fmt::*;
use procs::kthread;
use core::mem::transmute_copy;
use core::intrinsics::transmute;
use procs::kproc::{ProcStatus, ProcId, KProc};
use procs::interrupt;
use procs::sync::*;
use alloc::rc::*;
use drivers::*;
use base::errno::KResult;
use core::str::from_utf8;
use core::fmt::*;
use drivers::bytedev::ByteWriter;

/*
pub type ShellFunc<'a> = fn(&'a [&str]) -> KResult<()>;
pub struct KFunction<'a> {
    name : &'a str,
    description : &'a str,
    func : ShellFunc<'a>,
}

impl<'a> KFunction<'a> {
    pub fn create<'a>(name: &'a str, func: ShellFunc<'a>) -> KFunction<'a> {
        KFunction { name : name, description: name, func : func }
    }
    pub fn new<'a>(name: &'a str, description: &'a str, func: ShellFunc) -> KFunction<'a> {
        KFunction { name: name, description: description, func: func }
    }
    pub fn call<'a>(&'a self, args: &'a [&str]) -> KResult<()> {
        let f = self.func;
        f(args)
    }
    // TODO impl eq ord etc, so we can put it in a list
}
*/
pub struct KShell<'a> {
    tty: &'a mut Device<u8>,
    // TODO A list of KFunction's
}

impl<'a> KShell<'a> {
    pub fn new<'a>(dev: &'a mut Device<u8>) -> KShell<'a> {
        KShell { tty: dev }
    }

    pub fn run(&mut self) {
        // TODO Do stuff.
        loop {
            let mut buf : [u8, ..256] = [0, ..256];
            assert!(write!(ByteWriter(self.tty), "ksh# ").is_ok());
            let req = self.tty.read_from(0, &mut buf);
            let cmd = match from_utf8(
                        match req {
                            Ok(v) => buf.slice_to(v - 1),
                            Err(e) => {
                                assert!(writeln!(ByteWriter(self.tty), "An error occured while reading command. Error was {}. Quiting.",
                                                 e).is_ok());
                                return;
                            }
                        }) {
                Some(v) => v,
                None => { assert!(writeln!(ByteWriter(self.tty), "Given command included illegal charecters. Ignoring.").is_ok()); continue; }
            };
            match cmd.split(' ').nth(0).unwrap_or(cmd) {
                "exit" => {
                    assert!(writeln!(ByteWriter(self.tty), "quiting").is_ok());
                    return;
                },
                "echo" => {
                    let mut first = true;
                    for i in cmd.split(' ').skip(1).filter(|&s| { s.len() != 0 }) {
                        assert!(write!(ByteWriter(self.tty), "{}{}", if !first { " " } else { "" }, i).is_ok());
                        first = false;
                    }
                    assert!(write!(ByteWriter(self.tty), "\n").is_ok());
                },
                _ => {
                    assert!(writeln!(ByteWriter(self.tty), "unknown command {}", cmd).is_ok());
                }
            }
        }
    }
}
