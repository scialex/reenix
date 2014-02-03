#undef DEBUG_SH

#ifdef DEBUG_SH
#define dbg(x) fprintf x
#else
#define dbg(x)
#endif

#include <unistd.h>
#include <ctype.h>
#include <stdlib.h>
#include <string.h>
#include <fcntl.h>
#include <sys/mman.h>
#include <errno.h>
#include <stdio.h>

#define ROOT            "/"

#define HOME            ROOT "."
#define TMP             ROOT "tmp"

#define ARGV_MAX        256
#define REDIR_MAX       10

typedef struct redirect {
        int             r_sfd;
        int             r_dfd;
} redirect_t;

typedef struct redirect_map {
        int             rm_nfds;
        redirect_t      rm_redir[REDIR_MAX];
} redirect_map_t;

typedef struct ioenv {
        int             io_map_fd[3];
#define io_map_file io_map_fd
} ioenv_t;

static char **my_envp;

static void parse(char *line);
static int execute(int argc, char *argv[], redirect_map_t *map);
static void add_redirect(redirect_map_t *map, int sfd, int dfd);

#define DECL_CMD(x) static int cmd_ ## x (int argc, char *argv[], ioenv_t *io)

DECL_CMD(env);
DECL_CMD(cd);
DECL_CMD(help);
DECL_CMD(exit);
DECL_CMD(mkdir);
DECL_CMD(rmdir);
DECL_CMD(clear);
DECL_CMD(ln);
DECL_CMD(rm);
DECL_CMD(mv);
DECL_CMD(cat);
DECL_CMD(echo);
DECL_CMD(cp);
DECL_CMD(sync);
DECL_CMD(check);
DECL_CMD(repeat);
DECL_CMD(parallel);

typedef struct {
        const char      *cmd_name;
        int (*cmd_func)(int argc, char *argv[], ioenv_t *io);
        const char      *cmd_helptext;
} cmd_t;

static cmd_t builtin_cmds[] = {
        { "?",        cmd_help,     "list shell commands" },
        { "cat",      cmd_cat,      "display file" },
        { "env",      cmd_env,      "display environment"},
        { "cd",       cmd_cd,       "change directory" },
        { "check",    cmd_check,    "test operating system" },
        { "clear",    cmd_clear,    "clear screen" },
        { "cp",       cmd_cp,       "copy file" },
        { "echo",     cmd_echo,     "print arguments" },
        { "exit",     cmd_exit,     "exit shell" },
        { "help",     cmd_help,     "list shell commands" },
        { "ln",       cmd_ln,       "link file" },
        { "mkdir",    cmd_mkdir,    "create a directory" },
        { "mv",       cmd_mv,       "move file" },
        { "quit",     cmd_exit,     "exit shell" },
        { "rm",       cmd_rm,       "remove file(s)" },
        { "rmdir",    cmd_rmdir,    "remove a directory" },
        { "sync",     cmd_sync,     "sync filesystems" },
        { "repeat",   cmd_repeat,   "repeat a command" },
        { "parallel", cmd_parallel, "run multiple commands in parallel" },
        { NULL,       NULL,         NULL }
};

#define builtin_stdin (&io->io_map_file[0])
#define builtin_stdout (&io->io_map_file[1])
#define builtin_stderr (&io->io_map_file[2])

#define is_std_stream(fd) ((fd) >= 0 && (fd) <= 2)

DECL_CMD(chk_sparse);
DECL_CMD(chk_unlink);
DECL_CMD(chk_wrnoent);
DECL_CMD(chk_sbrk);
DECL_CMD(chk_zero);
DECL_CMD(chk_null);
DECL_CMD(chk_priv);

DECL_CMD(chk_malloc);

static cmd_t check_cmds[] = {
        { "null",       cmd_chk_null,   "read and write /dev/null" },
        { "sbrk",       cmd_chk_sbrk,   "memory allocation - sbrk" },
        { "sparse",     cmd_chk_sparse, "sparse mmap writes" },
        { "unlink",     cmd_chk_unlink, "create and unlink a file" },
        { "wrnoent",    cmd_chk_wrnoent,        "write to an unlinked file" },
        { "zero",       cmd_chk_zero,   "read and map /dev/zero" },
        { "priv",       cmd_chk_priv,   "writes to MAP_PRIVATE mapping" },
        { "malloc",     cmd_chk_malloc, "memory allocation - malloc" },
        { NULL,         NULL,           NULL }
};

static int check_failed(const char *test, const char *cmd)
{
        fprintf(stderr, "%s: %s failed: %s\n", test, cmd, strerror(errno));
        /*fprintf(stderr, "%s: %s failed: errno %d\n", test, cmd, errno);*/
        return 1;
}

static int check_exists(const char *file)
{
        int             fd;

        fd = open(file, O_RDONLY, 0);
        if (fd >= 0) {
                close(fd);
                return 1;
        } else
                return 0;
}

