/*
 *  File: ldalloc.c
 *  Date: 28 March 1998
 *  Acct: David Powell (dep)
 *  Desc: simple allocation routines
 */

#include "ldutil.h"
#include "ldalloc.h"
#include <stdio.h>
#include <unistd.h>
#include <stdlib.h>
#include <sys/mman.h>


static unsigned long start;
static unsigned long pos;
static unsigned long amount;


/* This function initializes the simple memory allocator.  We basically
 * allocate a specified number of pages to use as scratch memory for
 * the linker itself.  No deallocation functionality is provided; the
 * amount of memory used should be small, and is usually needed for the
 * duration of the program's execution, anyway.
 *
 * All this function does is mmap the specified number of pages of
 * /dev/zero to provide the memory for our little memory-wading-pool. */

void _ldainit(unsigned long pagesize, unsigned long pages)
{
        amount = pagesize * pages;
        pos = 0;

        start = (unsigned long)mmap(NULL, amount, PROT_READ | PROT_WRITE,
                                    MAP_PRIVATE | MAP_ANON, -1, 0);
        if (start == (unsigned long) MAP_FAILED) {
                fprintf(stderr, "ld-weenix: panic - unable to map /dev/zero\n");
                exit(1);
        }
}


/* This function allocates a block of memory of the specified size from
 * our memory pool.  The memory is word-aligned, and cannot be freed. */

void *_ldalloc(unsigned long size)
{
        unsigned long   next;

        if (size & 3) {
                size = (size&~3) + 4;
        }

        if (pos + size > amount) {
                fprintf(stderr, "ld.so.1: panic - unable to allocate %lu bytes (_ldalloc)\n", size);
                exit(1);
        }

        next = start + pos;
        pos += size;

        return (void *)next;
}

