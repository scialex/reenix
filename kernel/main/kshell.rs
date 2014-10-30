// TODO Copyright Header

use core::cell::*;
use base::errno;
use mm::page;
use mm::alloc;
use procs::kproc;
use alloc::boxed::*;
use libc::c_void;
use core::prelude::*;
use core::ptr::*;
use collections::string;
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
use collections::*;
use core::iter::*;
use base::from_str::*;
use procs::args::ProcArgs;

pub fn start(i: i32) {
    let tty = ProcArgs::new(bytedev::lookup_mut(DeviceId::create(2,i as u8)).unwrap()).unwrap();
    assert!(KProc::new(String::from_str("KSHELL proc"), tty_proc_run, 0, unsafe { tty.to_arg() }).is_ok());
}

extern "C" fn tty_proc_run(_:i32, t:*mut c_void) -> *mut c_void {
    let tty = unsafe { ProcArgs::from_arg(t) };
    let mut s = KShell::new(tty.unwrap());
    s.add_normal_functions();
    s.run();
    0 as *mut c_void
}


pub type ShellFunc = fn(io: &mut Device<u8>, argv: &[&str]) -> KResult<()>;
#[deriving(Clone)]
pub struct KFunction<'a> {
    name : &'a str,
    description : &'a str,
    func : ShellFunc,
}
macro_rules! KFunc(
    ($n:expr, $d:expr, $f:ident) => ( KFunction { name: $n, description: $d, func: $f } );
    ($n:expr, $f:ident) => (KFunc!($n, $n, $f));
    ($f:ident) => (KFunc!(stringify!($f),$f));
)

impl<'a> KFunction<'a> {
    pub fn create<'a>(name: &'a str, func: ShellFunc) -> KFunction<'a> {
        KFunction { name : name, description: name, func : func }
    }
    pub fn new<'a>(name: &'a str, description: &'a str, func: ShellFunc) -> KFunction<'a> {
        KFunction { name: name, description: description, func: func }
    }
    pub fn call<'b, I: Iterator<&'b str>>(&'a self, io: &'b mut Device<u8>, args: I) -> KResult<()> {
        let f = self.func;
        let mut v = Vec::new();
        v.extend(args);
        f(io, v.as_slice())
    }
}

pub struct KShell<'a> {
    tty: &'a mut Device<u8>,
    // TODO A list of KFunction's
    funcs: TreeMap<&'a str, KFunction<'a>>,
}

static NFUNCS : &'static [KFunction<'static>] = &[
    KFunc!("exit", "exits the current process with the given value", do_exit),
    KFunc!("echo", "echos its arguments to the output.", do_echo),
    KFunc!("kshell", "create a new kshell on given tty", do_newkshell),
    KFunc!("proctest", "test procs", do_proctest),
    KFunc!("write-blocks", "write to blocks", do_bdwrite),
    KFunc!("read-block", "read a block", do_bdread),
    KFunc!("ipl", "prints the ipl", do_ipl),
    KFunc!("mem-stats", "prints memory statistics", do_memstats),
];

