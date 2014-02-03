#pragma once

#include "types.h"

#include "mm/page.h"

/*
 * These color definitions are from the ANSI specs.
 * Do a web search for ANSI color codes to find out
 * more funky shit like this
 */

#define _NORMAL_    "\x1b[0m"
#define _BLACK_     "\x1b[30;47m"
#define _RED_       "\x1b[31;40m"
#define _GREEN_     "\x1b[32;40m"
#define _YELLOW_    "\x1b[33;40m"
#define _BLUE_      "\x1b[34;40m"
#define _MAGENTA_   "\x1b[35;40m"
#define _CYAN_      "\x1b[36;40m"
#define _WHITE_     "\x1b[37;40m"

#define _BRED_      "\x1b[1;31;40m"
#define _BGREEN_    "\x1b[1;32;40m"
#define _BYELLOW_   "\x1b[1;33;40m"
#define _BBLUE_     "\x1b[1;34;40m"
#define _BMAGENTA_  "\x1b[1;35;40m"
#define _BCYAN_     "\x1b[1;36;40m"
#define _BWHITE_    "\x1b[1;37;40m"

#define DBG_MODE(x)     (1ULL << (x))

/* These defines list all of the possible debugging
 * types. They are flags, so make sure to use the
 * DBG_MODE macro to declare new values. */
#define DBG_ALL         (~0ULL)         /* umm, "verbose"               */
#define DBG_CORE        DBG_MODE(0)     /* core boot code               */
#define DBG_MM          DBG_MODE(1)     /* memory management            */
#define DBG_INIT        DBG_MODE(2)     /* boot/init code               */
#define DBG_SCHED       DBG_MODE(3)     /* swtch, scheduling            */
#define DBG_DISK        DBG_MODE(4)     /* disk driver                  */
#define DBG_TEMP        DBG_MODE(5)     /* for resolving temporary problems */
#define DBG_KMALLOC     DBG_MODE(6)     /* kmalloc, kmem_cache_alloc    */
#define DBG_PAGEALLOC   DBG_MODE(7)     /* page_alloc, etc.             */
#define DBG_INTR        DBG_MODE(8)     /* misc. trap/interrupt         */
#define DBG_TERM        DBG_MODE(9)     /* the terminal device          */
#define DBG_FORK        DBG_MODE(10)    /* fork(2)                      */
#define DBG_PROC        DBG_MODE(11)    /* process stuff                */
#define DBG_VNREF       DBG_MODE(12)    /* vnode reference counts       */
#define DBG_PFRAME      DBG_MODE(13)    /* pframe subsys                */
#define DBG_ERROR       DBG_MODE(14)    /* error conditions             */
#define DBG_SYSCALL     DBG_MODE(15)    /* system calls                 */
#define DBG_FREF        DBG_MODE(16)    /* file reference counts        */
#define DBG_PGTBL       DBG_MODE(17)    /* page table manipulation      */
#define DBG_BRK         DBG_MODE(18)    /* process break; user memory alloc */
#define DBG_EXEC        DBG_MODE(19)    /* new process exec             */
#define DBG_VFS         DBG_MODE(20)    /* vfs                          */
#define DBG_S5FS        DBG_MODE(21)    /* system V file system         */
#define DBG_KB          DBG_MODE(22)    /* keyboard                     */
#define DBG_THR         DBG_MODE(23)    /* thread stuff                 */
#define DBG_PRINT       DBG_MODE(24)    /* printdbg.c                   */
#define DBG_OSYSCALL    DBG_MODE(25)    /* other system calls           */
#define DBG_VM          DBG_MODE(28)    /* VM                           */
#define DBG_TEST        DBG_MODE(30)    /* for testing code             */
#define DBG_TESTPASS    DBG_MODE(31)    /* for testing code             */
#define DBG_TESTFAIL    DBG_MODE(32)    /* for testing code             */

#define DBG_MEMDEV      DBG_MODE(33)    /* For memory devices ("null" and "zero") */
#define DBG_ANON        DBG_MODE(34)    /* anonymous vm objects         */
#define DBG_VMMAP       DBG_MODE(35)    /* vm area mappings             */
#define DBG_ELF         DBG_MODE(37)    /* elf loader                   */
#define DBG_USER        DBG_MODE(38)    /* user land                    */
#define DBG_DEFAULT     DBG_ERROR       /* default modes, 0 for none    */

/* This defines the name that is used in the
 * environment variable to turn on the given
 * debugging type, along with the color of the debug type */
/* NOTE that there is an order to these objects - the color chosen for a
 * debug statement with multiple DBG specifiers will be the first matching
 * result in the table */
/* Note that rearranging the table will affect results, and may be beneficial
 * later */
