/*
 *  File: stress.c
 *  Date: 16 November 1998
 *  Acct: David Powell (dep)
 *  Desc: Miscellaneous VM tests
 */

#include <unistd.h>
#include <sys/mman.h>
#include <fcntl.h>
#include <sys/types.h>
#include <stdlib.h>
#include <string.h>
#include <stdio.h>
#include <errno.h>

static void check_failed(const char *cmd)
{
        (void) printf("stress: %s failed: errno %d\n", cmd, errno);
        exit(1);
}

static int myfork()
{
        int result;

        result = fork();
        if (result == -1) {
                (void) printf("Fork failed (errno=%d)\n", errno);
                exit(1);
        }

        yield();
        return result;
}

static void fork_test()
{
        (void) printf("-- Fork torture test start\n");

        (void) printf("The final test: forking up a storm.\n"
                      "If this doesn't crash your kernel, "
                      "you might be in good shape\n");
        (void) printf("(note that this is running in the background)\n");
        if (!myfork()) {
                for (;;) {
                        if (myfork()) {
                                exit(0);
                        }
                }
        }
}

static void cow_fork()
{
        int     status;
        int     foo = 0;

        (void) printf("-- COW fork test start\n");

        if (!myfork()) {
                /* We are in the child process, and should be accessing
                 * our own memory
                 */
                foo = 1;
                exit(0);
        }

        if (wait(&status) == -1) {
                (void) printf("wait failed (errno=%d)\n", errno);
                exit(1);
        }

        if (foo) {
                (void) printf("Data changed in child affected parent.\n"
                              "Make sure you mark writable private mappings copy-on-write.\n");
                (void) printf("Copy-on-write failed.\n");
                exit(1);
        }

        (void) printf("-- COW fork test passed\n");
}

static void fault_test()
{
        int     status;

        (void) printf("-- fault test start\n");

        (void) printf("Fault test.  If this hangs, check your page fault handler...\n");
        (void) printf("Do you properly kill processes that segv?  ");
        if (!myfork()) {
                *(int *)0 = 0;
                exit(0);
        }

        if (wait(&status) == -1) {
                (void) printf("wait failed (errno=%d)\n", errno);
                exit(1);
        }

        /* This assumes that killing the process will set the status to
         * something other than 0
         */
        if (status) {
                (void) printf("yes\n");
        } else {
                (void) printf("no\n");
                exit(1);
        }

        (void) printf("-- fault test passed\n");
}


void mmap_test()
{
        int             fd;
        void            *addr1, *addr2;
        const char      *str1 = "Coconuts!!!!\n";
        const char      *str2 = "Hello there.\n";
        size_t          len;

        (void) printf("-- mmap test start\n");

        /* Create a file with some data. */

        fd = open("/test/stress0", O_RDWR | O_CREAT, 0);
        if (fd < 0) {
                check_failed("open");
        }

        /* Give us some space */
        if (1 > write(fd, "\0", 1))
                check_failed("write");

        /* Map the file MAP_PRIVATE */

        printf("MAP_PRIVATE test\n");
        len = strlen(str1) + 1;
        addr1 = mmap(0, len, PROT_READ | PROT_WRITE, MAP_SHARED, fd, 0);
        if (addr1 == MAP_FAILED) {
                check_failed("mmap");
        }
        addr2 = mmap(0, len, PROT_READ | PROT_WRITE, MAP_PRIVATE, fd, 0);
        if (addr2 == MAP_FAILED) {
                check_failed("mmap");
        }

        if (close(fd)) {
                check_failed("close");
        }

        printf("writing into %p\n", addr1);
        (void) snprintf((char *)addr1, len, "%s", str1);

        /* Read from the private mapped page for good measure
         * (this _shouldn't_ do anything) */
        printf("reading from privmap page\n");

        /* Verify that the string is initially in the mapping. */

        printf("making sure string in mapping okay\n");
        if (strcmp(str1, (char *)addr1)) {
                (void) printf("stress: write to shared mapping failed\n");
                exit(1);
        }
        if (strcmp(str1, (char *)addr2)) {
                (void) printf("stress: private mapping prematurely copied\n");
                exit(1);
        }

        (void) snprintf((char *)addr2, len, "%s", str2);

        /* Verify that the string has been overwritten in the mapping. */

        printf("making sure overwriting okay\n");
        if (strcmp(str2, (char *)addr2)) {
                (void) printf("stress: write to private mapping failed\n");
                exit(1);
        }

        if (!strcmp(str2, (char *)addr1)) {
                (void) printf("stress: wrote through private mapping!\n");
                exit(1);
        }

        printf("unmapping at %p\n", addr1);
        if (munmap(addr1, len)) {
                check_failed("munmap");
        }
        printf("unmapping at %p\n", addr2);
        if (munmap(addr2, len)) {
                check_failed("munmap");
        }

        if (!munmap((void *)0xc0000000, 15)       || (errno != EINVAL)) {
                printf("munmap bad one fail errno=%d einval=%d\n", errno, EINVAL);
                exit(1);
        }

        if (!munmap(0, 0)               || (errno != EINVAL)) {
                printf("munmap bad two fail errno=%d einval=%d\n", errno, EINVAL);
                exit(1);
        }

        if (!munmap((void *)137, 100)            || (errno != EINVAL)) {
                printf("munmap bad three fail errno=%d einval=%d\n", errno, EINVAL);
                exit(1);
        }

        (void) printf("-- mmap test passed\n");
}


