/*
 ****************************************************************************
 * (C) 2003 - Rolf Neugebauer - Intel Research Cambridge
 ****************************************************************************
 *
 *        File: printf.c
 *      Author: Rolf Neugebauer (neugebar@dcs.gla.ac.uk)
 *     Changes: Grzegorz Milos (gm281@cam.ac.uk)
 *
 *        Date: Aug 2003, Aug 2005
 *
 * Environment: Xen Minimal OS
 * Description: Library functions for printing
 *              (freebsd port, mainly sys/subr_prf.c)
 *
 ****************************************************************************
 *
 *-
 * Copyright (c) 1992, 1993
 *      The Regents of the University of California.  All rights reserved.
 *
 * This software was developed by the Computer Systems Engineering group
 * at Lawrence Berkeley Laboratory under DARPA contract BG 91-66 and
 * contributed to Berkeley.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 * 3. All advertising materials mentioning features or use of this software
 *    must display the following acknowledgement:
 *      This product includes software developed by the University of
 *      California, Berkeley and its contributors.
 * 4. Neither the name of the University nor the names of its contributors
 *    may be used to endorse or promote products derived from this software
 *    without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE REGENTS AND CONTRIBUTORS ``AS IS'' AND
 * ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
 * ARE DISCLAIMED.  IN NO EVENT SHALL THE REGENTS OR CONTRIBUTORS BE LIABLE
 * FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
 * OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
 * HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
 * LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
 * OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
 * SUCH DAMAGE.
 *
 * $FreeBSD: src/sys/libkern/divdi3.c,v 1.6 1999/08/28 00:46:31 peter Exp $
 */

#include "stdio.h"
#include "unistd.h"

int printf(const char *fmt, ...)
{
        va_list args;
        int i;

        va_start(args, fmt);
        i = vprintf(fmt, args);
        va_end(args);
        return i;
}

int fprintf(FILE *stream, const char *fmt, ...)
{
        va_list args;
        int i;

        va_start(args, fmt);
        i = vfprintf(stream, fmt, args);
        va_end(args);
        return i;
}

int sprintf(char *buf, const char *fmt, ...)
{
        va_list args;
        int i;

        va_start(args, fmt);
        i = vsprintf(buf, fmt, args);
        va_end(args);
        return i;
}

int snprintf(char *buf, size_t size, const char *fmt, ...)
{
        va_list args;
        int i;

        va_start(args, fmt);
        i = vsnprintf(buf, size, fmt, args);
        va_end(args);
        return i;
}

int vprintf(const char *fmt, va_list args)
{
        return vfprintf(stdout, fmt, args);
}

#define __LIBC_PRINTF_BUFSIZE 1024
int vfprintf(FILE *stream, const char *fmt, va_list args)
{
        /* I'm really lazy */
        char buf[__LIBC_PRINTF_BUFSIZE];
        int ret = vsnprintf(buf, __LIBC_PRINTF_BUFSIZE, fmt, args);
        if (ret > 0) {
                write(*stream, buf, ret);
        }
        return ret;
}

int vsprintf(char *buf, const char *fmt, va_list args)
{
        return vsnprintf(buf, 0xffffffffUL, fmt, args);
}

int fflush(FILE *stream)
{
        /* no-op */
        return 0;
}