DECL_CMD(chk_priv)
{
        const char      *test = argv[0];
        const char      *tmpfile = TMP "/priv";
        int             fd;
        void            *addr;
        const char      *str = "Hello there.\n";
        int             error;
        char            *tmpstr;
        size_t          len;
        uint32_t        ii;
        char            tmpbuf[256];

        if (check_exists(tmpfile)) {
                fprintf(stderr, "%s: file exists: %s\n", test, tmpfile);
                return 1;
        }

        /* Create a file with some data.
        */

        fd = open(tmpfile, O_CREAT | O_RDWR, 0);
        if (fd < 0)
                return check_failed(test, "open");

        if (write(fd, str, strlen(str)) != (ssize_t)strlen(str)) {
                error = check_failed(test, "write");
                goto err_close;
        }

        /* Map the file MAP_PRIVATE.
        */

        len = strlen(str);
        addr = mmap(0, len, PROT_READ | PROT_WRITE, MAP_PRIVATE,
                    fd, 0);
        if (addr == MAP_FAILED) {
                error = check_failed(test, "mmap");
                goto err_close;
        }

        tmpstr = (char *) addr;

        /* Verify that the string is initially in the mapping.
        */

        if (strncmp(tmpstr, str, strlen(str))) {
                fprintf(stderr, "%s: file doesn't have string\n", test);
                error = 1;
                goto err_unmap;
        }

        memset(addr, 0, strlen(str));

        /* Verify that the string has been overwritten in the mapping.
        */

        for (ii = 0; ii < len; ii++) {
                if (tmpstr[ii] != 0) {
                        fprintf(stderr, "%s: didn't write to mapping\n", test);
                        error = 1;
                        goto err_unmap;
                }
        }

        /* Verify that the file still contains the original string.
        */

        if (lseek(fd, 0, SEEK_SET) != 0) {
                error = check_failed(test, "lseek");
                goto err_unmap;
        }

        if ((ii = read(fd, tmpbuf, sizeof(tmpbuf))) != len) {
                fprintf(stderr, "%s: read returned %d, expecting %d.\n", test,
                        ii, len);
                error = 1;
                goto err_unmap;
        }

        if (strncmp(tmpbuf, str, strlen(str))) {
                fprintf(stderr, "%s: file was changed by MAP_PRIVATE?!\n",
                        test);
                error = 1;
                goto err_unmap;
        }

        error = 0;

err_unmap:
        if (munmap(addr, len) < 0)
                error = check_failed(test, "munmap");
err_close:
        if (close(fd) < 0)
                error = check_failed(test, "close");
        if (unlink(tmpfile) < 0)
                error = check_failed(test, "unlink");
        return error;
}

DECL_CMD(chk_null)
{
        const char      *test = argv[0];
        const char      *null = "/dev/null";
        int             fd;
        int             nbytes;
        char            buf[256];
        int             error;

        fd = open(null, O_RDWR, 0600);
        if (fd < 0)
                return check_failed(test, "open");

        memset(buf, 0xCC, sizeof(buf));

        /* Try writing to /dev/null.  Should return buffer size.
        */

        nbytes = write(fd, buf, sizeof(buf));
        if (nbytes != sizeof(buf)) {
                error = check_failed(test, "write");
                goto err_close;
        }

        /* Try reading from /dev/null.  Should return zero.
        */

        nbytes = read(fd, buf, sizeof(buf));
        if (nbytes != 0) {
                error = check_failed(test, "read");
                goto err_close;
        }

        error = 0;

err_close:
        if (close(fd) < 0)
                error = check_failed(test, "close");
        return error;
}

DECL_CMD(chk_zero)
{
        const char      *test = argv[0];
        void            *addr;
        int             fd;
        const char      *zero = "/dev/zero";
        char            buf[256];
        int             nbytes;
        int             error;
        uint32_t        ii;
        size_t          len;
        unsigned long   *lp;
        unsigned char   *cp;

        fd = open(zero, O_RDWR, 0600);
        if (fd < 0)
                return check_failed(test, "open");

        /* Set buffer to a non-zero value, then read from /dev/zero
         * and make sure that the buffer is cleared.
         */

        memset(buf, 0xCC, sizeof(buf));

        nbytes = read(fd, buf, sizeof(buf));
        if (nbytes != sizeof(buf)) {
                error = check_failed(test, "read");
                goto err_close;
        }

        for (ii = 0; ii < sizeof(buf); ii++) {
                if (buf[ii] != 0) {
                        error = check_failed(test, "verify read");
                        goto err_close;
                }
        }

        /* Map /dev/zero and make sure all pages are initially zero.
        */

        len = 8192 * 5;

        addr = mmap(0, len, PROT_READ | PROT_WRITE, MAP_PRIVATE, fd, 0);
        if (addr == MAP_FAILED) {
                error = check_failed(test, "mmap");
                goto err_close;
        }

        cp = (unsigned char *) addr;
        for (ii = 0; ii < len; ii++, cp++) {
                if (*cp != 0) {
                        error = check_failed(test, "verify mmap zeros");
                        goto err_unmap;
                }
        }

        /* ... make sure writes are allowed.
        */

        lp = (unsigned long *) addr;
        for (ii = 0; ii < (len / sizeof(*lp)); ii++, lp++)
                *lp = ii;

        lp = (unsigned long *) addr;
        for (ii = 0; ii < (len / sizeof(*lp)); ii++, lp++) {
                if (*lp != ii) {
                        error = check_failed(test, "verify map write");
                        goto err_unmap;
                }
        }

        error = 0;

err_unmap:
        if (munmap(addr, len) < 0)
                error = check_failed(test, "munmap");
err_close:
        if (close(fd) < 0)
                error = check_failed(test, "close");
        return error;
}

