#ifdef __KERNEL__

#include "kernel.h"
#include "globals.h"
#include "errno.h"
#include "config.h"
#include "limits.h"

#include "util/debug.h"
#include "util/string.h"
#include "util/printf.h"

#include "proc/proc.h"
#include "proc/kthread.h"

#include "fs/dirent.h"
#include "fs/vfs_syscall.h"
#include "fs/stat.h"
#include "fs/fcntl.h"
#include "fs/lseek.h"
#include "mm/mman.h"
#include "mm/kmalloc.h"

#include "test/usertest.h"
#include "test/vfstest/vfstest.h"

#undef __VM__

#else

#include <errno.h>
#include <string.h>
#include <stdlib.h>

#include <dirent.h>
#include <unistd.h>
#include <sys/stat.h>
#include <weenix/syscall.h>
#include <fcntl.h>
#include <sys/mman.h>
#include <stdio.h>

#include <test/test.h>

#endif

/* Some helpful strings */
#define LONGNAME "supercalifragilisticexpialidocious" /* Longer than NAME_LEN */

#define TESTSTR                                                                                 \
        "Lorem ipsum dolor sit amet, consectetur adipisicing elit, sed do "                     \
        "eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim "         \
        "veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo "      \
        "consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum "     \
        "dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, "     \
        "sunt in culpa qui officia deserunt mollit anim id est laborum."

#define SHORTSTR "Quidquid latine dictum, altum videtur"

static char root_dir[64];

static int
makedirs(const char *dir)
{
        char *d, *p;

        if (NULL == (d = malloc(strlen(dir) + 1))) {
                return ENOMEM;
        }
        strcpy(d, dir);

        p = d;
        while (NULL != (p = strchr(p + 1, '/'))) {
                *p = '\0';
                if (0 != mkdir(d, 0777) && EEXIST != errno) {
                        return errno;
                }
                *p = '/';
        }
        if (0 != mkdir(d, 0777) && EEXIST != errno) {
                return errno;
        }

        return 0;
}

static int
getdent(const char *dir, dirent_t *dirent)
{
        int ret, fd = -1;

        if (0 > (fd = open(dir, O_RDONLY, 0777))) {
                return -1;
        }

        ret = 1;
        while (ret != 0) {
                if (0 > (ret = getdents(fd, dirent, sizeof(*dirent)))) {
                        return -1;
                }
                if (0 != strcmp(".", dirent->d_name) && 0 != strcmp("..", dirent->d_name)) {
                        close(fd);
                        return 1;
                }
        }

        close(fd);
        return 0;
}

static int
removeall(const char *dir)
{
        int ret, fd = -1;
        dirent_t dirent;
        struct stat status;

        if (0 > chdir(dir)) {
                goto error;
        }

        ret = 1;
        while (ret != 0) {
                if (0 > (ret = getdent(".", &dirent))) {
                        goto error;
                }
                if (0 == ret) {
                        break;
                }

                if (0 > stat(dirent.d_name, &status)) {
                        goto error;
                }

                if (S_ISDIR(status.st_mode)) {
                        if (0 > removeall(dirent.d_name)) {
                                goto error;
                        }
                } else {
                        if (0 > unlink(dirent.d_name)) {
                                goto error;
                        }
                }
        }

        if (0 > chdir("..")) {
                return errno;
        }

        if (0 > rmdir(dir)) {
                return errno;
        }

        close(fd);
        return 0;

error:
        if (0 <= fd) {
                close(fd);
        }

        return errno;
}

static void
vfstest_start(void)
{
        int err;

        root_dir[0] = '\0';
        do {
                sprintf(root_dir, "vfstest-%d", rand());
                err = mkdir(root_dir, 0777);
        } while (err != 0);
        printf("Created test root directory: ./%s\n", root_dir);
}

/*
 * Terminates the testing environment
 */
static void
vfstest_term(void)
{
        if (0 != removeall(root_dir)) {
                fprintf(stderr, "ERROR: could not remove testing root %s: %s\n", root_dir, strerror(errno));
                exit(-1);
        }
        printf("Removed test root directory: ./%s\n", root_dir);
}