impl<'a> KShell<'a> {
    pub fn new<'b>(dev: &'b mut Device<u8>) -> KShell<'b> {
        KShell { tty: dev, funcs: TreeMap::new() }
    }

    pub fn add_normal_functions(&mut self) {
        for f in NFUNCS.iter() {
            self.add_function(f.clone());
        }
    }

    pub fn add_function(&mut self, f: KFunction<'a>) -> bool {
        let n : &'a str = f.name;
        self.funcs.insert(n, f)
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
                None => {
                    assert!(writeln!(ByteWriter(self.tty), "Given command included illegal charecters. Ignoring.").is_ok());
                    continue;
                },
            };
            self.run_command(cmd);
        }
    }

    pub fn run_command(&mut self, cmd: &str) {
        macro_rules! early_return(
            ($v:expr, $($a:expr),*) => ({
                match $v {
                    Some(x) => x,
                    None => {
                        assert!(writeln!(ByteWriter(self.tty), $($a),*).is_ok());
                        return;
                    }
                }
            })
        )
        let f = cmd.split(' ').nth(0).unwrap_or(cmd);
        if f == "help" {
            assert!(writeln!(ByteWriter(self.tty), "Loaded commands are: ").is_ok());
            assert!(writeln!(ByteWriter(self.tty), "help - print this message").is_ok());
            assert!(writeln!(ByteWriter(self.tty), "repeat - Repeat a command the given number of times").is_ok());
            for i in self.funcs.values() {
                assert!(writeln!(ByteWriter(self.tty), "{} - {}", i.name, i.description).is_ok());
            }
        } else if f == "repeat" {
            let v = early_return!(cmd.split(' ').filter(|s| { s.len() != 0 }).nth(1), "Usage: repeat num cmd ...");
            let reps = early_return!(from_str::<uint>(v), "'{}' is not a valid number. Usage: repeat num cmd ...", v);
            let r = early_return!(cmd.split(' ').filter(|s| { s.len() != 0 }).nth(2), "Usage: repeat num cmd ...");
            if let Some(func) = self.funcs.find_with(|x| { r.cmp(x) }) {
                for _ in range(0, reps) {
                    match func.call(self.tty, cmd.split(' ').filter(|s| { s.len() != 0 }).skip(2)) {
                        Ok(_) => {},
                        Err(v) => {
                            assert!(writeln!(ByteWriter(self.tty), "execution of command '{}' returned errno {}", f, v).is_ok());
                        },
                    }
                }
            } else {
                assert!(writeln!(ByteWriter(self.tty), "execution of command '{}' returned errno {}", f, v).is_ok());
            }
        } else if f == "" {
            return;
        } else if let Some(func) = self.funcs.find_with(|x| { f.cmp(x) }) {
            match func.call(self.tty, cmd.split(' ').filter(|s| { s.len() != 0 })) {
                Ok(_) => {},
                Err(v) => {
                    assert!(writeln!(ByteWriter(self.tty), "execution of command '{}' returned errno {}", f, v).is_ok());
                },
            }
        } else {
            assert!(writeln!(ByteWriter(self.tty), "unable to find command '{}'", f).is_ok());
        }
    }
}

fn do_memstats(io: &mut Device<u8>, _: &[&str]) -> KResult<()> {
    assert!(writeln!(ByteWriter(io), "{}", alloc::get_stats()).is_ok());
    alloc::stats_print();
    Ok(())
}

fn do_ipl(io: &mut Device<u8>, _: &[&str]) -> KResult<()> {
    assert!(writeln!(ByteWriter(io), "ipl is {:x}", interrupt::get_ipl()).is_ok());
    Ok(())
}

fn do_echo(io: &mut Device<u8>, argv: &[&str]) -> KResult<()> {
    let mut first = true;
    for i in argv.iter().skip(1) {
        assert!(write!(ByteWriter(io), "{}{}", if !first { " " } else { "" }, i).is_ok());
        first = false;
    }
    assert!(writeln!(ByteWriter(io), "").is_ok());
    Ok(())
}

fn do_exit(io: &mut Device<u8>, argv: &[&str]) -> KResult<()> {
    let status : uint = if argv.len() < 2 {
        0
    } else if argv.len() > 2 {
        assert!(writeln!(ByteWriter(io), "usage: exit [number]").is_ok());
        return Ok(());
    } else {
        match from_str(argv[1]) {
            Some(v) => v,
            None => {
                assert!(writeln!(ByteWriter(io), "usage: exit [number]").is_ok());
                return Ok(());
            },
        }
    };
    assert!(writeln!(ByteWriter(io), "quiting with status {}", status).is_ok());
    // TODO We should exit with the given status
    (current_thread!()).exit(status as *mut c_void);
    kpanic!("Should never reach here");
}

fn do_proctest(io: &mut Device<u8>, _: &[&str]) -> KResult<()> {
    let (pass, total) = super::proctest::run();
    assert!(writeln!(ByteWriter(io), "passed {} of {} tests", pass, total).is_ok());
    if pass == total { Ok(()) } else { Err(errno::EAGAIN) }
}

fn do_newkshell(io: &mut Device<u8>, argv: &[&str]) -> KResult<()> {
    if argv.len() != 2 {
        assert!(writeln!(ByteWriter(io), "Usage: kshell [tty id]").is_ok());
        return Ok(());
    }
    let id : u8 = match from_str(argv[1]) {
        Some(v) => v,
        None => {
            assert!(writeln!(ByteWriter(io), "Usage: kshell [tty id]").is_ok());
            return Err(errno::EINVAL);
        },
    };
    let tty = try!(ProcArgs::new(match bytedev::lookup_mut(DeviceId::create(2, id)) {
        Some(v) => v,
        None => {
            assert!(writeln!(ByteWriter(io), "{} is not a valid tty!", id).is_ok());
            return Err(errno::ENOTTY);
        },
    }).or_else(|_| Err(errno::ENOMEM)));
    assert!(writeln!(ByteWriter(io), "Creating new shell on tty {}", id).is_ok());
    KProc::new(String::from_str("KSHELL proc"), tty_proc_run, 0, unsafe { tty.to_arg() }).and(Ok(())).or_else(|_| Err(errno::ENOMEM))
}

