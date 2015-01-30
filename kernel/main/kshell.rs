// TODO Copyright Header

use base::errno::{self, KResult};
use drivers::*;
use drivers::bytedev::ByteWriter;
use libc::c_void;
use mm::{alloc, page};
use procs::args::ProcArgs;
use procs::interrupt;
use procs::kproc::{KProc, self};
use std::cell::*;
use std::collections::*;
use std::fmt::{self, Writer};
use std::str::{FromStr, from_utf8};

/// Just a wraper to writeln! or panic.
macro_rules! twriteln {
    ($t:expr, $f:expr, $($e:expr),*) => ({
        assert!(writeln!(&mut ByteWriter($t), $f, $($e),*).is_ok());
        dbg!(debug::KSHELL, concat!("writing: ", $f, "\\n"), $($e),*);
    });
    ($t:expr, $e:expr) => ({
        twriteln!($t, "{}", $e);
    });
}
/// Just a wraper to write! or panic.
macro_rules! twrite {
    ($t:expr, $f:expr, $($e:expr),*) => ({
        assert!(write!(&mut ByteWriter($t), $f, $($e),*).is_ok());
        dbg!(debug::KSHELL, concat!("writing: ", $f), $($e),*);
    });
    ($t:expr, $e:expr) => ({
        twrite!($t, "{}", $e);
    });
}

pub fn start(i: i32) {
    let tty = ProcArgs::new(bytedev::lookup(DeviceId::create(2,i as u8)).unwrap()).unwrap();
    assert!(KProc::new(String::from_str("KSHELL proc"), tty_proc_run, 0, unsafe { tty.to_arg() }).is_ok());
}

extern "C" fn tty_proc_run(_:i32, t:*mut c_void) -> *mut c_void {
    let tty = unsafe { ProcArgs::<&mut Device<u8>>::from_arg(t) };
    let mut s = KShell::new(tty.unwrap());
    s.add_normal_functions();
    s.run();
    current_thread!().retval
}


/// A shell function. It takes an io device and an argv.
pub type ExternShellFunc = fn(io: &mut Device<u8>, argv: &[&str]) -> KResult<()>;
/// An internal shell function. it can see and play with some elements of the kshell.
type InternShellFunc = for<'a> fn(sh: &KShell<'a>, argv: &[&str]) -> KResult<()>;

/// The different types of shell functions. Why can we not have the anonymous enum types.
enum ShellFunc {
    /// An external shell function.
    /// An external shell function.
    External(ExternShellFunc),
    /// An internal shell function.
    Internal(InternShellFunc),
}

/// A function that is for the kshell.
pub struct KFunction<'a> {
    name : &'a str,
    description : &'a str,
    func : ShellFunc,
}

/// A built-in function is implemented by the kshell.
macro_rules! KFunc_i {
    ($n:expr, $d:expr, $f:ident) => ( KFunction { name: $n, description: $d, func: ShellFunc::Internal($f) } );
    ($n:expr, $f:ident) => (KFunc!($n, $n, $f));
    ($f:ident) => (KFunc!(stringify!($f),$f));
}

/// A function for the kshell.
macro_rules! KFunc {
    ($n:expr, $d:expr, $f:ident) => ( KFunction { name: $n, description: $d, func: ShellFunc::External($f) } );
    ($n:expr, $f:ident) => (KFunc!($n, $n, $f));
    ($f:ident) => (KFunc!(stringify!($f),$f));
}

impl<'a> KFunction<'a> {
    #[allow(dead_code)]
    pub fn create<'b>(name: &'b str, func: ExternShellFunc) -> KFunction<'b> {
        KFunction { name : name, description: name, func : ShellFunc::External(func) }
    }
    #[allow(dead_code)]
    pub fn new<'b>(name: &'b str, description: &'b str, func: ExternShellFunc) -> KFunction<'b> {
        KFunction { name: name, description: description, func: ShellFunc::External(func) }
    }
    pub fn call<'b>(&'a self, ksh: &'a KShell<'b>, args: &[&str]) -> KResult<()> {
        match self.func {
            ShellFunc::External(f) => f(ksh.get_tty(), args),
            ShellFunc::Internal(f) => f(ksh, args),
        }
    }
}