#define paths_equal(p1,p2)                                                                      \
        do {                                                                                    \
                int __r;                                                                        \
                struct stat __s1, __s2;                                                         \
                if (__r = makedirs(p1), !test_assert(0 == __r, "makedirs(\"%s\"): %s", p1, test_errstr(__r))) break; \
                if (__r = stat(p1, &__s1), !test_assert(0 == __r, "stat(\"%s\"): %s", p1, test_errstr(errno))) break; \
                if (__r = stat(p2, &__s2), !test_assert(0 == __r, "stat(\"%s\"): %s", p2, test_errstr(errno))) break; \
                test_assert(__s1.st_ino == __s2.st_ino, "paths_equals(\"%s\" (ino %d), \"%s\" (ino %d))", \
                            p1, __s1.st_ino, p2, __s2.st_ino);                                  \
        } while (0);

#define syscall_fail(expr, err)                                                                 \
        (test_assert((errno = 0, -1 == (expr)), "\nunexpected success, wanted %s (%d)", test_errstr(err), err) ? \
         test_assert((expr, errno == err), "\nexpected %s (%d)"                                 \
                     "\ngot      %s (%d)",                                                      \
                     test_errstr(err), err,                                                     \
                     test_errstr(errno), errno) : 0)
#define syscall_success(expr)                                                                   \
        test_assert(0 <= (expr), "\nunexpected error: %s (%d)",                                 \
                    test_errstr(errno), errno)

#define create_file(file)                                                                       \
        do {                                                                                    \
                int __fd;                                                                       \
                if (syscall_success(__fd = open((file), O_RDONLY|O_CREAT, 0777))) {             \
                        syscall_success(close(__fd));                                           \
                }                                                                               \
        } while (0);
#define read_fd(fd, size, goal)                                                                 \
        do {                                                                                    \
                char __buf[64];                                                                 \
                test_assert((ssize_t)strlen(goal) == read(fd, __buf, size), "\nread unexpected number of bytes"); \
                test_assert(0 == memcmp(__buf, goal, strlen(goal)), "\nread data incorrect");   \
        } while (0);
#define test_fpos(fd, exp)                                                                      \
        do {                                                                                    \
                int __g, __e = (exp);                                                           \
                syscall_success(__g = lseek(fd, 0, SEEK_CUR));                                  \
                test_assert((__g == __e), "fd %d fpos at %d, expected %d", fd, __g, __e);       \
        } while (0);

static void
vfstest_stat(void)
{
        int fd;
        struct stat s;

        syscall_success(mkdir("stat", 0));
        syscall_success(chdir("stat"));

        syscall_success(stat(".", &s));
        test_assert(S_ISDIR(s.st_mode), NULL);

        create_file("file");
        syscall_success(stat("file", &s));
        test_assert(S_ISREG(s.st_mode), NULL);

        /* file size is correct */
        syscall_success(fd = open("file", O_RDWR, 0));
        syscall_success(write(fd, "foobar", 6));
        syscall_success(stat("file", &s));
        test_assert(s.st_size == 6, "unexpected file size");
        syscall_success(close(fd));

        /* error cases */
#ifdef __VM__
        syscall_fail(stat(".", NULL), EFAULT);
#endif
        syscall_fail(stat("noent", &s), ENOENT);

        syscall_success(chdir(".."));
}

static void
vfstest_mkdir(void)
{
        syscall_success(mkdir("mkdir", 0777));
        syscall_success(chdir("mkdir"));

        /* mkdir an existing file or directory */
        create_file("file");
        syscall_fail(mkdir("file", 0777), EEXIST);
        syscall_success(mkdir("dir", 0777));
        syscall_fail(mkdir("dir", 0777), EEXIST);

        /* mkdir an invalid path */
        syscall_fail(mkdir(LONGNAME, 0777), ENAMETOOLONG);
        syscall_fail(mkdir("file/dir", 0777), ENOTDIR);
        syscall_fail(mkdir("noent/dir", 0777), ENOENT);
        syscall_fail(rmdir("file/dir"), ENOTDIR);
        syscall_fail(rmdir("noent/dir"), ENOENT);
        syscall_fail(rmdir("noent"), ENOENT);
        syscall_fail(rmdir("."), EINVAL);
        syscall_fail(rmdir(".."), ENOTEMPTY);
        syscall_fail(rmdir("dir/."), EINVAL);
        syscall_fail(rmdir("dir/.."), ENOTEMPTY);
        syscall_fail(rmdir("noent/."), ENOENT);
        syscall_fail(rmdir("noent/.."), ENOENT);

        /* unlink and rmdir the inappropriate types */
        syscall_fail(rmdir("file"), ENOTDIR);
        syscall_fail(unlink("dir"), EPERM);

        /* remove non-empty directory */
        create_file("dir/file");
        syscall_fail(rmdir("dir"), ENOTEMPTY);

        /* remove empty directory */
        syscall_success(unlink("dir/file"));
        syscall_success(rmdir("dir"));

        syscall_success(chdir(".."));
}

