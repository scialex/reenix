#pragma once

/* Kernel and user header (via symlink) */

#ifdef __KERNEL__
#include "types.h"
#else
#include "sys/types.h"
#endif

/* Trap number for syscalls */
#define INTR_SYSCALL 0x2e

/* Keep all lists IN ORDER! */

#define SYS_syscall             0
#define SYS_exit                1
#define SYS_fork                2
#define SYS_read                3
#define SYS_write               4
#define SYS_open                5
#define SYS_close               6
#define SYS_waitpid             7
#define SYS_link                8
#define SYS_unlink              9
#define SYS_execve              10
#define SYS_chdir               11
#define SYS_sleep               12 /* NYI */
#define SYS_lseek               14
#define SYS_sync                15
#define SYS_nuke                16 /* NYI */
#define SYS_dup                 17
#define SYS_pipe                18
#define SYS_ioctl               19 /* NYI */
#define SYS_rmdir               21
#define SYS_mkdir               22
#define SYS_getdents            23
#define SYS_mmap                24
#define SYS_mprotect            25 /* NYI */
#define SYS_munmap              26
#define SYS_rename              27 /* NYI */
#define SYS_uname               28
#define SYS_thr_create          29 /* NYI */
#define SYS_thr_cancel          30
#define SYS_thr_exit            31
#define SYS_thr_yield           32
#define SYS_thr_join            33 /* NYI */
#define SYS_gettid              34 /* NYI */
#define SYS_getpid              35
#define SYS_errno               39
#define SYS_halt                40
#define SYS_get_free_mem        41 /* NYI */
#define SYS_set_errno           42
#define SYS_dup2                43
#define SYS_brk                 44
#define SYS_mount               45
#define SYS_umount              46
#define SYS_stat                47

/*
 * ... what does the scouter say about his syscall?
 * IT'S OVER 9000!
 * WHAT?! 9000?!
 */
#define SYS_debug               9001
#define SYS_kshell              9002

struct regs;
struct stat;

typedef struct argstr {
        const char *as_str;
        size_t      as_len; /* Not including null character */
} argstr_t;

typedef struct argvec {
        argstr_t   *av_vec;
        size_t      av_len; /* Not including null entry */
} argvec_t;

typedef struct waitpid_args {
        pid_t  wpa_pid;
        int    wpa_options;
        int   *wpa_status;
} waitpid_args_t;

typedef struct mmap_args {
        void   *mma_addr;
        size_t  mma_len;
        int     mma_prot;
        int     mma_flags;
        int     mma_fd;
        off_t   mma_off;
} mmap_args_t;

typedef struct munmap_args {
        void   *addr;
        size_t  len;
} munmap_args_t;

typedef struct open_args {
        argstr_t filename;
        int      flags;
        int      mode;
} open_args_t;

typedef struct read_args {
        int     fd;
        void   *buf;
        size_t  nbytes;
} read_args_t;

typedef struct write_args {
        int     fd;
        void   *buf;
        size_t  nbytes;
} write_args_t;

typedef struct mkdir_args {
        argstr_t path;
        int      mode;
} mkdir_args_t;

typedef struct link_args {
        argstr_t to;
        argstr_t from;
} link_args_t;

typedef struct execve_args {
        argstr_t filename;
        argvec_t argv;
        argvec_t envp;
} execve_args_t;

typedef struct rename_args {
        argstr_t oldname;
        argstr_t newname;
} rename_args_t;

typedef struct getdents_args {
        int            fd;
        struct dirent *dirp;
        size_t         count;
} getdents_args_t;

typedef struct lseek_args {
        int fd;
        int offset;
        int whence;
} lseek_args_t;

typedef struct dup2_args {
        int ofd;
        int nfd;
} dup2_args_t;

#ifdef __MOUNTING__
typedef struct mount_args {
        argstr_t spec;
        argstr_t dir;
        argstr_t fstype;
} mount_args_t;
#endif

typedef struct stat_args {
        argstr_t     path;
        struct stat *buf;
} stat_args_t;

struct utsname;
