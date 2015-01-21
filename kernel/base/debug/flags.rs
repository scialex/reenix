// TODO Copyright Header

//! The flags for dbg!.

macro_rules! dbg_modes {
    ($(($n:ident, $v:expr, $c:expr, $ex:expr)),+) => (
        bitmask_create!(
            #[doc="The different debugging modes"]
            flags DbgMode : u64 {
            #[doc = "no error"] default NONE,
            $(#[doc = $ex ] $n = $v),+
        });
        #[doc="All the errors at once"]
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
                $(if is_disabled!(DEBUG -> $n) { ret = ret - $n })+
                return ret;
            }
        }
    )
}

#[cfg(TEST_LOW_MEMORY)] pub const BACKUP_MM : DbgMode = MM;
#[cfg(not(TEST_LOW_MEMORY))] pub const BACKUP_MM : DbgMode = DANGER;

dbg_modes!{
    (CORE,        0,  color::GREEN,   "core boot code"),
    (MM,          1,  color::RED,     "memory management"),
    (INIT,        2,  color::NORMAL,  "boot/init code"),
    (SCHED,       3,  color::GREEN,   "swtch, scheduling"),
    (DISK,        4,  color::YELLOW,  "disk driver"),
    (TEMP,        5,  color::NORMAL,  "for resolving temporary problems"),
    (KMALLOC,     6,  color::MAGENTA, "kmalloc, kmem_cache_alloc"),
    (PAGEALLOC,   7,  color::WHITE,   "page_alloc, etc."),
    (INTR,        8,  color::BRED,    "misc. trap/interrupt"),
    (TERM,        9,  color::BMAGENTA,"the terminal device"),
    (FORK,        10, color::BYELLOW, "fork(2)"),
    (PROC,        11, color::BLUE,    "process stuff"),
    (VNREF,       12, color::CYAN,    "vnode reference counts"),
    (PFRAME,      13, color::BMAGENTA,"pframe subsys"),
    (ERROR,       14, color::BWHITE,  "error conditions"),
    (SYSCALL,     15, color::RED,     "system calls"),
    (FREF,        16, color::MAGENTA, "file reference counts"),
    (PGTBL,       17, color::BBLUE,   "page table manipulation"),
    (BRK,         18, color::YELLOW,  "process break; user memory alloc"),
    (EXEC,        19, color::BRED,    "new process exec"),
    (VFS,         20, color::WHITE,   "vfs"),
    (S5FS,        21, color::BRED,    "system V file system"),
    (KB,          22, color::BLUE,    "keyboard"),
    (THR,         23, color::CYAN,    "thread stuff"),
    (PRINT,       24, color::NORMAL,  "printdbg.c"),
    (OSYSCALL,    25, color::BMAGENTA,"other system calls"),
    (VM,          28, color::RED,     "VM"),
    (TEST,        30, color::RED,     "for testing code"),
    (TESTPASS,    31, color::GREEN,   "for testing code"),
    (TESTFAIL,    32, color::RED,     "for testing code"),
    (MEMDEV,      33, color::BBLUE,   "For memory devices ('null' and 'zero')"),
    (ANON,        34, color::WHITE,   "anonymous vm objects"),
    (VMMAP,       35, color::BGREEN,  "vm area mappings"),
    (ELF,         37, color::BGREEN,  "elf loader"),
    (USER,        38, color::BYELLOW, "user land"),
    (PCACHE,      39, color::BMAGENTA,"pinnable cache system"),
    (KSHELL,      40, color::BBLUE   ,"Kshell messages"),

    (DANGER,      62, color::RED,     "A likely very dangerous operation"),

    // This one should always be last.
    (PANIC,       63, color::RED,     "PANIC!")
}

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