static void
vfstest_chdir(void)
{
#define CHDIR_TEST_DIR "chdir"

        struct stat ssrc, sdest, sparent, sdir;
        struct stat rsrc, rdir;

        /* chdir back and forth to CHDIR_TEST_DIR */
        syscall_success(mkdir(CHDIR_TEST_DIR, 0777));
        syscall_success(stat(".", &ssrc));
        syscall_success(stat(CHDIR_TEST_DIR, &sdir));

        test_assert(ssrc.st_ino != sdir.st_ino, NULL);

        syscall_success(chdir(CHDIR_TEST_DIR));
        syscall_success(stat(".", &sdest));
        syscall_success(stat("..", &sparent));

        test_assert(sdest.st_ino == sdir.st_ino, NULL);
        test_assert(ssrc.st_ino == sparent.st_ino, NULL);
        test_assert(ssrc.st_ino != sdest.st_ino, NULL);

        syscall_success(chdir(".."));
        syscall_success(stat(".", &rsrc));
        syscall_success(stat(CHDIR_TEST_DIR, &rdir));

        test_assert(rsrc.st_ino == ssrc.st_ino, NULL);
        test_assert(rdir.st_ino == sdir.st_ino, NULL);

        /* can't chdir into non-directory */
        syscall_success(chdir(CHDIR_TEST_DIR));
        create_file("file");
        syscall_fail(chdir("file"), ENOTDIR);
        syscall_fail(chdir("noent"), ENOENT);
        syscall_success(chdir(".."));
}

static void
vfstest_paths(void)
{
#define PATHS_TEST_DIR "paths"

        struct stat s;

        syscall_success(mkdir(PATHS_TEST_DIR, 0777));
        syscall_success(chdir(PATHS_TEST_DIR));

        syscall_fail(stat("", &s), EINVAL);

        paths_equal(".", ".");
        paths_equal("1/2/3", "1/2/3");
        paths_equal("4/5/6", "4/5/6");

        /* root directory */
        paths_equal("/", "/");
        paths_equal("/", "/..");
        paths_equal("/", "/../");
        paths_equal("/", "/../.");

        /* . and .. */
        paths_equal(".", "./.");
        paths_equal(".", "1/..");
        paths_equal(".", "1/../");
        paths_equal(".", "1/2/../..");
        paths_equal(".", "1/2/../..");
        paths_equal(".", "1/2/3/../../..");
        paths_equal(".", "1/../1/..");
        paths_equal(".", "1/../4/..");
        paths_equal(".", "1/../1/..");
        paths_equal(".", "1/2/3/../../../4/5/6/../../..");
        paths_equal(".", "1/./2/./3/./.././.././.././4/./5/./6/./.././.././..");

        /* extra slashes */
        paths_equal("1/2/3", "1/2/3/");
        paths_equal("1/2/3", "1//2/3");
        paths_equal("1/2/3", "1/2//3");
        paths_equal("1/2/3", "1//2//3");
        paths_equal("1/2/3", "1//2//3/");
        paths_equal("1/2/3", "1///2///3///");

        /* strange names */
        paths_equal("-", "-");
        paths_equal(" ", " ");
        paths_equal("\\", "\\");
        paths_equal("0", "0");

        struct stat st;

        /* error cases */
        syscall_fail(stat("asdf", &st), ENOENT);
        syscall_fail(stat("1/asdf", &st), ENOENT);
        syscall_fail(stat("1/../asdf", &st), ENOENT);
        syscall_fail(stat("1/2/asdf", &st), ENOENT);

        create_file("1/file");
        syscall_fail(open("1/file/other", O_RDONLY, 0777), ENOTDIR);
        syscall_fail(open("1/file/other", O_RDONLY | O_CREAT, 0777), ENOTDIR);

        syscall_success(chdir(".."));
}