/// A kshell. This is mostly just a list of functions.
pub struct KShell<'a> {
    tty: UnsafeCell<&'a mut Device<u8>>,
    funcs: BTreeMap<&'a str, &'a KFunction<'a>>,
}

/// All of the functions we have for our kshell. This will grow as time goes on.
static NFUNCS : &'static [KFunction<'static>] = &[
    KFunc_i!("help", "Show this help message", do_help),
    KFunc_i!("repeat", "repeat the given command", do_repeat),
    KFunc_i!("prepeat", "repeat the given command in parallel", do_prepeat),
    KFunc_i!("parallel", "run commands seperated by || in parallel", do_parallel),
    KFunc!("exit", "exits the current process with the given value. (Actually just cancel it to allow for memory cleanup)", do_exit),
    KFunc!("hard-exit", "exits the current process with the given value, This may leak memory or cause Undefined Behavior from the KSHELL", do_exit),
    KFunc!("echo", "echos its arguments to the output.", do_echo),
    KFunc!("kshell", "create a new kshell on given tty", do_newkshell),
    KFunc!("proctest", "test procs", do_proctest),
    KFunc!("write-blocks", "write to blocks", do_bdwrite),
    KFunc!("read-block", "read a block", do_bdread),
    KFunc!("ipl", "prints the ipl", do_ipl),
    KFunc!("mem-stats", "prints memory statistics", do_memstats),
    KFunc!("pid", "prints current pid", do_pid),
    KFunc!("cancel", "cancels a pid", do_cancel),
];

impl<'a> KShell<'a> {
    pub fn new<'b>(dev: &'b mut Device<u8>) -> KShell<'b> {
        KShell { tty: UnsafeCell::new(dev), funcs: BTreeMap::new() }
    }

    pub fn get_tty<'b>(&'b self) -> &'b mut Device<u8> {
        unsafe { *self.tty.get() }
    }

    pub fn add_normal_functions(&mut self) {
        self.funcs.extend(NFUNCS.iter().map(|&: v| (v.name, v)))
    }

    #[allow(unused)]
    pub fn add_function(&'a mut self, f: &'a KFunction<'a>) -> bool {
        let n : &'a str = f.name;
        self.funcs.insert(n, f).is_none()
    }

    pub fn run(&self) {
        loop {
            let mut buf : [u8; 256] = [0; 256];
            twrite!(self.get_tty(), "ksh# ");
            let req = self.get_tty().read_from(0, &mut buf);
            let cmd = match from_utf8(
                        match req {
                            Ok(v) => buf.slice_to(v - 1),
                            Err(e) => {
                                twriteln!(self.get_tty(), "An error occured while reading command. Error was {:?}. Quiting.", e);
                                return;
                            }
                        }) {
                Ok(v) => v,
                Err(_) => {
                    twriteln!(self.get_tty(), "Given command included illegal charecters. Ignoring.");
                    continue;
                },
            };
            match self.run_command(cmd.split(' ').filter(|&: s: & &str| -> bool { (*s).len() != 0 }).collect::<Vec<&str>>().as_slice()) {
                Ok(_) => {},
                Err(e) => { dbg!(debug::KSHELL, "recieved {:?}", e); },
            }

            if (current_thread!()).cancelled {
                return;
            }
        }
    }

    pub fn run_command(&self, argv: &[&str]) -> KResult<()> {
        if argv.len() == 0 {
            return Err(errno::ENOMSG);
        }
        let f = argv[0];
        if let Some(func) = self.funcs.get(f) {
            match func.call(self, argv) {
                Ok(_) => Ok(()),
                Err(v) => {
                    twriteln!(self.get_tty(), "execution of command '{:?}' returned errno {:?}", f, v);
                    Err(v)
                },
            }
        } else {
            twriteln!(self.get_tty(), "unable to find command '{:?}'", f);
            Err(errno::ENOMSG)
        }
    }

    pub fn print_help(&self) {
        twriteln!(self.get_tty(), "KSHELL COMMANDS");
        for v in self.funcs.values() {
            twriteln!(self.get_tty(), "{} - {}", v.name, v.description);
        }
    }
}