void null_test()
{
        int             fd;
        int             nbytes;
        char            buf[256];

        (void) printf("-- null test start\n");

        fd = open("/dev/null", O_RDWR, 0600);
        if (fd < 0) {
                check_failed("open");
        }

        (void) memset(buf, 0xCC, sizeof(buf));

        /* Try writing to /dev/null.  Should return buffer size.
        */

        nbytes = write(fd, buf, sizeof(buf));
        if (nbytes != sizeof(buf)) {
                check_failed("write");
        }

        /* Try reading from /dev/null.  Should return zero.
        */

        nbytes = read(fd, buf, sizeof(buf));
        if (nbytes != 0) {
                check_failed("read");
        }

        if (close(fd)) {
                check_failed("close");
        }

        (void) printf("-- null test passed\n");
}


void zero_test()
{
        void *addr;
        int fd;
        char buf[256];
        int nbytes;
        unsigned int ii;
        size_t len;
        unsigned long *lp;
        unsigned char *cp;

        (void) printf("-- zero test start\n");

        fd = open("/dev/zero", O_RDWR, 0600);
        if (fd < 0) {
                check_failed("open");
        }

        /* Set buffer to a non-zero value, then read from /dev/zero
         * and make sure that the buffer is cleared.
         */

        memset(buf, 0xCC, sizeof(buf));

        nbytes = read(fd, buf, sizeof(buf));
        if (nbytes != sizeof(buf)) {
                check_failed("read");
        }

        for (ii = 0; ii < sizeof(buf); ii++) {
                if (buf[ii] != 0) {
                        printf("read %x not zero\n", buf[ii]);
                        check_failed("verify read");
                }
        }

        /* Map /dev/zero and make sure all pages are initially zero.
        */

        len = 8192 * 5;

        addr = mmap(0, len, PROT_READ | PROT_WRITE, MAP_PRIVATE, fd, 0);
        if (addr == MAP_FAILED) {
                check_failed("mmap");
        }

        if (close(fd)) {
                check_failed("close");
        }

        cp = (unsigned char *) addr;
        for (ii = 0; ii < len; ii++, cp++) {
                if (*cp != 0) {
                        check_failed("verify mmap zeros");
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
                        check_failed("verify map write");
                }
        }

        if (munmap(addr, len)) {
                check_failed("munmap");
        }

        (void) printf("-- zero test passed\n");
}

void wait_test()
{
        int     status;

        (void) printf("-- wait test start\n");

        if (!wait(&status) || (errno != ECHILD)) {
                (void) printf("error: wait() didn't return an error of "
                              "ECHILD when no children existed!\n");
                exit(1);
        }

        (void) printf("-- wait test passed\n");
}

void brk_test()
{
        void *oldbrk1, *oldbrk2;
        const void *brk_failed = (void *) - 1;
        int len;
        unsigned int *tmp;
        unsigned int ii;

        (void) printf("-- brk test start\n");

        /* A length which is not a page multiple, yet a multiple of 8.
        */
        len = 8192 * 5 + 128;

        /* Try allocating some memory.
        */
        oldbrk1 = sbrk(len);
        if (oldbrk1 == brk_failed) {
                check_failed("sbrk alloc");
        }

        /* Try writing to the memory.
        */
        printf("writing to memory at %p\n", oldbrk1);
        tmp = (unsigned int *) oldbrk1;
        for (ii = 0; ii < (len / sizeof(int)); ii++)
                *tmp++ = ii;

        /* Try verifying what we wrote.
        */
        printf("verifying memory\n");
        tmp = (unsigned int *) oldbrk1;
        for (ii = 0; ii < (len / sizeof(int)); ii++) {
                if (*tmp++ != ii) {
                        (void) printf("verify failed at 0x%lx\n",
                                      (unsigned long) tmp);
                        exit(1);
                }
        }

        /* Try freeing the memory.
        */
        printf("freeing memory\n");
        oldbrk2 = sbrk(-len);
        if (oldbrk2 == brk_failed) {
                check_failed("sbrk dealloc");
        }

        /* oldbrk2 should be at least "len" greater than oldbrk1.
        */
        if ((unsigned long) oldbrk2 < ((unsigned long) oldbrk1 + len)) {
                (void) printf("sbrk didn't return old brk??\n");
                exit(1);
        }

        (void) printf("-- brk test passed\n");
}

int main(int argc, char **argv)
{
        (void) printf("Congrats!  You're running this executable.\n");
        (void) printf("Now let's see how you handle the tests...\n");


        mmap_test();

        null_test();
        zero_test();
        brk_test();

        fault_test();

        wait_test();
        cow_fork();

        fork_test();

        return 0;
}