static void
vfstest_fd(void)
{
#define FD_BUFSIZE 5
#define BAD_FD 20
#define HUGE_FD 9999

        int fd1, fd2;
        char buf[FD_BUFSIZE];
        struct dirent d;

        syscall_success(mkdir("fd", 0));
        syscall_success(chdir("fd"));

        /* read/write/close/getdents/dup nonexistent file descriptors */
        syscall_fail(read(BAD_FD, buf, FD_BUFSIZE), EBADF);
        syscall_fail(read(HUGE_FD, buf, FD_BUFSIZE), EBADF);
        syscall_fail(read(-1, buf, FD_BUFSIZE), EBADF);

        syscall_fail(write(BAD_FD, buf, FD_BUFSIZE), EBADF);
        syscall_fail(write(HUGE_FD, buf, FD_BUFSIZE), EBADF);
        syscall_fail(write(-1, buf, FD_BUFSIZE), EBADF);

        syscall_fail(close(BAD_FD), EBADF);
        syscall_fail(close(HUGE_FD), EBADF);
        syscall_fail(close(-1), EBADF);

        syscall_fail(lseek(BAD_FD, 0, SEEK_SET), EBADF);
        syscall_fail(lseek(HUGE_FD, 0, SEEK_SET), EBADF);
        syscall_fail(lseek(-1, 0, SEEK_SET), EBADF);

        syscall_fail(getdents(BAD_FD, &d, sizeof(d)), EBADF);
        syscall_fail(getdents(HUGE_FD, &d, sizeof(d)), EBADF);
        syscall_fail(getdents(-1, &d, sizeof(d)), EBADF);

        syscall_fail(dup(BAD_FD), EBADF);
        syscall_fail(dup(HUGE_FD), EBADF);
        syscall_fail(dup(-1), EBADF);

        syscall_fail(dup2(BAD_FD, 10), EBADF);
        syscall_fail(dup2(HUGE_FD, 10), EBADF);
        syscall_fail(dup2(-1, 10), EBADF);

        /* dup2 has some extra cases since it takes a second fd */
        syscall_fail(dup2(0, HUGE_FD), EBADF);
        syscall_fail(dup2(0, -1), EBADF);

        /* if the fds are equal, but the first is invalid or out of the
         * allowed range */
        syscall_fail(dup2(BAD_FD, BAD_FD), EBADF);
        syscall_fail(dup2(HUGE_FD, HUGE_FD), EBADF);
        syscall_fail(dup2(-1, -1), EBADF);

        /* dup works properly in normal usage */
        create_file("file01");
        syscall_success(fd1 = open("file01", O_RDWR, 0));
        syscall_success(fd2 = dup(fd1));
        test_assert(fd1 < fd2, "dup(%d) returned %d", fd1, fd2);
        syscall_success(write(fd2, "hello", 5));
        test_fpos(fd1, 5); test_fpos(fd2, 5);
        syscall_success(lseek(fd2, 0, SEEK_SET));
        test_fpos(fd1, 0); test_fpos(fd2, 0);
        read_fd(fd1, 5, "hello");
        test_fpos(fd1, 5); test_fpos(fd2, 5);
        syscall_success(close(fd2));

        /* dup2 works properly in normal usage */
        syscall_success(fd2 = dup2(fd1, 10));
        test_assert(10 == fd2, "dup2(%d, 10) returned %d", fd1, fd2);
        test_fpos(fd1, 5); test_fpos(fd2, 5);
        syscall_success(lseek(fd2, 0, SEEK_SET));
        test_fpos(fd1, 0); test_fpos(fd2, 0);
        syscall_success(close(fd2));

        /* dup2-ing a file to itself works */
        syscall_success(fd2 = dup2(fd1, fd1));
        test_assert(fd1 == fd2, "dup2(%d, %d) returned %d", fd1, fd1, fd2);
        syscall_success(close(fd2));

        /* dup2 closes previous file */
        int fd3;
        create_file("file02");
        syscall_success(fd3 = open("file02", O_RDWR, 0));
        syscall_success(fd2 = dup2(fd1, fd3));
        test_assert(fd2 == fd3, "dup2(%d, %d) returned %d", fd1, fd3, fd2);
        test_fpos(fd1, 0); test_fpos(fd2, 0);
        syscall_success(lseek(fd2, 5, SEEK_SET));
        test_fpos(fd1, 5); test_fpos(fd2, 5);

        syscall_success(chdir(".."));
}

/* These operations should run for a long time and halt when the file
 * descriptor overflows. */
static void
vfstest_infinite(void)
{
        int res, fd;
        char buf[4096];

        res = 1;
        syscall_success(fd = open("/dev/null", O_WRONLY, 0));
        while (0 < res) {
                syscall_success(res = write(fd, buf, sizeof(buf)));
        }
        syscall_success(close(fd));

        res = 1;
        syscall_success(fd = open("/dev/zero", O_RDONLY, 0));
        while (0 < res) {
                syscall_success(res = read(fd, buf, sizeof(buf)));
        }
        syscall_success(close(fd));
}