struct Showwer<'a>(&'a (fmt::Debug + 'static));
impl<'a> fmt::Debug for Showwer<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { self.0.fmt(f) }
}
fn do_memstats(io: &mut Device<u8>, _: &[&str]) -> KResult<()> {
    twriteln!(io, "{:?}", Showwer(alloc::get_stats()));
    alloc::stats_print();
    Ok(())
}

fn do_pid(io: &mut Device<u8>, _: &[&str]) -> KResult<()> {
    twriteln!(io, "pid is {:?}", current_pid!());
    Ok(())
}

fn do_ipl(io: &mut Device<u8>, _: &[&str]) -> KResult<()> {
    twriteln!(io, "ipl is {:x}", interrupt::get_ipl());
    Ok(())
}

fn do_echo(io: &mut Device<u8>, argv: &[&str]) -> KResult<()> {
    let mut first = true;
    for i in argv.iter().skip(1) {
        twrite!(io, "{}{}", if !first { " " } else { "" }, i);
        first = false;
    }
    twriteln!(io, "");
    Ok(())
}

fn do_exit(io: &mut Device<u8>, argv: &[&str]) -> KResult<()> {
    let status : usize = if argv.len() < 2 {
        0
    } else if argv.len() > 2 {
        twriteln!(io, "usage: exit [number]");
        return Ok(());
    } else {
        match FromStr::from_str(argv[1]) {
            Some(v) => v,
            None => {
                twriteln!(io, "usage: exit [number]");
                return Ok(());
            },
        }
    };
    twriteln!(io, "quiting with status {:?}", status);
    // TODO We should exit with the given status
    if argv[0] != "hard-exit" {
        (current_thread!()).cancel(status as *mut c_void);
    } else {
        twriteln!(io, "WARNING: This will likely leak memory (unless I somehow get CFI unwinding working)");
        (current_thread!()).exit(status as *mut c_void);
        kpanic!("Should never reach here");
    }
    Ok(())
}

fn do_proctest(io: &mut Device<u8>, _: &[&str]) -> KResult<()> {
    let (pass, total) = super::proctest::run();
    twriteln!(io, "passed {:?} of {:?} tests", pass, total);
    if pass == total { Ok(()) } else { Err(errno::EAGAIN) }
}

fn do_newkshell(io: &mut Device<u8>, argv: &[&str]) -> KResult<()> {
    if argv.len() != 2 {
        twriteln!(io, "Usage: kshell [tty id]");
        return Ok(());
    }
    let id : u8 = match FromStr::from_str(argv[1]) {
        Some(v) => v,
        None => {
            twriteln!(io, "Usage: kshell [tty id]");
            return Err(errno::EINVAL);
        },
    };
    let tty = try!(ProcArgs::new(match bytedev::lookup(DeviceId::create(2, id)) {
        Some(v) => v,
        None => {
            twriteln!(io, "{:?} is not a valid tty!", id);
            return Err(errno::ENOTTY);
        },
    }).or_else(|_| Err(errno::ENOMEM)));
    twriteln!(io, "Creating new shell on tty {:?}", id);
    KProc::new(String::from_str("KSHELL proc"), tty_proc_run, 0, unsafe { tty.to_arg() }).and(Ok(())).or_else(|_| Err(errno::ENOMEM))
}

