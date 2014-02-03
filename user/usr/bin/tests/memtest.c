/*
 * Test correct user space memory management, particularly segfaults
 * Tests fun cases of mmap, munmap, and brk
 * -- Alvin Kerber (alvin)
 */

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

/* Shared header trickery */
#include "page.h"
#include "mm.h"

#include "linkermagic.h"

/* Helpful defines */
#define assert_fault(statement, msg)            \
        do {                                    \
                int __status;                   \
                test_fork_begin() {             \
                        statement;              \
                        return 0;               \
                } test_fork_end(&__status);     \
                test_assert(EFAULT == __status, "Unexpected lack of segfault on " #statement " : " msg); \
        } while (0);

#define assert_nofault(statement, msg)          \
        do {                                    \
                int __status;                   \
                test_fork_begin() {             \
                        statement;              \
                        return 0;               \
                } test_fork_end(&__status);     \
                test_assert(0 == __status, "Unexpected segfault on " #statement " : " msg); \
        } while (0);

/* Overflow the stack */
static void overflow(void)
{
        int junk[1000];
        overflow();
}

static int test_overflow(void)
{
        printf("Testing stack overflow\n");
        assert_fault(overflow(), "Stack overflow");
        return 0;
}

static int test_mmap_bounds(void)
{
        int fd, status;
        void *addr;

        printf("Testing boundaries and permissions of mmap()\n");

        test_assert(0 < (fd = open("/dev/zero", O_RDWR, 0)), NULL);
        test_assert(MAP_FAILED != (addr = mmap(NULL, PAGE_SIZE * 3,
                                               PROT_READ | PROT_WRITE, MAP_PRIVATE, fd, 0)), NULL);
        /* Make sure we can actually access these addresses */
        test_assert('\0' == *(char *)addr, NULL);
        test_assert('\0' == *((char *)addr + PAGE_SIZE), NULL);
        test_assert('\0' == *((char *)addr + PAGE_SIZE * 2), NULL);
        test_assert('\0' == *((char *)addr + PAGE_SIZE * 3 - 1), NULL);

        /* Unmap the ends */
        test_assert(0 == munmap(addr, PAGE_SIZE), NULL);
        test_assert(0 == munmap((char *)addr + PAGE_SIZE * 2, PAGE_SIZE), NULL);

        /* Adjust to center, now surrounded by unmapped regions */
        addr = (char *)addr + PAGE_SIZE;

        /* Make sure we didn't unmap the middle */
        test_assert('\0' == *((char *)addr), NULL);
        test_assert('\0' == *((char *)addr + PAGE_SIZE - 1), NULL);
        assert_nofault(*(char *)addr = 'a', "");
        assert_nofault(*((char *)addr + PAGE_SIZE - 1) = 'b', "");

        /* Regions around it are unmapped */
        assert_fault(char foo = *((char *) addr + PAGE_SIZE), "");
        assert_fault(char foo = *((char *) addr - PAGE_SIZE), "");
        assert_fault(char foo = *((char *) addr - 1), "");
        assert_fault(*((char *) addr + PAGE_SIZE) = 'a', "");
        assert_fault(*((char *) addr - 1) = 'a', "");
        assert_fault(*((char *) addr + PAGE_SIZE * 2 - 1) = 'a', "");

        /* Remap as read-only */
        test_assert(addr == mmap(addr, 1,
                                 PROT_READ, MAP_PRIVATE | MAP_FIXED, fd, 0), NULL);

        assert_fault(*((char *) addr) = 'a', "");
        assert_fault(*((char *) addr + PAGE_SIZE - 1) = 'a', "");

        /* "Unmap" */
        test_assert(0 == munmap((char *)addr - PAGE_SIZE, PAGE_SIZE), NULL);
        test_assert(0 == munmap((char *)addr + PAGE_SIZE, PAGE_SIZE), NULL);

        /* Make sure it's still there, also that it's overwritten */
        test_assert('\0' == *((char *)addr), NULL);
        test_assert('\0' == *((char *)addr + PAGE_SIZE - 1), NULL);

        /* Unmap for real */
        test_assert(0 == munmap(addr, 1), NULL);

        assert_fault(char foo = *(char *) addr, "");
        assert_fault(char foo = *((char *) addr + PAGE_SIZE - 1), "");

        /* Test fun permissions */
        test_assert(addr == mmap(addr, PAGE_SIZE,
                                 PROT_EXEC, MAP_PRIVATE | MAP_FIXED, fd, 0), NULL);
        assert_fault(char foo = *(char *) addr, "");
        assert_fault(char foo = *((char *) addr + PAGE_SIZE - 1), "");
        assert_fault(*((char *) addr) = 'a', "");

        test_assert(addr == mmap(addr, PAGE_SIZE,
                                 0, MAP_PRIVATE | MAP_FIXED, fd, 0), NULL);
        assert_fault(char foo = *(char *) addr, "");
        assert_fault(char foo = *((char *) addr + PAGE_SIZE - 1), "");
        assert_fault(*((char *) addr) = 'a', "");


        return 0;
}

static int test_brk_bounds(void)
{
        void *oldbrk, *newbrk;
        int status;

        printf("Testing boundaries and permissions of brk()\n");

        /* "Stabilize" our old brk at a page boundary */
        test_assert((void *) - 1 != (oldbrk = sbrk(0)), NULL);
        oldbrk = PAGE_ALIGN_UP(oldbrk);
        test_assert(0 == brk(oldbrk), NULL);

        /* Look at next page-aligned addr */
        newbrk = (char *)oldbrk + PAGE_SIZE;

        assert_fault(char foo = *(char *)newbrk, "");
        assert_fault(*(char *)newbrk = 'a', "");

        /* Move brk to next page-aligned addr */
        test_assert(0 == brk(newbrk), NULL);

        /* Access the new memory */
        test_assert('\0' == *(char *)oldbrk, NULL);
        test_assert('\0' == *((char *)newbrk - 1), NULL);
        *((char *)newbrk - 1) = 'a';

        assert_fault(char foo = *(char *)newbrk, "");
        assert_fault(*(char *)newbrk = 'a', "");

        /* Move brk up by 1 byte */
        test_assert(0 == brk((char *)newbrk + 1), NULL);

        /* Access the new memory */
        test_assert('\0' == *(char *)newbrk, NULL);
        test_assert('\0' == *((char *)newbrk + PAGE_SIZE - 1), NULL);
        assert_nofault(*(char *)newbrk = 'b', "");

        /* Old memory didn't change */
        test_assert('a' == *((char *)newbrk - 1), NULL);

        /* Move it back */
        test_assert(0 == brk(newbrk), NULL);

        assert_fault(char foo = *(char *)newbrk, "");
        assert_fault(*(char *)newbrk = 'a', "");

        /* Move it up, make sure region wiped. Note that the actual wipe test is
         * 'evil' and is in eviltest. This just checks to make sure the brk region
         * is private mapped (modified in subprocesses) */
        test_assert(0 == brk((char *)newbrk + PAGE_SIZE), NULL);
        test_assert('\0' == *(char *)newbrk, NULL);
        test_assert('\0' == *((char *)newbrk + PAGE_SIZE - 1), NULL);

        /* Move it down by 1 byte */
        test_assert(0 == brk((char *)newbrk - 1), NULL);

        /* Access still-accessible memory */
        test_assert('a' == *((char *)newbrk - 1), NULL);
        *((char *)newbrk - 2) = 'z';

        /* Move brk to multiple addrs on same page, make sure page remains */
        test_assert(0 == brk((char *)newbrk - 1000), NULL);
        test_assert('z' == *((char *)newbrk - 2), NULL);
        test_assert(0 == brk((char *)oldbrk + 1), NULL);
        test_assert('z' == *((char *)newbrk - 2), NULL);
        test_assert(0 == brk((char *)oldbrk + 1000), NULL);
        test_assert('a' == *((char *)newbrk - 1), NULL);

        return 0;
}

static int test_munmap(void)
{
        char *addr, *middle;

        printf("Testing munmap()\n");

        /* Map lots of areas. We're kind of lazy, so for now they're all anonymous */
        test_assert(MAP_FAILED != (addr = mmap(NULL, PAGE_SIZE * 20,
                                               PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANON, -1, 0)), NULL);

        *(addr + PAGE_SIZE * 8) = '^';
        *(addr + PAGE_SIZE * 12) = '$';

        /* Make sure TLB / page tables are cleared on unmap */
        assert_fault(*addr = 'a';
                     munmap(addr, PAGE_SIZE);
                     char foo = *addr; , "");
        assert_fault(*addr = 'a';
                     munmap(addr, PAGE_SIZE * 20);
                     char foo = *addr; , "");
        assert_fault(*(addr + PAGE_SIZE * 10) = 'a';
                     munmap(addr + PAGE_SIZE * 10, PAGE_SIZE * 5);
                     char foo = *(addr + PAGE_SIZE * 10); , "");

        /* Overwrite middle of area (implicit unmap) */
        test_assert(MAP_FAILED != (middle = mmap(addr + PAGE_SIZE * 10, PAGE_SIZE,
                                            PROT_READ | PROT_WRITE, MAP_SHARED | MAP_ANON | MAP_FIXED, -1, 0)), NULL);

        /* Make sure we overwrote the middle but not the whole thing */
        test_assert('\0' == *middle, NULL);
        assert_nofault(*middle = 'a', "");
        test_assert('a' == *middle, NULL);

        test_assert('\0' == *(addr + PAGE_SIZE * 9), NULL);
        assert_nofault(*(addr + PAGE_SIZE * 9) = 'a', "");
        test_assert('\0' == *(addr + PAGE_SIZE * 9), NULL);

        test_assert('\0' == *(addr + PAGE_SIZE * 11), NULL);
        assert_nofault(*(addr + PAGE_SIZE * 11) = 'a', "");
        test_assert('\0' == *(addr + PAGE_SIZE * 11), NULL);

        test_assert('\0' == *addr, NULL);
        test_assert('\0' == *(addr + PAGE_SIZE * 20 - 1), NULL);

        /* Make sure the offsets are appropriate */
        test_assert('^' == *(addr + PAGE_SIZE * 8), NULL);
        test_assert('$' == *(addr + PAGE_SIZE * 12), NULL);

        /* Unmap a weird overlapping region */
        test_assert(0 == munmap(addr + PAGE_SIZE * 9, PAGE_SIZE * 3), NULL);

        /* Make sure everything's gone */
        assert_fault(char foo = *(addr + PAGE_SIZE * 9), "");
        assert_fault(char foo = *(addr + PAGE_SIZE * 10), "");
        assert_fault(char foo = *(addr + PAGE_SIZE * 12 - 1), "");

        /* Make sure offsets are still correct */
        test_assert('^' == *(addr + PAGE_SIZE * 8), NULL);
        test_assert('$' == *(addr + PAGE_SIZE * 12), NULL);

        /* Unmap nothing at all */
        test_assert(0 == munmap(addr + PAGE_SIZE * 10, PAGE_SIZE), NULL);
        test_assert(0 == munmap(addr + PAGE_SIZE * 9, PAGE_SIZE * 3), NULL);

        /* Unmap almost the whole (remaining) thing */
        test_assert(0 == munmap(addr + PAGE_SIZE, PAGE_SIZE * 19), NULL);

        /* Make sure the beginning's still there */
        test_assert('\0' == *addr, NULL);

        /* Finish up, make sure everything's gone */
        test_assert(0 == munmap(addr, PAGE_SIZE * 15), NULL);
        assert_fault(char foo = *(addr + PAGE_SIZE), "");
        assert_fault(char foo = *(addr), "");
        assert_fault(char foo = *(addr + PAGE_SIZE * 20 - 1), "");

        return 0;
}

static int test_start_brk(void)
{
        printf("Testing using brk() near starting brk\n");
        test_assert(bss_end == sbrk(0), "brk should not have moved yet");
        test_assert(!PAGE_ALIGNED(bss_end) && !PAGE_ALIGNED((char *)bss_end + 1), "starting brk is page aligned; test is too easy...");

        /* Up to next page boundary should already be accessible (end of bss) */
        char *oldbrk = PAGE_ALIGN_UP(bss_end);
        test_assert('\0' == *(oldbrk - 1), NULL);
        *(oldbrk - 1) = 'a';
        assert_fault(char foo = *oldbrk, "");

        /* Move brk up to next page boundary */
        test_assert(0 == brk(oldbrk), NULL);
        test_assert('a' == *(oldbrk - 1), NULL);
        *(oldbrk - 1) = 'b';
        assert_fault(char foo = *oldbrk, "");
        assert_fault(char foo = *(oldbrk + PAGE_SIZE), "");

        /* Try to move before starting brk */
        test_assert(0 != brk((char *)bss_end - 1), NULL);
        test_assert(0 != brk(PAGE_ALIGN_DOWN(bss_end)), NULL);

        /* Move it up another page */
        char *newbrk = oldbrk + PAGE_SIZE;
        test_assert(0 == brk(newbrk), NULL);

        /* Make sure everything accessible (read/write) */
        test_assert('b' == *(oldbrk - 1), NULL);
        test_assert('\0' == *oldbrk, NULL);
        test_assert('\0' == *(newbrk - 1), NULL);
        *oldbrk = 'z';
        *(newbrk - 1) = 'y';
        assert_fault(char foo = *newbrk, "");
        assert_fault(char foo = *(newbrk + PAGE_SIZE), "");

        /* Try to move before starting brk */
        test_assert(0 != brk((char *)bss_end - 1), NULL);
        test_assert(0 != brk(PAGE_ALIGN_DOWN(bss_end)), NULL);

        /* Move back to starting brk */
        test_assert(0 == brk((char *)bss_end + 1), NULL);
        /* Make sure region is gone */
        test_assert('b' == *(oldbrk - 1), NULL);
        assert_fault(char foo = *oldbrk, "");
        assert_fault(char foo = *newbrk, "");

        /* Move it up, make sure we have new clean region */
        test_assert(0 == brk(oldbrk + 1), NULL);
        /* This behavior is undefined, this represents how it
         * works on Linux but these need not pass
         */
        /*test_assert('\0' == *oldbrk, NULL);
        test_assert('\0' == *(newbrk - 1), NULL);*/
        assert_fault(char foo = *newbrk, "");

        /* Move back and finish */
        test_assert(0 == brk(bss_end), NULL);
        test_assert('b' == *(oldbrk - 1), NULL);

        return 0;
}

static int test_brk_mmap(void)
{
        printf("Testing interactions of brk() and mmap()\n");
        test_assert(bss_end == sbrk(0), "brk should not have moved yet");
        char *oldbrk = PAGE_ALIGN_UP(bss_end);

        /* Put a mapping in the way */
        test_assert(MAP_FAILED != mmap(oldbrk, PAGE_SIZE * 2,
                                       PROT_READ, MAP_ANON | MAP_FIXED | MAP_PRIVATE, -1, 0), NULL);
        /* Mapping is there */
        test_assert('\0' == *oldbrk, NULL);
        test_assert('\0' == *(oldbrk - 1), NULL);

        /* Moving brk without getting area is fine */
        test_assert(0 == brk(oldbrk), NULL);
        test_assert('\0' == *oldbrk, NULL);
        test_assert('\0' == *(oldbrk - 1), NULL);
        test_assert(0 == brk((char *)bss_end + 1), NULL);
        test_assert('\0' == *oldbrk, NULL);
        test_assert('\0' == *(oldbrk - 1), NULL);

        /* But can't move it up at all */
        test_assert(0 != brk(oldbrk + 1), NULL);
        test_assert(0 != brk(oldbrk + PAGE_SIZE), NULL);
        test_assert(0 != brk(oldbrk + PAGE_SIZE * 2), NULL);
        test_assert(0 != brk(oldbrk + PAGE_SIZE * 3), NULL);

        /* Make it smaller */
        test_assert(0 == munmap(oldbrk, PAGE_SIZE), NULL);
        /* Region inaccessible */
        assert_fault(char foo = *oldbrk, "");
        assert_fault(char foo = *(oldbrk + PAGE_SIZE - 1), "");

        /* Expand brk accordingly */
        test_assert(0 == brk(oldbrk + PAGE_SIZE), NULL);
        test_assert('\0' == *oldbrk, NULL);
        test_assert('\0' == *(oldbrk + PAGE_SIZE - 1), NULL);
        *oldbrk = 'a';

        /* Can't go too far */
        test_assert(0 != brk(oldbrk + PAGE_SIZE + 1), NULL);
        test_assert(0 != brk(oldbrk + PAGE_SIZE * 2), NULL);
        test_assert(0 != brk(oldbrk + PAGE_SIZE * 3), NULL);

        return 0;
}

static int test_mmap_fill(void)
{
        printf("Testing filling up virtual address space\n");
        char *hi, *lo, *addr;
        /* map something and remove it to find out how high we can go */
        test_assert(MAP_FAILED != (hi = mmap(NULL, 1,
                                             0, MAP_ANON | MAP_PRIVATE, -1, 0)), NULL);
        test_assert(0 == munmap(hi, 1), NULL);
        hi += PAGE_SIZE;

        test_assert(bss_end == sbrk(0), NULL);
        lo = PAGE_ALIGN_UP(bss_end);

        /* Fill this up with 2 mappings */
#define MID_ADDR ((char *)0x80000000)
        if (MID_ADDR > lo) {
                test_assert(MID_ADDR == mmap(NULL,
                                             (size_t)((uintptr_t)hi - (uintptr_t)MID_ADDR),
                                             0, MAP_ANON | MAP_PRIVATE, -1, 0), NULL);
        }
        if (MID_ADDR < hi) {
                test_assert(lo == mmap(NULL,
                                       (size_t)((uintptr_t)MID_ADDR - (uintptr_t)lo),
                                       0, MAP_ANON | MAP_PRIVATE, -1, 0), NULL);
        }

        /* mmap file below stack */
        test_assert(MAP_FAILED != (addr = mmap(NULL, 1,
                                               PROT_READ, MAP_ANON | MAP_PRIVATE, -1, 0)), NULL);
        test_assert((uintptr_t)addr < (uintptr_t)&addr, NULL);
        test_assert('\0' == *addr, NULL);
        /* mmap fixed on top of it */
        test_assert(MAP_FAILED != mmap(addr, 1,
                                       PROT_READ, MAP_FIXED | MAP_ANON | MAP_PRIVATE, -1, 0), NULL);
        test_assert('\0' == *addr, NULL);

        /* Try something too big */
        test_assert(MAP_FAILED == mmap(NULL, (size_t)addr,
                                       0, MAP_ANON | MAP_PRIVATE, -1, 0), NULL);

        /* Fill up the entire remaining space */
        test_assert(MAP_FAILED != mmap(NULL,
                                       (size_t)((uintptr_t)addr - (uintptr_t)USER_MEM_LOW),
                                       0, MAP_ANON | MAP_PRIVATE, -1, 0), NULL);


        /* No space left */
        test_assert(MAP_FAILED == mmap(NULL, 1,
                                       0, MAP_ANON | MAP_PRIVATE, -1, 0), NULL);

        /* Make some space, we should fill it */
        test_assert(0 == munmap(addr, 1), NULL);
        test_assert(addr == mmap(NULL, 1,
                                 PROT_READ, MAP_ANON | MAP_PRIVATE, -1, 0), NULL);
        test_assert('\0' == *addr, NULL);

        test_assert(MAP_FAILED == mmap(NULL, 1,
                                       0, MAP_ANON | MAP_PRIVATE, -1, 0), NULL);

        /* Clean out some more space */
        test_assert(0 == munmap(MID_ADDR - PAGE_SIZE, PAGE_SIZE * 2), NULL);
        test_assert(MID_ADDR - PAGE_SIZE == mmap(NULL, PAGE_SIZE * 2,
                        PROT_READ, MAP_ANON | MAP_PRIVATE, -1, 0), NULL);
        test_assert('\0' == *MID_ADDR, NULL);
        test_assert('\0' == *(MID_ADDR - PAGE_SIZE), NULL);
        test_assert('\0' == *(MID_ADDR + PAGE_SIZE - 1), NULL);

        /* Cut into pieces, access each of them */
        char *p;
        for (p = lo + PAGE_SIZE; p < lo + PAGE_SIZE * 20; p += PAGE_SIZE * 2) {
                test_assert(MAP_FAILED != mmap(p, 1, PROT_READ | PROT_WRITE,
                                               MAP_ANON | MAP_PRIVATE | MAP_FIXED, -1, 0), NULL);
                test_assert('\0' == *p, NULL);
                *p = 'a';
                assert_fault(char foo = *(p + PAGE_SIZE), "");
        }

        test_assert(MAP_FAILED == mmap(NULL, 1,
                                       0, MAP_ANON | MAP_PRIVATE, -1, 0), NULL);

        /* Try brk too */
        test_assert(0 == brk(lo), NULL);
        test_assert(0 != brk(lo + 1), NULL);

        /* Clean it all up */
        test_assert(0 == munmap(lo, (size_t)((uintptr_t)hi - (uintptr_t)lo)), NULL);
        return 0;
}

static int test_mmap_repeat(void)
{
#define MMAP_REPEAT_FILE "mmaprepeattest"
#define REPEAT_STR "FooFooFoo"

        int fd, i;
        char *addrs[10];
        printf("Testing repeated mmap() of same file\n");

        /* Set up test file */
        test_assert(-1 != (fd = open(MMAP_REPEAT_FILE, O_RDWR | O_CREAT, 0)), NULL);
        test_assert(10 == write(fd, REPEAT_STR, 10), NULL);
        test_assert(0 == unlink(MMAP_REPEAT_FILE), NULL);

        /* map it private many times */
        for (i = 0; i < 10; i++) {
                test_assert(MAP_FAILED != (addrs[i] = mmap(NULL, PAGE_SIZE,
                                                      PROT_READ | PROT_WRITE, MAP_PRIVATE, fd, 0)), NULL);
                test_assert(!strcmp(addrs[i], REPEAT_STR), NULL);
        }
        /* Make sure changes don't propagate */
        *addrs[0] = 'Z';
        *(addrs[0] + PAGE_SIZE - 1) = 'Q';
        for (i = 1; i < 10; i++) {
                test_assert(!strcmp(addrs[i], REPEAT_STR), NULL);
                test_assert('\0' == *(addrs[i] + PAGE_SIZE - 1), NULL);
        }

        /* map it shared many times */
        for (i = 0; i < 10; i++) {
                test_assert(MAP_FAILED != (addrs[i] = mmap(NULL, PAGE_SIZE,
                                                      PROT_READ | PROT_WRITE, MAP_SHARED, fd, 0)), NULL);
                test_assert(!strcmp(addrs[i], REPEAT_STR), NULL);
        }
        /* Make sure changes propagate */
        *addrs[3] = 'Z';
        *(addrs[5] + PAGE_SIZE - 1) = 'Q';
        for (i = 0; i < 10; i++) {
                test_assert('Z' == *addrs[i], NULL);
                test_assert('Q' == *(addrs[i] + PAGE_SIZE - 1), NULL);
        }

        return 0;
}

static int test_mmap_beyond(void)
{
        /* <insert evil laughter here> */
#define MMAP_BEYOND_FILE "mmapbeyondtest"
#define BEYOND_STR "FOOBAR!"

        int fd;
        char *addr, *addr2;
        int status;

        printf("Testing mmap() beyond end of backing object\n");

        /* Set up test file */
        test_assert(-1 != (fd = open(MMAP_BEYOND_FILE, O_RDWR | O_CREAT, 0)), NULL);
        test_assert(8 == write(fd, BEYOND_STR, 8), NULL);
        test_assert(0 == unlink(MMAP_BEYOND_FILE), NULL);

        /* Set up test mmap */
        test_assert(MAP_FAILED != (addr = mmap(NULL, PAGE_SIZE * 10,
                                               PROT_READ | PROT_WRITE, MAP_SHARED, fd, 0)), NULL);
        /* make sure it's there */
        test_assert(!strcmp(addr, BEYOND_STR), NULL);

        /* Do it again, but with private mapping. */
        test_assert(MAP_FAILED != (addr2 = mmap(NULL, PAGE_SIZE * 10,
                                                PROT_READ | PROT_WRITE, MAP_PRIVATE, fd, 0)), NULL);
        /* make sure it's there */
        test_assert(!strcmp(addr2, BEYOND_STR), NULL);
        *addr2 = 'a';

        /* Can't go too far on either */
        assert_fault(char foo = *(addr + PAGE_SIZE), "");
        assert_fault(char foo = *(addr + PAGE_SIZE * 5), "");
        assert_fault(*((char *)addr + PAGE_SIZE * 5) = 'a', "");

        assert_fault(char foo = *(addr2 + PAGE_SIZE), "");
        assert_fault(char foo = *(addr2 + PAGE_SIZE * 5), "");
        assert_fault(*(addr2 + PAGE_SIZE * 5) = 'a', "");

        /* Write more to it */
        test_assert(PAGE_SIZE * 3 == lseek(fd, PAGE_SIZE * 3, SEEK_SET), NULL);
        test_assert(8 == write(fd, BEYOND_STR, 8), NULL);

        /* Can go up to new location */
        test_assert(!strcmp(addr, BEYOND_STR), NULL);
        test_assert('\0' == *(addr + PAGE_SIZE), NULL);
        test_assert('\0' == *(addr + PAGE_SIZE * 2), NULL);
        test_assert(!strcmp(addr + PAGE_SIZE * 3, BEYOND_STR), NULL);

        test_assert('a' == *addr2, NULL);
        test_assert('\0' == *(addr2 + PAGE_SIZE), NULL);
        test_assert('\0' == *(addr2 + PAGE_SIZE * 2), NULL);
        test_assert(!strcmp(addr2 + PAGE_SIZE * 3, BEYOND_STR), NULL);

        /* Can't go beyond it */
        assert_fault(char foo = *(addr + PAGE_SIZE * 4), "");
        assert_fault(char foo = *(addr + PAGE_SIZE * 8), "");
        assert_fault(*(addr + PAGE_SIZE * 5) = 'a', "");

        assert_fault(char foo = *(addr2 + PAGE_SIZE * 4), "");
        assert_fault(char foo = *(addr2 + PAGE_SIZE * 8), "");
        assert_fault(*(addr2 + PAGE_SIZE * 5) = 'a', "");

        return 0;
}

int main(int argc, char **argv)
{
        if (argc != 1) {
                fprintf(stderr,
                        "USAGE: memtest\n");
                return 1;
        }
        int status;

        /* Make sure we found out if anything segfaults that shouldn't */
#define childtest(fun) \
        do { \
                test_fork_begin() { \
                        return fun(); \
                } test_fork_end(&status); \
                test_assert(EFAULT != status, "Test process shouldn't segfault!"); \
                test_assert(0 == status, "Test process returned error"); \
        } while (0)

        /* printf("Linker magic: start 0x%p, text end 0x%p, data end 0x%p, bss end 0x%p\n",
               text_start, text_end, data_end, bss_end); */
        test_init();
        childtest(test_overflow);
        childtest(test_mmap_bounds);
        childtest(test_brk_bounds);
        childtest(test_munmap);
        childtest(test_start_brk);
        childtest(test_brk_mmap);
        childtest(test_mmap_fill);
        childtest(test_mmap_repeat);
        childtest(test_mmap_beyond);
        test_fini();

        return 0;
}