DECL_CMD(chk_malloc)
{
        const char      *test = argv[0];
        void            *addr;
        int             len;
        int             *tmp;
        uint32_t        ii;
        int             error;

        len = 8192 + 128;
        addr = malloc(len);
        if (!addr)
                return check_failed(test, "malloc");

        /* Try writing to the memory.
        */
        tmp = (int *) addr;
        for (ii = 0; ii < (len / sizeof(int)); ii++)
                *tmp++ = ii;

        /* Verify what we've written.
        */
        tmp = (int *) addr;
        for (ii = 0; ii < (len / sizeof(int)); ii++) {
                if (*tmp++ != (int)ii) {
                        fprintf(stderr, "%s: verify failed at 0x%lx\n",
                                test, (unsigned long) tmp);
                        error = 1;
                        goto out_free;
                }
        }

        error = 0;
out_free:
        free(addr);
        return error;
}

DECL_CMD(chk_sbrk)
{
        void            *oldbrk1, *oldbrk2;
        const void      *brk_failed = (void *) - 1;
        const char      *test = argv[0];
        int             len;
        int             *tmp;
        uint32_t        ii;

        /* A length which is not a page multiple, yet a multiple of 8.
        */
        len = 8192 * 5 + 128;

        /* Try allocating some memory.
        */
        oldbrk1 = sbrk(len);
        if (oldbrk1 == brk_failed)
                return check_failed(test, "sbrk alloc");

        /* Try writing to the memory.
        */
        tmp = (int *) oldbrk1;
        for (ii = 0; ii < (len / sizeof(int)); ii++)
                *tmp++ = ii;

        /* Try verifying what we wrote.
        */
        tmp = (int *) oldbrk1;
        for (ii = 0; ii < (len / sizeof(int)); ii++) {
                if (*tmp++ != (int)ii) {
                        fprintf(stderr, "%s: verify failed at 0x%lx\n",
                                test, (unsigned long) tmp);
                        return 1;
                }
        }

        /* Try freeing the memory.
        */
        oldbrk2 = sbrk(-len);
        if (oldbrk2 == brk_failed)
                return check_failed(test, "sbrk dealloc");

        /* oldbrk2 should be at least "len" greater than oldbrk1.
        */
        if ((unsigned long) oldbrk2 < ((unsigned long) oldbrk1 + len)) {
                fprintf(stderr, "%s: sbrk didn't return old brk??\n",
                        test);
                return 1;
        }

        return 0;
}

DECL_CMD(chk_wrnoent)
{
        int             fd;
        int             error;
        int             nfd;
        const char      *tmpfile = TMP "/chk_wrnoent";
        const char      *test = argv[0];
        const char      *teststr = "Hello World!";
        char            buf[256];

        /* Verify that file doesn't exist.
        */
        if (check_exists(tmpfile)) {
                fprintf(stderr, "%s: tmpfile exists\n", test);
                return 1;
        }

        /* Create file.
        */
        fd = open(tmpfile, O_CREAT | O_RDWR, 0600);
        if (fd < 0)
                return check_failed(test, "create");

        /* Unlink file.
        */
        error = unlink(tmpfile);
        if (error < 0) {
                error = check_failed(test, "unlink");
                goto out_close;
        }

        /* Verify that file is gone.
        */
        nfd = open(tmpfile, O_RDONLY, 0);
        if (nfd >= 0) {
                error = check_failed(test, "open nonexistent");
                goto out_close;
        }

        /* Try writing a string to the file and reading it back.
        */
        error = write(fd, teststr, strlen(teststr));
        if (error != (ssize_t)strlen(teststr)) {
                error = check_failed(test, "write teststr");
                goto out_close;
        }
        error = lseek(fd, 0, SEEK_SET);
        if (error != 0) {
                error = check_failed(test, "lseek begin");
                goto out_close;
        }
        error = read(fd, buf, strlen(teststr));
        if (error != (ssize_t)strlen(teststr)) {
                error = check_failed(test, "read teststr");
                goto out_close;
        }

        /* Verify string.
        */
        if (strncmp(buf, teststr, strlen(teststr))) {
                fprintf(stderr, "%s: verify string failed\n", test);
                error = 1;
                goto out_close;
        }

        error = 0;
out_close:
        if (close(fd) < 0)
                error = check_failed(test, "close");
        return error;
}

