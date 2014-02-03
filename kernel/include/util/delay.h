#pragma once
#include "types.h"
#include "util/debug.h"

/* Approximate numbers taken from various points in Linux kernel */
#define LOOPS_PER_JIFFY (1 << 12)
#define HZ 100 /* Found this in a random place in the kernel */


/* From arch/x86/lib/delay.c in Linux kernel */
/*
 *      Precise Delay Loops for i386
 *
 *      Copyright (C) 1993 Linus Torvalds
 *      Copyright (C) 1997 Martin Mares <mj@atrey.karlin.mff.cuni.cz>
 *      Copyright (C) 2008 Jiri Hladky <hladky _dot_ jiri _at_ gmail _dot_ com>
 *
 *      The __delay function must _NOT_ be inlined as its execution time
 *      depends wildly on alignment on many x86 processors. The additional
 *      jump magic is needed to get the timing stable on all the CPU's
 *      we have to worry about.
 */

static void __delay(unsigned long loops)
{
        __asm__ volatile(
                "	test %0,%0	\n"
                "	jz 3f		\n"
                "	jmp 1f		\n"

                ".align 16		\n"
                "1:	jmp 2f		\n"

                ".align 16		\n"
                "2:	dec %0		\n"
                "	jnz 2b		\n"
                "3:	dec %0		\n"

                : /* we don't need output */
                :"a"(loops)
        );

}

static inline void __const_udelay(unsigned long xloops)
{
        int d0;

        xloops *= 4;
        __asm__ volatile(
                "mull %%edx"
                :"=d"(xloops), "=&a"(d0)
                :"1"(xloops), "0"
                (LOOPS_PER_JIFFY * (HZ/4))
        );

        __delay(++xloops);

}

static inline void __udelay(unsigned long usecs)
{
        __const_udelay(usecs * 4295);   /* 2**32 / 1000000 */
}

static inline void __ndelay(unsigned long nsecs)
{
        __const_udelay(nsecs * 5);      /* 2**32 / 1000000000 */
}

#define udelay(n) (__builtin_constant_p(n) ? \
                   ((n) > 20000 ? panic("Delay too large!") : __const_udelay((n) * 4295)) : \
                           __udelay(n))

#define ndelay(n) (__builtin_constant_p(n) ? \
                   ((n) > 20000 ? panic("Delay too large!") : __const_udelay((n) * 5)) : \
                           __ndelay(n))