fn do_bdread(io: &mut Device<u8>, argv: &[&str]) -> KResult<()> {
    use core::str::is_utf8;
    if argv.len() != 2 {
        assert!(writeln!(ByteWriter(io), "Usage: read-block block_num").is_ok());
        return Ok(());
    }
    let blk = match from_str(argv[1]) {
        Some(v) => v,
        None => {
            assert!(writeln!(ByteWriter(io), "Illegal block number {}, Usage: read_block block_num", argv[1]).is_ok());
            return Ok(());
        }
    };
    let mut buf : Box<[[u8, ..page::SIZE], ..1]> = box [[0,..page::SIZE], ..1];
    let disk = blockdev::lookup_mut(DeviceId::create(1,0)).expect("should have disk 0");
    let res = disk.read_from(blk, &mut *buf).and(Ok(()));
    if res.is_err() {
        assert!(writeln!(ByteWriter(io), "failed to read block {}", blk).is_ok());
        return res;
    }
    let mut cnt = 0;
    for i in buf[0].iter() {
        if *i == 0 || !is_utf8(&[*i]) {
            break;
        } else {
            cnt += 1;
        }
    }
    let s = buf[0].slice_to(cnt);
    match from_utf8(s) {
        Some(v) => { assert!(writeln!(ByteWriter(io), "{}", v).is_ok()) },
        None => { assert!(writeln!(ByteWriter(io), "**read succeeded but contained unprintable chars**").is_ok()) },
    }
    Ok(())
}
fn do_bdwrite(io: &mut Device<u8>, argv: &[&str]) -> KResult<()> {
    if argv.len() < 4 {
        assert!(writeln!(ByteWriter(io), "Usage: write-blocks block_num reps text [...]").is_ok());
        return Ok(());
    }
    let start = match from_str(argv[1]) {
        Some(v) => v,
        None => {
            assert!(writeln!(ByteWriter(io), "Illegal block number {}, Usage: write-blocks block_num reps text [...]", argv[1]).is_ok());
            return Ok(());
        }
    };
    let blks : uint = match from_str(argv[2]) {
        Some(v) => v,
        None => {
            assert!(writeln!(ByteWriter(io), "Illegal reps number {}, Usage: write-blocks block_num reps text [...]", argv[2]).is_ok());
            return Ok(());
        },
    };
    if blks > 8 {
        assert!(writeln!(ByteWriter(io), "Will not write more than 8 blocks for performance reasons").is_ok());
        return Ok(());
    }
    let mut example : [u8, ..page::SIZE] = [0, ..page::SIZE];
    let mut cur = 0;
    'end: for i in argv.slice_from(3).iter() {
        for v in i.bytes() {
            if cur >= (example.len() - 1) {
                break 'end;
            }
            example[cur] = v;
            cur += 1;
        }
        example[cur] = '\n' as u8;
        cur += 1;
    }
    example[cur] = '\0' as u8;

    let mut buf : Vec<[u8, ..page::SIZE]> = try!(alloc!(try Vec::with_capacity(blks)).or_else(
            |_| {
                assert!(writeln!(ByteWriter(io), "Unable to allocate a large enough buffer for {} pages.", blks).is_ok());
                Err(errno::ENOMEM)
            }
        ));
    for _ in range(0, blks) {
        use core::slice::bytes::copy_memory;
        let mut out : [u8, ..page::SIZE] = [0, ..page::SIZE];
        copy_memory(out, example);
        buf.push(out);
    }
    let disk = blockdev::lookup_mut(DeviceId::create(1,0)).expect("should have disk 0");
    disk.write_to(start, buf.as_slice()).and(Ok(()))
}

/*
extern "C" fn block_dev_proc(_: i32, _:*mut c_void) -> *mut c_void {
    // TODO Use this.
    // Try write
    let disk = blockdev::lookup_mut(DeviceId::create(1,0)).expect("should have tty");
    let mut buf : Box<[[u8, ..page::SIZE], ..3]> = box [[0, ..page::SIZE], ..3];
    let res = disk.write_to(0, &*buf);
    dbg!(debug::TEST, "result is {}", res);
    let res = disk.read_from(0, &mut *buf);
    dbg!(debug::TEST, "result is {}", res);
    0 as *mut c_void
}

*/