DECL_CMD(chk_unlink)
{
        int             fd;
        int             error;
        int             nfd;
        const char      *tmpfile = TMP "/chk_unlink";
        const char      *test = argv[0];

        /* Verify that file doesn't exist.
        */
        if (check_exists(tmpfile)) {
                fprintf(stderr, "%s: tmpfile exists\n", test);
                return 1;
        }

        /* Create file.
        */
        fd = open(tmpfile, O_CREAT | O_RDONLY, 0600);
        if (fd < 0)
                return check_failed(test, "create");

        /* Unlink file.
        */
        error = unlink(tmpfile);
        if (error < 0) {
                error = check_failed(test, "unlink");
                goto out_close;
        }

        /* Verify that file is gone.
        */
        nfd = open(tmpfile, O_RDONLY, 0);
        if (nfd >= 0) {
                error = check_failed(test, "open nonexistent");
                goto out_close;
        }

        error = 0;
out_close:
        if (close(fd) < 0)
                error = check_failed(test, "close");
        return error;
}

DECL_CMD(chk_sparse)
{
        int             fd;
        const char      *tmpfile = TMP "/chk_sparse";
        const char      *test = argv[0];
        int             error;
        int             seek;
        int             len = 5 * 8192;
        void            *map;
        const char      *teststr = "Hello there?";
        char            *map_ch;
        const char      *tmpstr;
        char            buf[256];
        int             ii;

        /* Verify that file doesn't exist.
        */
        if (check_exists(tmpfile)) {
                fprintf(stderr, "%s: tmpfile exists\n", test);
                return 1;
        }

        /* Create file.
        */
        fd = open(tmpfile, O_CREAT | O_RDWR, 0600);
        if (fd < 0)
                return check_failed(test, "create");

        /* Extend length, so that there's a really big set of zero
         * blocks.
         */
        seek = lseek(fd, len, SEEK_SET);
        if (seek != len) {
                error = check_failed(test, "lseek len");
                goto out_close;
        }
        error = write(fd, &fd, 1);
        if (error < 0) {
                error = check_failed(test, "write");
                goto out_close;
        }

        /* Verify that the first few bytes are zero.  This should be
         * true, since the file should be sparse.
         */
        seek = lseek(fd, 0, SEEK_SET);
        if (seek != 0) {
                error = check_failed(test, "lseek begin");
                goto out_close;
        }
        error = read(fd, buf, strlen(teststr));
        if (error != (ssize_t)strlen(teststr)) {
                error = check_failed(test, "read zeros");
                goto out_close;
        }
        for (ii = strlen(teststr), tmpstr = buf; ii; ii--) {
                if (*tmpstr++) {
                        fprintf(stderr, "%s: verify zeros failed\n",
                                test);
                        error = 1;
                        goto out_close;
                }
        }

        /* Map file.
        */
        map = mmap(NULL, len, PROT_READ | PROT_WRITE, MAP_SHARED, fd, 0);
        if (map == MAP_FAILED) {
                error = check_failed(test, "mmap");
                goto out_close;
        }

        /* Write to first page of the mapping, which should be a zero
         * block.
         */
        map_ch = (char *) map;
        tmpstr = teststr;
        while (*tmpstr)
                *map_ch++ = *tmpstr++;

        /* Unmap the file.
        */
        if (munmap(map, len) < 0)
                error = check_failed(test, "munmap");

        /* Read from the first page.
        */
        seek = lseek(fd, 0, SEEK_SET);
        if (seek != 0) {
                error = check_failed(test, "lseek begin");
                goto out_close;
        }
        error = read(fd, buf, strlen(teststr));
        if (error != (ssize_t)strlen(teststr)) {
                error = check_failed(test, "read teststr");
                goto out_close;
        }
        buf[strlen(teststr)] = 0;

        /* Verify the data.
        */
        if (strcmp(teststr, buf)) {
                fprintf(stderr, "%s: verify data failed\n", test);
                fprintf(stderr, "%s: teststr \"%s\", buf \"%s\"\n",
                        test, teststr, buf);
                error = 1;
                goto out_close;
        }

        error = 0;
out_close:
        if (close(fd) < 0)
                error = check_failed(test, "close");
        if (unlink(tmpfile) < 0)
                error = check_failed(test, "unlink");
        return error;
}

static int check_run_one(cmd_t *cmd, ioenv_t *io)
{
        int             retval;
        char            *argv[2];

        argv[0] = (char *) cmd->cmd_name;
        argv[1] = NULL;

        fprintf(stdout, "%10s ... ", cmd->cmd_name);
        fflush(NULL);
        retval = (*cmd->cmd_func)(1, argv, io);
        fprintf(stdout, "%s\n", retval ? "FAILED" : "SUCCESS");

        return retval;
}

