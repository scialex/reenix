// TODO Copyright Header

use core::prelude::*;
use core::fmt::*;

/// The colors we can have
pub mod color {
    pub static NORMAL   : &'static str = "\x1b[0m";
    pub static BLACK    : &'static str = "\x1b[30;47m";
    pub static RED      : &'static str = "\x1b[31;40m";
    pub static GREEN    : &'static str = "\x1b[32;40m";
    pub static YELLOW   : &'static str = "\x1b[33;40m";
    pub static BLUE     : &'static str = "\x1b[34;40m";
    pub static MAGENTA  : &'static str = "\x1b[35;40m";
    pub static CYAN     : &'static str = "\x1b[36;40m";
    pub static WHITE    : &'static str = "\x1b[37;40m";
    pub static BRED     : &'static str = "\x1b[1;31;40m";
    pub static BGREEN   : &'static str = "\x1b[1;32;40m";
    pub static BYELLOW  : &'static str = "\x1b[1;33;40m";
    pub static BBLUE    : &'static str = "\x1b[1;34;40m";
    pub static BMAGENTA : &'static str = "\x1b[1;35;40m";
    pub static BCYAN    : &'static str = "\x1b[1;36;40m";
    pub static BWHITE   : &'static str = "\x1b[1;37;40m";
}

macro_rules! dbg_modes (
    ($(($n:ident, $v:expr, $c:expr, $ex:expr, $cfg:ident)),+) => (
        bitmask_create!(flags DbgMode : u64 {
            $($n = (0x1 << $v)),+
        })
        pub const NONE : DbgMode = DbgMode(0);
        pub const ALL : DbgMode = DbgMode(-1);
        impl DbgMode {
            #[allow(dead_code)]
            pub fn get_color(&self) -> &'static str {
                $(if $n & *self != NONE { return $c; })+
                return color::NORMAL;
            }
            #[allow(dead_code)]
            pub fn get_description(&self) -> &'static str {
                $(if $n & *self != NONE { return $ex; })+
                return "Unknown debug mode";
            }

            pub fn get_default() -> DbgMode {
                let mut ret : DbgMode = ALL;
                $(if cfg!($cfg) { ret = ret - $n })+
                return ret;
            }
        }
    )
)

dbg_modes!(
    (CORE,        0,  color::GREEN,   "core boot code", NDEBUG_CORE),
    (MM,          1,  color::RED,     "memory management", NDEBUG_MM),
    (INIT,        2,  color::NORMAL,  "boot/init code", NDEBUG_INIT),
    (SCHED,       3,  color::GREEN,   "swtch, scheduling", NDEBUG_SCHED),
    (DISK,        4,  color::YELLOW,  "disk driver", NDEBUG_DISK),
    (TEMP,        5,  color::NORMAL,  "for resolving temporary problems", NDEBUG_TEMP),
    (KMALLOC,     6,  color::MAGENTA, "kmalloc, kmem_cache_alloc", NDEBUG_KMALLOC),
    (PAGEALLOC,   7,  color::WHITE,   "page_alloc, etc.", NDEBUG_PAGEALLOC),
    (INTR,        8,  color::BRED,    "misc. trap/interrupt", NDEBUG_INTR),
    (TERM,        9,  color::BMAGENTA,"the terminal device", NDEBUG_TERM),
    (FORK,        10, color::BYELLOW, "fork(2)", NDEBUG_FORK),
    (PROC,        11, color::BLUE,    "process stuff", NDEBUG_PROC),
    (VNREF,       12, color::CYAN,    "vnode reference counts", NDEBUG_VNREF),
    (PFRAME,      13, color::BMAGENTA,"pframe subsys", NDEBUG_PFRAME),
    (ERROR,       14, color::BWHITE,  "error conditions", NDEBUG_ERROR),
    (SYSCALL,     15, color::RED,     "system calls", NDEBUG_SYSCALL),
    (FREF,        16, color::MAGENTA, "file reference counts", NDEBUG_FREF),
    (PGTBL,       17, color::BBLUE,   "page table manipulation", NDEBUG_PGTBL),
    (BRK,         18, color::YELLOW,  "process break; user memory alloc", NDEBUG_BRK),
    (EXEC,        19, color::BRED,    "new process exec", NDEBUG_EXEC),
    (VFS,         20, color::WHITE,   "vfs", NDEBUG_VFS),
    (S5FS,        21, color::BRED,    "system V file system", NDEBUG_S5FS),
    (KB,          22, color::BLUE,    "keyboard", NDEBUG_KB),
    (THR,         23, color::CYAN,    "thread stuff", NDEBUG_THR),
    (PRINT,       24, color::NORMAL,  "printdbg.c", NDEBUG_PRINT),
    (OSYSCALL,    25, color::BMAGENTA,"other system calls", NDEBUG_OSYSCALL),
    (VM,          28, color::RED,     "VM", NDEBUG_VM),
    (TEST,        30, color::RED,     "for testing code", NDEBUG_TEST),
    (TESTPASS,    31, color::GREEN,   "for testing code", NDEBUG_TESTPASS),
    (TESTFAIL,    32, color::RED,     "for testing code", NDEBUG_TESTFAIL),
    (MEMDEV,      33, color::BBLUE,   "For memory devices ('null' and 'zero')", NDEBUG_MEMDEV),
    (ANON,        34, color::WHITE,   "anonymous vm objects", NDEBUG_ANON),
    (VMMAP,       35, color::BGREEN,  "vm area mappings", NDEBUG_VMMAP),
    (ELF,         37, color::BGREEN,  "elf loader", NDEBUG_ELF),
    (USER,        38, color::BYELLOW, "user land", NDEBUG_USER),

    // This one should always be last.
    (PANIC,       63, color::RED,     "PANIC!", NDEBUG_PANIC)
)

