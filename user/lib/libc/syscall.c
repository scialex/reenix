#include "sys/types.h"
#include "stdarg.h"

#include "string.h"
#include "stdlib.h"

#include "unistd.h"
#include "weenix/trap.h"

#include "dirent.h"

static void *__curbrk = NULL;
#define MAX_EXIT_HANDLERS 32

static void     (*atexit_func[MAX_EXIT_HANDLERS])();
static int      atexit_handlers = 0;


void *sbrk(intptr_t incr)
{
        uintptr_t oldbrk;

        /* If we don't have a saved break, find it from the kernel */
        if (!__curbrk) {
                if (0 > (long)(__curbrk = (void *) trap(SYS_brk, (uint32_t) NULL))) {
                        return (void *) -1;
                }
        }

        oldbrk = (uintptr_t) __curbrk;

        /* Increment or decrement the saved break */

        if (incr < 0) {
                if ((uintptr_t) - incr > oldbrk) {
                        return (void *) -1;
                } else if (brk((void *)(oldbrk - (uintptr_t) - incr)) < 0) {
                        return (void *) -1;
                }
        } else if (incr > 0) {
                if (brk((void *)(oldbrk + (uintptr_t) incr)) < 0) {
                        return (void *) -1;
                }
        }
        return (void *) oldbrk;
}

int brk(void *addr)
{
        if (NULL == addr)
                return -1;
        void *newbrk = (void *) trap(SYS_brk, (uint32_t) addr);
        if (newbrk == (void *) -1)
                return -1;
        __curbrk = newbrk;
        return 0;
}

int fork(void)
{
        return trap(SYS_fork, 0);
}

int atexit(void (*func)(void))
{
        if (atexit_handlers < MAX_EXIT_HANDLERS) {
                atexit_func[atexit_handlers++] = func;
                return 0;
        }

        return 1;
}

void _exit(int status)
{
        trap(SYS_exit, (uint32_t) status);

        /* this keeps gcc from warning us about exit returning, because it
         * expects exit not to. We never actually get here. */
        for (;;);
}

void exit(int status)
{
        while (atexit_handlers--) {
                atexit_func[atexit_handlers]();
        }

        _exit(status);
        exit(status); /* gcc doesn't realize that _exit() exits */
}

void yield(void)
{
        /* Due to a Bochs bug, the yield syscall itself is highly unyielding
         * (for instance, it's impossible to type while a process is in a yield
         * loop. This is good enough. */
        (fork() ? wait(NULL) : exit(0));
}

pid_t wait(int *status)
{
        waitpid_args_t args;

        args.wpa_pid = -1;
        args.wpa_options = 0;
        args.wpa_status = status;

        return trap(SYS_waitpid, (uint32_t) &args);
}

pid_t waitpid(pid_t pid, int options, int *status)
{
        waitpid_args_t args;

        args.wpa_pid = pid;
        args.wpa_options = options;
        args.wpa_status = status;

        return trap(SYS_waitpid, (uint32_t) &args);
}

void thr_exit(int status)
{
        trap(SYS_thr_exit, (uint32_t) status);
}

pid_t getpid(void)
{
        return trap(SYS_getpid, 0);
}

int halt(void)
{
        return trap(SYS_halt, 0);
}

void *mmap(void *addr, size_t len, int prot, int flags, int fd, off_t off)
{
        mmap_args_t args;

        args.mma_addr = addr;
        args.mma_len = len;
        args.mma_prot = prot;
        args.mma_flags = flags;
        args.mma_fd = fd;
        args.mma_off = off;

        return (void *) trap(SYS_mmap, (uint32_t) &args);
}

int munmap(void *addr, size_t len)
{
        munmap_args_t args;

        args.addr = addr;
        args.len = len;

        return trap(SYS_munmap, (uint32_t) &args);
}

void sync(void)
{
        trap(SYS_sync, 0);
}

int open(const char *filename, int flags, int mode)
{
        open_args_t args;

        args.filename.as_len = strlen(filename);
        args.filename.as_str = filename;
        args.flags = flags;
        args.mode = mode;

        return trap(SYS_open, (uint32_t) &args);
}

off_t lseek(int fd, off_t offset, int whence)
{
        lseek_args_t args;

        args.fd = fd;
        args.offset = offset;
        args.whence = whence;

        return trap(SYS_lseek, (uint32_t) &args);
}


int read(int fd, void *buf, size_t nbytes)
{
        read_args_t args;

        args.fd = fd;
        args.buf = buf;
        args.nbytes = nbytes;


        return trap(SYS_read, (uint32_t) &args);
}

int write(int fd, const void *buf, size_t nbytes)
{
        write_args_t args;

        args.fd = fd;
        args.buf = (void *) buf;
        args.nbytes = nbytes;

        return trap(SYS_write, (uint32_t) &args);
}

int close(int fd)
{
        return trap(SYS_close, (uint32_t) fd);
}