DECL_CMD(check)
{
        int             argn;
        cmd_t           *cmd;
        int             errors;

        if (argc < 2) {
                fprintf(stderr, "usage: check <test> [...]\n\n");

                fprintf(stderr, "Where <test> is either \"all\" or one of:\n");
                for (cmd = check_cmds; cmd->cmd_name; cmd++) {
                        fprintf(stderr, "%20s - %s\n",
                                cmd->cmd_name, cmd->cmd_helptext);
                }
                fprintf(stderr, "\n");
                return 1;
        }

        errors = 0;

        fprintf(stdout, "Running tests:\n");

        if (argc == 2 && !strcmp(argv[1], "all")) {
                for (cmd = check_cmds; cmd->cmd_name; cmd++)
                        errors += check_run_one(cmd, io);
                return errors;
        }

        for (argn = 1; argn < argc; argn++) {
                const char      *test;

                test = argv[argn];

                for (cmd = check_cmds; cmd->cmd_name; cmd++)
                        if (!strcmp(cmd->cmd_name, test))
                                break;
                if (!cmd->cmd_name) {
                        fprintf(stderr, "Unknown test: %s\n", test);
                        errors++;
                        continue;
                }

                errors += check_run_one(cmd, io);
        }

        return errors;
}

DECL_CMD(env)
{
        int i = 0;
        if (my_envp) {
                while (my_envp[i]) {
                        printf("env: %s\n", my_envp[i++]);
                }
        }
        return 0;
}

DECL_CMD(sync)
{
        sync();
        return 0;
}

static int do_cp(ioenv_t *io, const char *cmd,
                 const char *in_file, int in_fd,
                 const char *out_file, int out_fd)
{
#define buffer_sz 4096

        static char             buffer[buffer_sz];
        int                     nbytes_in;
        int                     nbytes_out;

        if (is_std_stream(in_fd))
                in_fd = io->io_map_fd[in_fd];
        if (is_std_stream(out_fd))
                out_fd = io->io_map_fd[out_fd];

        while ((nbytes_in = read(in_fd, buffer, buffer_sz)) > 0) {
                if ((nbytes_out = write(out_fd, buffer, nbytes_in)) < 0) {
                        fprintf(stderr,
                                "%s: unable to write to %s: %s\n",
                                cmd, out_file, strerror(errno));
                        return 0;
                }
        }
        if (nbytes_in < 0) {
                fprintf(stderr,
                        "%s: unable to read from %s: %s\n",
                        cmd, in_file, strerror(errno));
                return 0;
        }
        return 1;

#undef buffer_sz
}

DECL_CMD(cp)
{
        const char              *src;
        const char              *dest;
        int                     src_fd;
        int                     dest_fd;
        int                     error;

        if (argc != 3) {
                fprintf(stderr, "usage: cp <src> <dest>\n");
                return 1;
        }

        src = argv[1];
        dest = argv[2];

        src_fd = open(src, O_RDONLY, 0);
        if (src_fd < 0) {
                fprintf(stderr, "cp: unable to open %s: %s\n",
                        src, strerror(errno));
                return 1;
        }

        dest_fd = open(dest, O_WRONLY | O_CREAT | O_TRUNC, 0666);
        if (dest_fd < 0) {
                fprintf(stderr, "cp: unable to open %s: %s\n",
                        dest, strerror(errno));
                return 1;
        }

        error = 0;
        if (!do_cp(io, "cp", src, src_fd, dest, dest_fd))
                error = 1;

        close(src_fd);
        close(dest_fd);
        return error;
}

DECL_CMD(echo)
{
        int                     argn;

        for (argn = 2; argn < argc; argn++)
                fprintf(stdout, "%s ", argv[argn - 1]);
        fprintf(stdout, "%s\n", argv[argn - 1]);
        return 0;
}

DECL_CMD(cat)
{
        const char              *file;
        int                     argn;
        int                     fd;
        int                     error;

        if (argc == 1) {
                return !do_cp(io, "cat", "<stdin>", 0, "<stdout>", 1);
        }

        error = 0;

        for (argn = 1; argn < argc; argn++) {
                file = argv[argn];
                fd = open(file, O_RDONLY, 0);
                if (fd < 0) {
                        fprintf(stderr,
                                "cat: unable to open %s: %s\n",
                                file, strerror(errno));
                        error = 1;
                        continue;
                }
                if (!do_cp(io, "cat", file, fd, "<stdout>", 1))
                        error = 1;
                close(fd);
        }

        return error;
}

DECL_CMD(mv)
{
        const char              *src;
        const char              *dest;

        if (argc != 3) {
                fprintf(stderr, "usage: mv <src> <dest>\n");
                return 1;
        }

        src = argv[1];
        dest = argv[2];

        if (rename(src, dest) < 0) {
                fprintf(stderr,
                        "mv: unable to move %s to %s: %s\n",
                        src, dest, strerror(errno));
                return 1;
        }
        return 0;
}

DECL_CMD(rm)
{
        int                     argn;
        const char              *file;
        int                     error = 0;

        if (argc == 1) {
                fprintf(stderr, "usage: rm <file> [...]\n");
                return 1;
        }

        for (argn = 1; argn < argc; argn++) {
                file = argv[argn];
                if (unlink(file) < 0) {
                        fprintf(stderr,
                                "rm: unable to remove %s: %s\n",
                                file, strerror(errno));
                        error = 1;
                }
        }
        return error;
}

