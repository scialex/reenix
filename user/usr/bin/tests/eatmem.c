/*
 * Eats kernel memory. Lots of it.
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

/* TODO ensure this matches the kernel value */
#define PAGE_SIZE 4096

static void eat(void *addr, int *count, int *num)
{
        int status;
        test_fork_begin() {
                if (*num <= 0) {
                        /* Eat the memory until we die */
                        while (1) {
                                char foo = *((char *)addr + ((*count)++ * PAGE_SIZE));
                                if ((*count & 0x7f) == 0)
                                        printf("Ate %d pages\n", *count);
                        }
                } else {
                        /* Eat until we have the necessary number of pages */
                        while (*count < *num) {
                                char foo = *((char *)addr + ((*count)++ * PAGE_SIZE));
                                if ((*count & 0x7f) == 0)
                                        printf("Ate %d pages\n", *count);
                        }
                }
        } test_fork_end(&status);
        if (*num <= 0 && EFAULT != status) {
                fprintf(stderr, "Child process didn't segfault!\n");
                exit(1);
        }
        if (*num < 0) {
                /* Free the required number of pages */
                munmap(addr, PAGE_SIZE * (-*num));
                printf("Gave back %d pages\n", -*num);
                *count += *num;
        }
}

#define FLAG_DAEMON   "-d"
#define FLAG_INFINITE "-i"
#define FLAG_ITER     "-y"
#define FLAG_NUM      "-#"

#define OPT_DAEMON    1
#define OPT_INFINITE  2
#define OPT_ITER       4
#define OPT_NUM    8

int parse_args(int argc, char **argv, int *opts, int *iter, int *num)
{
        int i;
        *opts = *iter = *num = 0;
        for (i = 1; i < argc; i++) {
                if (!strcmp(FLAG_DAEMON, argv[i])) {
                        *opts |= OPT_DAEMON;
                } else if (!strcmp(FLAG_INFINITE, argv[i])) {
                        *opts |= OPT_INFINITE;
                } else if (!strcmp(FLAG_ITER, argv[i])) {
                        *opts |= OPT_ITER;
                        if (++i >= argc || (errno = 0,
                                            *iter = strtol(argv[i], NULL, 0),
                                            0 != errno)) {
                                return -1;
                        }
                } else if (!strcmp(FLAG_NUM, argv[i])) {
                        *opts |= OPT_NUM;
                        if (++i >= argc || (errno = 0,
                                            *num = strtol(argv[i], NULL, 0),
                                            0 != errno)) {
                                return -1;
                        }
                } else {
                        return -1;
                }
        }
        return 0;
}

int main(int argc, char **argv)
{
        int status;
        void *addr;
        int *count;
        int *opts, *iter, *num;


        /* Get our huge mess of space. We map this as a bunch of regions at the
         * beginning so that unmapping (above) actually does something. */
        if (MAP_FAILED == (addr = mmap(NULL, PAGE_SIZE * 10000,
                                       PROT_READ | PROT_WRITE, MAP_SHARED | MAP_ANON, -1, 0)))
                return 1;

        int i;
        for (i = 0; i < 40; i++) {
                if (MAP_FAILED == mmap((char *)addr + PAGE_SIZE * 25 * i, PAGE_SIZE * 25,
                                       PROT_READ | PROT_WRITE, MAP_FIXED | MAP_SHARED | MAP_ANON, -1, 0))
                        return 1;
        }


        /* Set the initial count */
        count = addr;
        *count = 0;
        opts = count + 1;
        iter = count + 2;
        num = count + 3;

        if (0 > parse_args(argc, argv, opts, iter, num)) {
                fprintf(stderr,
                        "USAGE: eatmem [options]\n"
                        FLAG_DAEMON   "          run as daemon\n"
                        FLAG_INFINITE "          run forever\n"
                        FLAG_ITER     " [num]    number of iterations to yield\n"
                        FLAG_NUM      " [num]    number of pages to eat (if negative, to relinquish)\n");
                return 1;
        }

        addr = (char *)addr + PAGE_SIZE;

        printf("OM NOM NOM NOM\n");

        if (*opts & OPT_DAEMON) {
                if (fork())
                        exit(0);
        }

        eat(addr, count, num);

        printf("Ate %d pages in total\n", *count);

        if (*opts & OPT_INFINITE) {
                while (1) {
                        yield();
                }
        } else if (*opts & OPT_ITER) {
                while (--iter) {
                        yield();
                }
        }
        printf("Giving memory back now\n");
        return 0;
}