/// Cancel a specific thread.
fn do_cancel(io: &mut Device<u8>, argv: &[&str]) -> KResult<()> {
    use procs::kproc::*;
    if argv.len() < 2 {
        twriteln!(io, "Usage: cancel pid [status]");
        return Ok(());
    }
    let pid_num = match FromStr::from_str(argv[1]) {
        Some(v) => v,
        None => {
            twriteln!(io, "Illegal pid number {:?}, Usage: cancel pid", argv[1]);
            return Ok(());
        }
    };
    let exit_status = match argv.get(2).map_or(Some(0), |v| FromStr::from_str(*v)) {
        Some(v) => v,
        None => {
            twriteln!(io, "illegal exit status {:?} given.", argv[2]);
            return Ok(());
        },
    };
    let pid = ProcId(pid_num);
    if pid == current_pid!() {
        (current_thread!()).cancel(exit_status as *mut c_void);
        twriteln!(io, "canceled process {:?} with status {:?}", pid, exit_status);
    } else {
        match KProc::get_proc(&pid) {
            Some(p) => {
                p.borrow_mut().kill(exit_status);
                twriteln!(io, "canceled process {:?} with status {:?}", pid, exit_status);
            },
            None => {
                twriteln!(io, "no process with {:?} found", pid);
            }
        }
    }
    return Ok(());
}

fn do_bdread(io: &mut Device<u8>, argv: &[&str]) -> KResult<()> {
    use std::str::from_utf8;
    if argv.len() != 2 {
        twriteln!(io, "Usage: read-block block_num");
        return Ok(());
    }
    let blk = match FromStr::from_str(argv[1]) {
        Some(v) => v,
        None => {
            twriteln!(io, "Illegal block number {:?}, Usage: read_block block_num", argv[1]);
            return Ok(());
        }
    };
    let mut buf : Box<[[u8; page::SIZE]; 1]> = box [[0; page::SIZE]; 1];
    let disk = blockdev::lookup(DeviceId::create(1,0)).expect("should have disk 0");
    let res = disk.read_from(blk, &mut *buf).and(Ok(()));
    if res.is_err() {
        twriteln!(io, "failed to read block {:?}", blk);
        return res;
    }
    let mut cnt = 0;
    for i in buf[0].iter() {
        if *i == 0 {
            break;
        } else {
            cnt += 1;
        }
    }
    let s = buf[0].slice_to(cnt);
    match from_utf8(s) {
        Ok(v) => { twriteln!(io, "{:?}", v); },
        Err(e) => { twriteln!(io, "**read succeeded but contained unprintable chars because {:?} **", e); }
    }
    Ok(())
}

fn do_bdwrite(io: &mut Device<u8>, argv: &[&str]) -> KResult<()> {
    if argv.len() < 4 {
        twriteln!(io, "Usage: write-blocks block_num reps text [...]");
        return Ok(());
    }
    let start = match FromStr::from_str(argv[1]) {
        Some(v) => v,
        None => {
            twriteln!(io, "Illegal block number {:?}, Usage: write-blocks block_num reps text [...]", argv[1]);
            return Ok(());
        }
    };
    let blks : usize = match FromStr::from_str(argv[2]) {
        Some(v) => v,
        None => {
            twriteln!(io, "Illegal reps number {:?}, Usage: write-blocks block_num reps text [...]", argv[2]);
            return Ok(());
        },
    };
    if blks > 8 {
        twriteln!(io, "Will not write more than 8 blocks for performance reasons");
        return Ok(());
    }
    let mut example : [u8; page::SIZE] = [0; page::SIZE];
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

    let mut buf : Vec<[u8; page::SIZE]> = try!(alloc!(try Vec::with_capacity(blks)).or_else(
            |_| {
                twriteln!(io, "Unable to allocate a large enough buffer for {:?} pages.", blks);
                Err(errno::ENOMEM)
            }
        ));
    for _ in range(0, blks) {
        use std::slice::bytes::copy_memory;
        let mut out : [u8; page::SIZE] = [0; page::SIZE];
        copy_memory(&mut out, &example);
        buf.push(out);
    }
    let disk = blockdev::lookup(DeviceId::create(1,0)).expect("should have disk 0");
    disk.write_to(start, buf.as_slice()).and(Ok(()))
}

fn do_help<'a>(sh: &KShell<'a>, _: &[&str]) -> KResult<()> {
    sh.print_help();
    Ok(())
}

#[derive(Clone)]
struct Instr { ksh: &'static KShell<'static>, line: &'static[&'static str] }
extern "C" fn parallel_run(_: i32, v:*mut c_void) -> *mut c_void {
    let i: Instr = unsafe { ProcArgs::from_arg(v).unwrap() };
    match i.ksh.run_command(i.line) {
        Ok(_) => 0 as *mut c_void,
        Err(e) => e as usize as *mut c_void,
    }
}