/*
 * Tests open(), close(), and unlink()
 *      - Accepts only valid combinations of flags
 *      - Cannot open nonexistent file without O_CREAT
 *      - Cannot write to readonly file
 *      - Cannot read from writeonly file
 *      - Cannot close non-existent file descriptor
 *      - Lowest file descriptor is always selected
 *      - Cannot unlink a directory
 #      - Cannot unlink a non-existent file
 *      - Cannot open a directory for writing
 *      - File descriptors are correctly released when a proc exits
 */
static void
vfstest_open(void)
{
#define OPEN_BUFSIZE 5

        char buf[OPEN_BUFSIZE];
        int fd, fd2;
        struct stat s;

        syscall_success(mkdir("open", 0777));
        syscall_success(chdir("open"));

        /* No invalid combinations of O_RDONLY, O_WRONLY, and O_RDWR.  Since
         * O_RDONLY is stupidly defined as 0, the only invalid possible
         * combination is O_WRONLY|O_RDWR. */
        syscall_fail(open("file01", O_WRONLY | O_RDWR | O_CREAT, 0), EINVAL);
        syscall_fail(open("file01", O_RDONLY | O_RDWR | O_WRONLY | O_CREAT, 0), EINVAL);

        /* Cannot open nonexistent file without O_CREAT */
        syscall_fail(open("file02", O_WRONLY, 0), ENOENT);
        syscall_success(fd = open("file02", O_RDONLY | O_CREAT, 0));
        syscall_success(close(fd));
        syscall_success(unlink("file02"));
        syscall_fail(stat("file02", &s), ENOENT);

        /* Cannot create invalid files */
        create_file("tmpfile");
        syscall_fail(open("tmpfile/test", O_RDONLY | O_CREAT, 0), ENOTDIR);
        syscall_fail(open("noent/test", O_RDONLY | O_CREAT, 0), ENOENT);
        syscall_fail(open(LONGNAME, O_RDONLY | O_CREAT, 0), ENAMETOOLONG);

        /* Cannot write to readonly file */
        syscall_success(fd = open("file03", O_RDONLY | O_CREAT, 0));
        syscall_fail(write(fd, "hello", 5), EBADF);
        syscall_success(close(fd));

        /* Cannot read from writeonly file.  Note that we do not unlink() it
         * from above, so we do not need O_CREAT set. */
        syscall_success(fd = open("file03", O_WRONLY, 0));
        syscall_fail(read(fd, buf, OPEN_BUFSIZE), EBADF);
        syscall_success(close(fd));
        syscall_success(unlink("file03"));
        syscall_fail(stat("file03", &s), ENOENT);

        /* Lowest file descriptor is always selected. */
        syscall_success(fd = open("file04", O_RDONLY | O_CREAT, 0));
        syscall_success(fd2 = open("file04", O_RDONLY, 0));
        test_assert(fd2 > fd, "open() did not return lowest fd");
        syscall_success(close(fd));
        syscall_success(close(fd2));
        syscall_success(fd2 = open("file04", O_WRONLY, 0));
        test_assert(fd2 == fd, "open() did not return correct fd");
        syscall_success(close(fd2));
        syscall_success(unlink("file04"));
        syscall_fail(stat("file04", &s), ENOENT);

        /* Cannot open a directory for writing */
        syscall_success(mkdir("file05", 0));
        syscall_fail(open("file05", O_WRONLY, 0), EISDIR);
        syscall_fail(open("file05", O_RDWR, 0), EISDIR);
        syscall_success(rmdir("file05"));

        /* Cannot unlink a directory */
        syscall_success(mkdir("file06", 0));
        syscall_fail(unlink("file06"), EPERM);
        syscall_success(rmdir("file06"));

        /* Cannot unlink a non-existent file */
        syscall_fail(unlink("file07"), ENOENT);

        syscall_success(chdir(".."));
}