DECL_CMD(ln)
{
        const char              *src;
        const char              *dest;

        if (argc != 3) {
                fprintf(stderr, "usage: ln <src> <dest>\n");
                return 1;
        }

        src = argv[1];
        dest = argv[2];

        if (link(src, dest) < 0) {
                fprintf(stderr,
                        "ln: couldn't link %s to %s: %s\n",
                        dest, src, strerror(errno));
                return 1;
        }
        return 0;
}

DECL_CMD(mkdir)
{
        const char              *dir;

        if (argc != 2) {
                fprintf(stderr, "usage: mkdir <directory>\n");
                return 1;
        }

        dir = argv[1];
        if (mkdir(dir, 0777) < 0) {
                fprintf(stderr,
                        "mkdir: couldn't create %s: %s\n",
                        dir, strerror(errno));
                return 1;
        }
        return 0;
}

DECL_CMD(clear)
{
#define ESC "\x1B"
        fprintf(stdout, ESC "[H" ESC "[J");
#undef ESC
        return 0;
}

DECL_CMD(rmdir)
{
        const char              *dir;

        if (argc != 2) {
                fprintf(stderr, "usage: rmdir <directory>\n");
                return 1;
        }

        dir = argv[1];
        if (rmdir(dir) < 0) {
                fprintf(stderr,
                        "rmdir: couldn't remove %s: %s\n",
                        dir, strerror(errno));
                return 1;
        }
        return 0;
}

DECL_CMD(exit)
{
        exit(0);
        return 1;
}

DECL_CMD(help)
{
        cmd_t           *cmd;

        fprintf(stdout, "Shell commands:\n");

        for (cmd = builtin_cmds; cmd->cmd_name; cmd++)
                fprintf(stdout, "%20s - %s\n",
                        cmd->cmd_name, cmd->cmd_helptext);

        return 0;
}

DECL_CMD(cd)
{
        const char      *dir;

        if (argc > 2) {
                fprintf(stderr, "usage: cd <dir>\n");
                return 1;
        }

        if (argc == 1)
                dir = HOME;
        else
                dir = argv[1];

        if (chdir(dir) < 0) {
                fprintf(stderr, "sh: couldn't cd to %s: %s\n",
                        dir, strerror(errno));
                return 1;
        }
        return 0;
}

DECL_CMD(repeat)
{
        long            ntimes;

        if (argc < 3) {
                fprintf(stderr, "usage: repeat <ntimes> command [args ...]\n");
                return 1;
        }

        ntimes = strtol(argv[1], NULL, 10);
        if (ntimes <= 0) {
                fprintf(stderr, "repeat: <ntimes> must be non-zero\n");
                return 1;
        }

        while (ntimes--) {
                redirect_map_t          map;
                int                     ii;

                map.rm_nfds = 0;

                for (ii = 0; ii < 3; ii++) {
                        int             fd;

                        fd = dup(io->io_map_fd[ii]);
                        if (fd < 0) {
                                fprintf(stderr, "repeat: dup(%d) failed: "
                                        "%s\n",
                                        io->io_map_fd[ii], strerror(errno));
                                return 1;
                        }

                        add_redirect(&map, fd, ii);
                }

                execute(argc - 2, &argv[2], &map);
        }

        return 0;
}

DECL_CMD(parallel)
{
        int i, cmdbegin, ncmds = 0;
        char **cmd_argvs[32];
        int cmd_argcs[32];
        int cmd_pids[32];

        if (argc < 2) {
                fprintf(stderr, "usage: parallel <cmd1> [args] -- <cmd2> [args] [-- ...]\n");
                return 1;
        }

        /* Parse commands, dividing up argv */
        for (cmdbegin = (i = 1); i < argc; i++) {
                if (!strcmp(argv[i], "--")) {
                        /* Command delimiter - make sure command non-empty */
                        if (cmdbegin == i) {
                                fprintf(stderr, "empty command\n");
                                return 1;
                        }
                        argv[i] = NULL;

                        cmd_argcs[ncmds] = i - cmdbegin;
                        cmd_argvs[ncmds] = &argv[cmdbegin];
                        ncmds++;
                        if (ncmds > 32) {
                                fprintf(stderr, "too many commands\n");
                                return 1;
                        }
                        cmdbegin = i + 1;
                }
        }
        if (cmdbegin == argc) {
                fprintf(stderr, "empty command\n");
                return 1;
        }
        cmd_argcs[ncmds] = argc - cmdbegin;
        cmd_argvs[ncmds] = &argv[cmdbegin];
        ncmds++;
        if (ncmds > 32) {
                fprintf(stderr, "too many commands\n");
                return 1;
        }

        /* Fork and execute each command */
        for (i = 0; i < ncmds; i++) {
                if (0 == (cmd_pids[i] = fork())) {
                        int status, fd, ii;
                        /* Build weird map thing (as in repeat) */
                        redirect_map_t map;
                        map.rm_nfds = 0;
                        for (ii = 0; ii < 3; ii++) {
                                if (0 > (fd = dup(io->io_map_fd[ii])))
                                        exit(1);
                                add_redirect(&map, fd, ii);
                        }
                        /* Execute and return its status */
                        exit(execute(cmd_argcs[i], cmd_argvs[i], &map));
                }
        }
        /* Wait for each command */
        int status;
        for (i = 0; i < ncmds; i++) {
                wait(&status);
        }
        /* Return last status */
        return status;
}