fn do_prepeat<'a>(sh: &KShell<'a>, argv: &[&str]) -> KResult<()> {
    if argv.len() < 3 {
        twriteln!(sh.get_tty(), "Usage: prepeat cnt cmd ..");
        return Ok(());
    }
    let reps = match FromStr::from_str(argv[1]) {
        Some(v) => v,
        None => {
            twriteln!(sh.get_tty(), "Usage: prepeat cnt cmd ..");
            return Ok(());
        },
    };
    let mut cmd = Vec::with_capacity((argv.len() - 1) * reps + 1);
    cmd.push("parallel");
    for i in range(0, reps) {
        if i != 0 {
            cmd.push("||");
        }
        cmd.push_all(argv.slice_from(2));
    }
    do_parallel(sh, cmd.as_slice())
}
#[allow(unused_must_use)]
fn do_parallel<'a>(sh: &KShell<'a>, argv: &[&str]) -> KResult<()> {
    use std::mem::transmute;
    if argv.len() < 2 {
        twriteln!(sh.get_tty(), "Usage: parallel cmd1 .. || cmd2 .. || cmd3 .. || ...");
        return Ok(());
    }
    let mut pids = Vec::new();
    let all_commands = argv.slice_from(1);
    for cmd in all_commands.split(|x| { *x == "||" }) {
        if cmd.len() == 0 {
            continue;
        } else if cmd[0] == "hard-exit" {
            twriteln!(sh.get_tty(), "Will not call hard-exit parallel, will cause memory leaks");
        }
        let args = Instr { ksh: unsafe { transmute(sh) }, line: unsafe { transmute(cmd) } };
        let pa = unsafe {
            match ProcArgs::new(args) {
                Ok(v) => v,
                Err(_) => { continue; },
            }.to_arg()
        };
        match KProc::new(String::from_str("KSHELL parallel proc"), parallel_run, 0, pa) {
            Ok(pid) => pids.push(pid),
            Err(v) => {
                twriteln!(sh.get_tty(), "Unable to create process for command {:?}, error was {:?}", cmd, v);
                drop(unsafe { ProcArgs::<Instr>::from_arg(pa).unwrap() });
            },
        }
    }
    // TODO Wait on everything.
    for pid in pids.iter() {
        let x = KProc::waitpid(kproc::Pid(*pid), 0);
        match x {
            Ok((_, _)) => {},
            Err(errno) => {
                twriteln!(sh.get_tty(), "Unable to wait for {:?}, error was {:?}", pid, errno);
            }
        }
    }
    Ok(())
}

#[allow(unused_must_use)]
fn do_repeat<'a>(sh: &KShell<'a>, argv: &[&str]) -> KResult<()> {
    if argv.len() < 3 {
        twriteln!(sh.get_tty(), "Usage: repeat num cmd ...");
        return Err(errno::EBADMSG);
    }
    match FromStr::from_str(argv[1]) {
        Some(c) => {
            for _ in range(0, c) {
                sh.run_command(argv.slice_from(2));
            }
            Ok(())
        },
        None => {
            twriteln!(sh.get_tty(), "{:?} is not a number, usage: repeat num cmd ...", argv[1]);
            Err(errno::EBADMSG)
        },
    }
}

/*
extern "C" fn block_dev_proc(_: i32, _:*mut c_void) -> *mut c_void {
    // TODO Use this.
    // Try write
    let disk = blockdev::lookup_mut(DeviceId::create(1,0)).expect("should have tty");
    let mut buf : Box<[[u8, ..page::SIZE], ..3]> = box [[0, ..page::SIZE], ..3];
    let res = disk.write_to(0, &*buf);
    dbg!(debug::TEST, "result is {:?}", res);
    let res = disk.read_from(0, &mut *buf);
    dbg!(debug::TEST, "result is {:?}", res);
    0 as *mut c_void
}

*/