static void
vfstest_read(void)
{
#define READ_BUFSIZE 256

        int fd, ret;
        char buf[READ_BUFSIZE];
        struct stat s;

        syscall_success(mkdir("read", 0777));
        syscall_success(chdir("read"));

        /* Can read and write to a file */
        syscall_success(fd = open("file01", O_RDWR | O_CREAT, 0));
        syscall_success(ret = write(fd, "hello", 5));
        test_assert(5 == ret, "write(%d, \"hello\", 5) returned %d", fd, ret);
        syscall_success(ret = lseek(fd, 0, SEEK_SET));
        test_assert(0 == ret, "lseek(%d, 0, SEEK_SET) returned %d", fd, ret);
        read_fd(fd, READ_BUFSIZE, "hello");
        syscall_success(close(fd));

        /* cannot read from a directory */
        syscall_success(mkdir("dir01", 0));
        syscall_success(fd = open("dir01", O_RDONLY, 0));
        syscall_fail(read(fd, buf, READ_BUFSIZE), EISDIR);
        syscall_success(close(fd));

        /* Can seek to beginning, middle, and end of file */
        syscall_success(fd = open("file02", O_RDWR | O_CREAT, 0));
        syscall_success(write(fd, "hello", 5));

#define test_lseek(expr, res)                                                           \
        do {                                                                            \
                int __r = (expr);                                                       \
                test_assert((res) == __r, # expr " returned %d, expected %d", __r, res);\
        } while (0);

        test_lseek(lseek(fd, 0, SEEK_CUR), 5);
        read_fd(fd, 10, "");
        test_lseek(lseek(fd, -1, SEEK_CUR), 4);
        read_fd(fd, 10, "o");
        test_lseek(lseek(fd, 2, SEEK_CUR), 7);
        read_fd(fd, 10, "");
        syscall_fail(lseek(fd, -8, SEEK_CUR), EINVAL);

        test_lseek(lseek(fd, 0, SEEK_SET), 0);
        read_fd(fd, 10, "hello");
        test_lseek(lseek(fd, 3, SEEK_SET), 3);
        read_fd(fd, 10, "lo");
        test_lseek(lseek(fd, 7, SEEK_SET), 7);
        read_fd(fd, 10, "");
        syscall_fail(lseek(fd, -1, SEEK_SET), EINVAL);

        test_lseek(lseek(fd, 0, SEEK_END), 5);
        read_fd(fd, 10, "");
        test_lseek(lseek(fd, -2, SEEK_END), 3);
        read_fd(fd, 10, "lo");
        test_lseek(lseek(fd, 3, SEEK_END), 8);
        read_fd(fd, 10, "");
        syscall_fail(lseek(fd, -8, SEEK_END), EINVAL);

        syscall_fail(lseek(fd, 0, SEEK_SET + SEEK_CUR + SEEK_END), EINVAL);
        syscall_success(close(fd));

        /* O_APPEND works properly */
        create_file("file03");
        syscall_success(fd = open("file03", O_RDWR, 0));
        test_fpos(fd, 0);
        syscall_success(write(fd, "hello", 5));
        test_fpos(fd, 5);
        syscall_success(close(fd));

        syscall_success(fd = open("file03", O_RDWR | O_APPEND, 0));
        test_fpos(fd, 0);
        syscall_success(write(fd, "hello", 5));
        test_fpos(fd, 10);

        syscall_success(lseek(fd, 0, SEEK_SET));
        test_fpos(fd, 0);
        read_fd(fd, 10, "hellohello");
        syscall_success(lseek(fd, 5, SEEK_SET));
        test_fpos(fd, 5);
        syscall_success(write(fd, "again", 5));
        test_fpos(fd, 15);
        syscall_success(lseek(fd, 0, SEEK_SET));
        test_fpos(fd, 0);
        read_fd(fd, 15, "hellohelloagain");
        syscall_success(close(fd));

        /* seek and write beyond end of file */
        create_file("file04");
        syscall_success(fd = open("file04", O_RDWR, 0));
        syscall_success(write(fd, "hello", 5));
        test_fpos(fd, 5);
        test_lseek(lseek(fd, 10, SEEK_SET), 10);
        syscall_success(write(fd, "again", 5));
        syscall_success(stat("file04", &s));
        test_assert(s.st_size == 15, "actual size: %d", s.st_size);
        test_lseek(lseek(fd, 0, SEEK_SET), 0);
        test_assert(15 == read(fd, buf, READ_BUFSIZE), "unexpected number of bytes read");
        test_assert(0 == memcmp(buf, "hello\0\0\0\0\0again", 15), "unexpected data read");
        syscall_success(close(fd));

        syscall_success(chdir(".."));
}

static void
vfstest_getdents(void)
{
        int fd, ret;
        dirent_t dirents[4];

        syscall_success(mkdir("getdents", 0));
        syscall_success(chdir("getdents"));

        /* getdents works */
        syscall_success(mkdir("dir01", 0));
        syscall_success(mkdir("dir01/1", 0));
        create_file("dir01/2");

        syscall_success(fd = open("dir01", O_RDONLY, 0));
        syscall_success(ret = getdents(fd, dirents, 4 * sizeof(dirent_t)));
        test_assert(4 * sizeof(dirent_t) == ret, NULL);

        syscall_success(ret = getdents(fd, dirents, sizeof(dirent_t)));
        test_assert(0 == ret, NULL);

        syscall_success(lseek(fd, 0, SEEK_SET));
        test_fpos(fd, 0);
        syscall_success(ret = getdents(fd, dirents, 2 * sizeof(dirent_t)));
        test_assert(2 * sizeof(dirent_t) == ret, NULL);
        syscall_success(ret = getdents(fd, dirents, 2 * sizeof(dirent_t)));
        test_assert(2 * sizeof(dirent_t) == ret, NULL);
        syscall_success(ret = getdents(fd, dirents, sizeof(dirent_t)));
        test_assert(0 == ret, NULL);
        syscall_success(close(fd));

        /* Cannot call getdents on regular file */
        create_file("file01");
        syscall_success(fd = open("file01", O_RDONLY, 0));
        syscall_fail(getdents(fd, dirents, 4 * sizeof(dirent_t)), ENOTDIR);
        syscall_success(close(fd));

        syscall_success(chdir(".."));
}

#ifdef __VM__
/*
 * Tests link(), rename(), and mmap() (and munmap, and brk).
 * These functions are not supported on testfs, and not included in kernel-land
 * vfs privtest (hence the name)
 */

static void
vfstest_s5fs_vm(void)
{
        int fd, newfd, ret;
        char buf[2048];
        struct stat oldstatbuf, newstatbuf;
        void *addr;

        syscall_success(mkdir("s5fs", 0));
        syscall_success(chdir("s5fs"));

        /* Open some stuff */
        syscall_success(fd = open("oldchld", O_RDWR | O_CREAT, 0));
        syscall_success(mkdir("parent", 0));

        /* link/unlink tests */
        syscall_success(link("oldchld", "newchld"));

        /* Make sure stats match */
        syscall_success(stat("oldchld", &oldstatbuf));
        syscall_success(stat("newchld", &newstatbuf));
        test_assert(0 == memcmp(&oldstatbuf, &newstatbuf, sizeof(struct stat)), NULL);

        /* Make sure contents match */
        syscall_success(newfd = open("newchld", O_RDWR, 0));
        syscall_success(ret = write(fd, TESTSTR, strlen(TESTSTR)));
        test_assert(ret == (int)strlen(TESTSTR), NULL);
        syscall_success(ret = read(newfd, buf, strlen(TESTSTR)));
        test_assert(ret == (int)strlen(TESTSTR), NULL);
        test_assert(0 == strncmp(buf, TESTSTR, strlen(TESTSTR)), "string is %.*s, expected %s", strlen(TESTSTR), buf, TESTSTR);

        syscall_success(close(fd));
        syscall_success(close(newfd));

        /* Remove one, make sure the other remains */
        syscall_success(unlink("oldchld"));
        syscall_fail(mkdir("newchld", 0), EEXIST);
        syscall_success(link("newchld", "oldchld"));

        /* Link/unlink error cases */
        syscall_fail(link("oldchld", "newchld"), EEXIST);
        syscall_fail(link("oldchld", LONGNAME), ENAMETOOLONG);
        syscall_fail(link("parent", "newchld"), EPERM);

        /* only rename test */
        /*syscall_success(rename("oldchld", "newchld"));*/

        /* mmap/munmap tests */
        syscall_success(fd = open("newchld", O_RDWR, 0));
        test_assert(MAP_FAILED != (addr = mmap(0, strlen(TESTSTR), PROT_READ | PROT_WRITE, MAP_PRIVATE, fd, 0)), NULL);
        /* Check contents of memory */
        test_assert(0 == memcmp(addr, TESTSTR, strlen(TESTSTR)), NULL);

        /* Write to it -> we shouldn't pagefault */
        memcpy(addr, SHORTSTR, strlen(SHORTSTR));
        test_assert(0 == memcmp(addr, SHORTSTR, strlen(SHORTSTR)), NULL);

        /* mmap the same thing on top of it, but shared */
        test_assert(MAP_FAILED != mmap(addr, strlen(TESTSTR), PROT_READ | PROT_WRITE, MAP_SHARED | MAP_FIXED, fd, 0), NULL);
        /* Make sure the old contents were restored (the mapping was private) */
        test_assert(0 == memcmp(addr, TESTSTR, strlen(TESTSTR)), NULL);

        /* Now change the contents */
        memcpy(addr, SHORTSTR, strlen(SHORTSTR));
        /* mmap it on, private, on top again */
        test_assert(MAP_FAILED != mmap(addr, strlen(TESTSTR), PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_FIXED, fd, 0), NULL);
        /* Make sure it changed */
        test_assert(0 == memcmp(addr, SHORTSTR, strlen(SHORTSTR)), NULL);

        /* Fork and try changing things */
        if (!fork()) {
                /* Child changes private mapping */
                memcpy(addr, TESTSTR, strlen(TESTSTR));
                exit(0);
        }

        /* Wait until child is done */
        syscall_success(wait(0));

        /* Make sure it's actually private */
        test_assert(0 == memcmp(addr, SHORTSTR, strlen(SHORTSTR)), NULL);

        /* Unmap it */
        syscall_success(munmap(addr, 2048));

        /* mmap errors */
        test_assert(MAP_FAILED == mmap(0, 1024, PROT_READ, MAP_PRIVATE, 12, 0), NULL);
        test_assert(MAP_FAILED == mmap(0, 1024, PROT_READ, MAP_PRIVATE, -1, 0), NULL);
        test_assert(MAP_FAILED == mmap(0, 1024, PROT_READ, 0, fd, 0), NULL);
        test_assert(MAP_FAILED == mmap(0, 1024, PROT_READ, MAP_FIXED, fd, 0), NULL);
        test_assert(MAP_FAILED == mmap(0, 1024, PROT_READ, MAP_FIXED | MAP_PRIVATE, fd, 0), NULL);
        test_assert(MAP_FAILED == mmap(0, 1024, PROT_READ, MAP_PRIVATE, fd, 0x12345), NULL);
        test_assert(MAP_FAILED == mmap((void *) 0x12345, 1024, PROT_READ, MAP_PRIVATE | MAP_FIXED, fd, 0), NULL);
        test_assert(MAP_FAILED == mmap(0, 0, PROT_READ, MAP_PRIVATE, fd, 0), NULL);
        test_assert(MAP_FAILED == mmap(0, -1, PROT_READ, MAP_PRIVATE, fd, 0), NULL);
        test_assert(MAP_FAILED == mmap(0, 1024, PROT_READ, MAP_PRIVATE | MAP_FIXED, fd, 0), NULL);
        syscall_success(close(fd));

        syscall_success(fd = open("newchld", O_RDONLY, 0));
        test_assert(MAP_FAILED == mmap(0, 1024, PROT_READ | PROT_WRITE, MAP_SHARED, fd, 0), NULL);
        syscall_success(close(fd));

        /* TODO ENODEV (mmap a terminal)
           EOVERFLOW (mmap SO MUCH of /dev/zero that fpointer would overflow) */

        /* Also should test opening too many file descriptors somewhere */

        /* munmap errors */
        syscall_fail(munmap((void *) 0x12345, 15), EINVAL);
        syscall_fail(munmap(0x0, 15), EINVAL);
        syscall_fail(munmap(addr, 0), EINVAL);
        syscall_fail(munmap(addr, -1), EINVAL);

        /* brk tests */
        /* Set the break, and use the memory in question */
        test_assert((void *) - 1 != (addr = sbrk(128)), NULL);
        memcpy(addr, TESTSTR, 128);
        test_assert(0 == memcmp(addr, TESTSTR, 128), NULL);

        /* Make sure that the brk is being saved properly */
        test_assert((void *)((unsigned long) addr + 128) == sbrk(0), NULL);
        /* Knock the break back down */
        syscall_success(brk(addr));

        /* brk errors */
        syscall_fail(brk((void *)(&"brk")), ENOMEM);
        syscall_fail(brk((void *) 1), ENOMEM);
        syscall_fail(brk((void *) &addr), ENOMEM);

        syscall_success(chdir(".."));
}
#endif

/*
 * Finally, the main function.
 */
#ifndef __KERNEL__
int main(int argc, char **argv)
#else
int vfstest_main(int argc, char **argv)
#endif
{
        if (argc != 1) {
                fprintf(stderr, "USAGE: vfstest\n");
                return 1;
        }

        test_init();
        vfstest_start();

        syscall_success(chdir(root_dir));

        vfstest_stat();
        vfstest_chdir();
        vfstest_mkdir();
        vfstest_paths();
        vfstest_fd();
        vfstest_open();
        vfstest_read();
        vfstest_getdents();

#ifdef __VM__
        vfstest_s5fs_vm();
#endif

        /*vfstest_infinite();*/

        syscall_success(chdir(".."));

        vfstest_term();
        test_fini();

        return 0;
}