static int do_redirect(redirect_map_t *map)
{
        int             ii;
        int             newfd, oldfd;

        for (ii = 0; ii < map->rm_nfds; ii++) {
                oldfd = map->rm_redir[ii].r_sfd;
                newfd = map->rm_redir[ii].r_dfd;

                dbg((stderr, "do_redirect: dup2(%d,%d)\n", oldfd, newfd));

                if (dup2(oldfd, newfd) < 0) {
                        fprintf(stderr, "do_redirect: dup2() failed: "
                                "%s\n", strerror(errno));
                        return -1;
                }
                close(oldfd);
        }
        return 0;
}

static void cleanup_redirects(redirect_map_t *map)
{
        int             ii;

        for (ii = 0; ii < map->rm_nfds; ii++)
                close(map->rm_redir[ii].r_sfd);
}

static void build_ioenv(redirect_map_t *map, ioenv_t *io)
{
        int             ii;

        /* Map stdin, stdout, stderr to themselves. */
        for (ii = 0; ii < 3; ii++)
                io->io_map_fd[ii] = ii;


        /* Execute redirection mappings. */
        for (ii = 0; ii < map->rm_nfds; ii++) {
                int sfd = map->rm_redir[ii].r_sfd;
                int dfd = map->rm_redir[ii].r_dfd;
                if (dfd >= 0 && dfd <= 2)
                        io->io_map_fd[dfd] = sfd;
        }
}

static void destroy_ioenv(ioenv_t *io)
{
}

static int builtin_exec(cmd_t *cmd, int argc, char *argv[], ioenv_t *io)
{
        return (*cmd->cmd_func)(argc, argv, io);
}

static int execute(int argc, char *argv[], redirect_map_t *map)
{
        int             status, pid;
        cmd_t           *cmd;

        for (cmd = builtin_cmds; cmd->cmd_name; cmd++) {
                if (!strcmp(cmd->cmd_name, argv[0]))
                        break;
        }
        if (cmd->cmd_name) {
                ioenv_t io;

                build_ioenv(map, &io);
                status = builtin_exec(cmd, argc, argv, &io);
                destroy_ioenv(&io);
                cleanup_redirects(map);
                return 0;
        }

        if (!(pid = fork())) {
                if (do_redirect(map) < 0)
                        exit(1);

                execve(argv[0], argv, my_envp);
                if (errno == ENOENT) {
                        char buf[256];
                        snprintf(buf, 255, "/usr/bin/%s", argv[0]);
                        execve(buf, argv, my_envp);
                        fprintf(stderr, "sh: command not found: %s\n", argv[0]);
                } else
                        fprintf(stderr, "sh: exec failed for %s: %s\n",
                                argv[0], strerror(errno));
                exit(1);
        } else {
                if (0 > pid) {
                        fprintf(stderr, "sh: fork failed errno = %d\n", errno);
                }
        }

        cleanup_redirects(map);
        int ret = wait(&status);
        if (status == EFAULT) {
                fprintf(stderr, "sh: child process accessed invalid memory\n");
        }

        return ret;
}

#define sh_isredirect(ch) ((ch) == '>' || (ch) == '<')

static int parse_redirect_dfd(char *line, char **start_p)
{
        char            *start;

        start = *start_p;

        if (start == line)
                return -1;

        /* Skip redirect symbol. */
        *start = 0;
        start--;

        /* Go backwards, skipping whitespace. */
        while ((start != line) && isspace(*start))
                start--;
        if (start == line)
                return -1;

        /* Go backwards, scanning digits. */
        while ((start != line) && isdigit(*start))
                start--;
        if (!(isspace(*start) || isdigit(*start)))
                return -1;

        /* This is the descriptor number. */
        *start_p = start;
        return strtol(start, NULL, 10);
}

static int redirect_default_fd(int type)
{
        if (type == '<')
                return 0;
        else if (type == '>')
                return 1;
        else {
                fprintf(stderr, "redirect_default_fd: Eh?\n");
                return -1;
        }
}

static void add_redirect(redirect_map_t *map, int sfd, int dfd)
{
        dbg((stderr, "add_redirect: %d -> %d\n", sfd, dfd));
        map->rm_redir[map->rm_nfds].r_sfd = sfd;
        map->rm_redir[map->rm_nfds].r_dfd = dfd;
        ++map->rm_nfds;
}