#define DBG_TAB                                 \
        /* General */                           \
        {"error", DBG_ERROR, _BWHITE_ },        \
        {"temp", DBG_TEMP, _NORMAL_ },          \
        {"print", DBG_PRINT, _NORMAL_ },        \
        {"test", DBG_TEST, _RED_ },             \
        {"testpass", DBG_TESTPASS, _GREEN_ },   \
        {"testfail", DBG_TESTFAIL, _RED_ },     \
        /* Kern 1 */                            \
        {"proc", DBG_PROC, _BLUE_ },            \
        {"thr", DBG_THR, _CYAN_ },              \
        {"sched", DBG_SCHED, _GREEN_ },         \
        {"init", DBG_INIT, _NORMAL_ },          \
        /* Kern 2 */                            \
        {"term", DBG_TERM, _BMAGENTA_ },        \
        {"disk", DBG_DISK, _YELLOW_ },          \
        {"memdev", DBG_MEMDEV, _BBLUE_ },       \
        /* VFS */                               \
        {"vfs", DBG_VFS, _WHITE_ },             \
        {"fref", DBG_FREF, _MAGENTA_ },         \
        {"vnref", DBG_VNREF, _CYAN_ },          \
        /* S5FS */                              \
        {"s5fs", DBG_S5FS, _BRED_ },            \
        {"pframe", DBG_PFRAME, _BMAGENTA_ },    \
        /* VM */                                \
        {"anon", DBG_ANON, _WHITE_ },           \
        {"vmmap", DBG_VMMAP, _BGREEN_ },        \
        {"fork", DBG_FORK, _BYELLOW_ },         \
        {"brk", DBG_BRK , _YELLOW_ },           \
        {"exec", DBG_EXEC, _BRED_ },            \
        {"elf", DBG_ELF, _BGREEN_ },            \
        {"pgtbl", DBG_PGTBL, _BBLUE_ },         \
        {"osyscall", DBG_OSYSCALL, _BMAGENTA_ }, \
        {"vm", DBG_VM, _RED_ },                 \
        /* Syscalls (VFS - VM) */               \
        {"syscall", DBG_SYSCALL, _RED_ },       \
        /* support code */                      \
        {"intr", DBG_INTR, _BRED_ },            \
        {"kmalloc", DBG_KMALLOC, _MAGENTA_ },   \
        {"pagealloc", DBG_PAGEALLOC, _WHITE_ }, \
        {"kb", DBG_KB, _BLUE_ },                \
        {"core", DBG_CORE, _GREEN_ },           \
        {"mm", DBG_MM, _RED_ },                 \
        {"user", DBG_USER, _BYELLOW_},          \
        /* Note this MUST be last or the color code will break */ \
        /* Also note that the color specified here is effectively the "default" */ \
        {"all", DBG_ALL, _NORMAL_ },            \
        { NULL,         0, NULL }

extern uint64_t dbg_modes;

/* A common interface for functions which provide human-readable information about
 * some data structure. Functions implementing this interface should fill buf with
 * up to size characters to describe the data passed in as data, then return the
 * number of characters writen. If there is not enough space in buf to write all
 * information then only size characters will be writen and size will be returned.
 * The returned string will be null terminated regardless of its length. */
typedef size_t (*dbg_infofunc_t)(const void *data, char *buf, size_t size);
#define DBG_BUFFER_SIZE (PAGE_SIZE)

void dbg_init(void);
void dbg_print(char *fmt, ...) __attribute__((format(printf, 1, 2)));
void dbg_printinfo(dbg_infofunc_t func, const void *data);

const char *dbg_color(uint64_t d_mode);

#ifndef NDEBUG
#define dbg(mode, ...)                                          \
        do {                                                    \
                if (dbg_active(mode)) {                         \
                        dbg_print("%s", dbg_color(mode));       \
                        dbg_print("%s:%d %s(): ",__FILE__, __LINE__, __func__); \
                        dbg_print(__VA_ARGS__);                 \
                        dbg_print("%s", _NORMAL_);              \
                }                                               \
        } while(0)

#define dbgq(mode, ...)                                         \
        do {                                                    \
                if (dbg_active(mode)) {                         \
                        dbg_print("%s", dbg_color(mode));       \
                        dbg_print(__VA_ARGS__);                 \
                        dbg_print("%s", _NORMAL_);              \
                }                                               \
        } while(0)

#define dbginfo(mode,func,data)                                 \
        do {                                                    \
                if (dbg_active(mode)) {                         \
                        dbg_print("%s", dbg_color(mode));       \
                        dbg_printinfo(func, data);              \
                        dbg_print("%s", _NORMAL_);              \
                }                                               \
        } while(0)

#define dbg_active(mode) (dbg_modes & (mode))
void dbg_add_mode(const char *mode);
void dbg_add_modes(const char *modes);
#else
#define dbg(mode, arg)
#define dbg_active(mode) 0
#define dbg_add_mode(mode)
#define dbg_add_modes(modes)
#endif

void dbg_panic(const char *file, int line, const char *func, const char *fmt, ...) __attribute__((format(printf, 4, 5)));
#define panic(fmt, args...) dbg_panic(__FILE__, __LINE__, __func__, (fmt), ## args)

#ifndef NDEBUG
#define KASSERT(x) do { if (!(x)) panic("assertion failed: %s", #x); } while(0)
#else
#define KASSERT(x)
#endif