int dup(int fd)
{
        return trap(SYS_dup, (uint32_t) fd);
}

int dup2(int ofd, int nfd)
{
        dup2_args_t args;

        args.ofd = ofd;
        args.nfd = nfd;

        return trap(SYS_dup2, (uint32_t) &args);
}

int mkdir(const char *path, int mode)
{
        mkdir_args_t args;

        args.path.as_len = strlen(path);
        args.path.as_str = path;
        args.mode = mode;

        return trap(SYS_mkdir, (uint32_t) &args);
}

int rmdir(const char *path)
{
        argstr_t args;
        args.as_len = strlen(path);
        args.as_str = path;
        return trap(SYS_rmdir, (uint32_t) &args);
}

int unlink(const char *path)
{
        argstr_t args;
        args.as_len = strlen(path);
        args.as_str = path;
        return trap(SYS_unlink, (uint32_t) &args);
}

int link(const char *from, const char *to)
{
        link_args_t args;

        args.from.as_len = strlen(from);
        args.from.as_str = from;
        args.to.as_len = strlen(to);
        args.to.as_str = to;

        return trap(SYS_link, (uint32_t) &args);
}

int rename(const char *oldname, const char *newname)
{
        rename_args_t args;

        args.oldname.as_len = strlen(oldname);
        args.oldname.as_str = oldname;
        args.newname.as_len = strlen(newname);
        args.newname.as_str = newname;

        return trap(SYS_rename, (uint32_t) &args);
}

int chdir(const char *path)
{
        argstr_t args;
        args.as_len = strlen(path);
        args.as_str = path;
        return trap(SYS_chdir, (uint32_t) &args);
}

size_t get_free_mem(void)
{
        return (size_t) trap(SYS_get_free_mem, 0);
}

int execve(const char *filename, char *const argv[], char *const envp[])
{
        execve_args_t           args;

        int i;

        args.filename.as_len = strlen(filename);
        args.filename.as_str = filename;

        /* Build argv vector */
        for (i = 0; argv[i] != NULL; i++)
                ;
        args.argv.av_len = i;
        args.argv.av_vec = malloc((args.argv.av_len + 1) * sizeof(argstr_t));
        for (i = 0; argv[i] != NULL; i++) {
                args.argv.av_vec[i].as_len = strlen(argv[i]);
                args.argv.av_vec[i].as_str = argv[i];
        }
        args.argv.av_vec[i].as_len = 0;
        args.argv.av_vec[i].as_str = NULL;

        /* Build envp vector */
        for (i = 0; envp[i] != NULL; i++)
                ;
        args.envp.av_len = i;
        args.envp.av_vec = malloc((args.envp.av_len + 1) * sizeof(argstr_t));
        for (i = 0; envp[i] != NULL; i++) {
                args.envp.av_vec[i].as_len = strlen(envp[i]);
                args.envp.av_vec[i].as_str = envp[i];
        }
        args.envp.av_vec[i].as_len = 0;
        args.envp.av_vec[i].as_str = NULL;

        /* Note that we don't need to worry about freeing since we are going to exec
         * (so all our memory will be cleaned up) */

        return trap(SYS_execve, (uint32_t) &args);
}

void thr_set_errno(int n)
{
        trap(SYS_set_errno, (uint32_t) n);
}

int thr_errno(void)
{
        return trap(SYS_errno, 0);
}

int getdents(int fd, dirent_t *dir, size_t size)
{
        getdents_args_t args;

        args.fd = fd;
        args.dirp = dir;
        args.count = size;

        return trap(SYS_getdents, (uint32_t) &args);
}

#ifdef __MOUNTING__
int
mount(const char *spec, const char *dir, const char *fstype)
{
        mount_args_t args;

        args.spec.as_len = strlen(spec);
        args.spec.as_str = spec;
        args.dir.as_len = strlen(dir);
        args.dir.as_str = dir;
        args.fstype.as_len = strlen(fstype);
        args.fstype.as_str = fstype;

        return trap(SYS_mount, (uint32_t) &args);
}

int
umount(const char *path)
{
        argstr_t argstr;

        argstr.as_len = strlen(path);
        argstr.as_str = path;

        return trap(SYS_umount, (uint32_t) &argstr);
}
#endif /* MOUNTING */

int
stat(const char *path, struct stat *buf)
{
        stat_args_t args;

        args.path.as_len = strlen(path);
        args.path.as_str = path;
        args.buf = buf;

        return trap(SYS_stat, (uint32_t) &args);
}

int
pipe(int pipefd[2])
{
        return trap(SYS_pipe, (uint32_t) pipefd);
}

int
uname(struct utsname *buf)
{
        return trap(SYS_uname, (uint32_t) buf);
}

int
debug(const char *str)
{
        argstr_t argstr;
        argstr.as_len = strlen(str);
        argstr.as_str = str;
        return trap(SYS_debug, (uint32_t) &argstr);
}