static int parse_redirect_dup(char *line, redirect_map_t *map, int dfd,
                              int mode, char *start, char **end_p)
{
        int     real_sfd, sfd;
        char    *sfdstr;

        /* Skip whitespace. */
        while (*start && isspace(*start))
                start++;
        if (!*start) {
                fprintf(stderr, "sh: bad redirect at end of line\n");
                return -1;
        }

        sfdstr = start;

        /* Scan digits. */
        if (!isdigit(*start)) {
                fprintf(stderr, "sh: parse error in dup redirect: 1\n");
                return -1;
        }
        while (*start && isdigit(*start))
                start++;
        if (*start && !isspace(*start)) {
                fprintf(stderr, "sh: parse error in dup redirect: 2\n");
                return -1;
        }

        /* Got a descriptor. */
        *start = 0;
        real_sfd = strtol(sfdstr, NULL, 10);

        dbg((stderr, "redirect_dup: %d -> %d\n", real_sfd, dfd));

        sfd = dup(real_sfd);
        if (sfd < 0) {
                fprintf(stderr, "sh: invalid file descriptor: %d\n", real_sfd);
                return -1;
        }

        add_redirect(map, sfd, dfd);

        *end_p = start + 1;
        return 0;
}

static int parse_redirect_norm(char *line, redirect_map_t *map, int dfd,
                               int mode, char *start, char **end_p)
{
        int     sfd;
        char    *path;

        /* Skip initial whitespace. */
        while (*start && isspace(*start))
                start++;
        if (!*start) {
                fprintf(stderr, "sh: bad redirect at end of line\n");
                return -1;
        }

        path = start;

        /* Scan pathname. */
        while (*start && !isspace(*start))
                start++;
        *start = 0;

        dbg((stderr, "redirect_norm: %s -> %d\n", path, dfd));

        sfd = open(path, mode, 0666);
        if (sfd < 0) {
                fprintf(stderr, "sh: unable to open %s: %s\n",
                        path, strerror(errno));
                return -1;
        }

        add_redirect(map, sfd, dfd);

        *end_p = start + 1;
        return 0;
}

static int parse_redirects(char *line, redirect_map_t *map)
{
        char    *tmp;

        map->rm_nfds = 0;

        tmp = line;
        for (;;) {
                char *start, *end;
                int type, dup, append;
                int mode;
                int dfd;

                dup = 0;
                append = 0;

                /* Find first redirect symbol. */
                while (*tmp && !sh_isredirect(*tmp))
                        tmp++;
                if (!*tmp)
                        break;

                start = tmp;
                type = *tmp;

                /* Parse the redirect.
                */

                /* Destination file descriptor.
                */
                dfd = parse_redirect_dfd(line, &start);
                if (dfd < 0)
                        dfd = redirect_default_fd(type);

                /* Look for append or dup.
                */
                tmp++;
                if (*tmp == '>') {
                        if (type != '>') {
                                fprintf(stderr, "sh: parse error at %c%c\n",
                                        type, *tmp);
                                return -1;
                        }
                        append = 1;
                        tmp++;
                }
                if (*tmp == '&') {
                        dup = 1;
                        tmp++;
                }

                /* Calculate open mode for file.
                */
                if (type == '<')
                        mode = O_RDONLY;
                else if (type == '>') {
                        mode = O_WRONLY | O_CREAT;
                        if (append)
                                mode |= O_APPEND;
                        else
                                mode |= O_TRUNC;
                } else {
                        fprintf(stderr, "sh: bad type in redirect: %c\n",
                                type);
                        return -1;
                }

                /* Parse the rest of the redirection.
                */
                if (dup) {
                        if (parse_redirect_dup(line, map, dfd, mode,
                                               tmp, &end) < 0)
                                return -1;
                } else {
                        if (parse_redirect_norm(line, map, dfd, mode,
                                                tmp, &end) < 0)
                                return -1;
                }

                /* Clear the redirect from the string.
                */
                while (start < end)
                        *start++ = ' ';

                tmp = end;
        }

        return 0;
}

static void parse(char *line)
{
        char            *argv[ARGV_MAX];
        int             argc;
        char            *tmp;
        int             len;
        redirect_map_t  map;

        argc = 0;
        tmp = line;

        len = strlen(line);
        if (line[len - 1] == '\n')
                line[len - 1] = 0;

        if (parse_redirects(line, &map) < 0)
                return;

        for (;;) {
                /* Ignore leading whitespace.
                */
                while (*tmp && isspace(*tmp))
                        tmp++;
                if (!*tmp)
                        break;

                argv[argc++] = tmp;

                /* Token is everything up to trailing whitespace.
                */
                while (*tmp && !isspace(*tmp))
                        tmp++;
                if (!*tmp)
                        break;

                /* Null-terminate token.
                */
                *tmp++ = 0;
        }

        argv[argc] = NULL;

        if (!argc)
                return;

        execute(argc, argv, &map);
}

static char linebuf[1024];

int main(int argc, char *argv[], char *envp[])
{
        int             nbytes;
        char            prompt [64];

        my_envp = envp;

        snprintf(prompt, 63, "weenix -> ");

        fprintf(stdout, "%s", prompt);
        fflush(NULL);
        while ((nbytes = read(0, linebuf, sizeof(linebuf))) > 0) {
                linebuf[nbytes] = 0;
                parse(linebuf);
                fprintf(stdout, "%s", prompt);
                fflush(NULL);
        }

        fprintf(stdout, "exit\n");

#ifdef __static__
        exit(0);
#endif
        return 0;
}
