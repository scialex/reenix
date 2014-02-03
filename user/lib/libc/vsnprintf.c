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

#include "ctype.h"
#include "sys/types.h"
#include "stdarg.h"
#include "stddef.h"
#include "stdio.h"
#include "string.h"

static int skip_atoi(const char **s)
{
        int i = 0;

        while (isdigit(**s))
                i = i * 10 + *((*s)++) - '0';
        return i;
}

#define ZEROPAD 1               /* pad with zero */
#define SIGN    2               /* unsigned/signed long */
#define PLUS    4               /* show plus */
#define SPACE   8               /* space if plus */
#define LEFT    16              /* left justified */
#define SPECIAL 32              /* 0x */
#define LARGE   64              /* use 'ABCDEF' instead of 'abcdef' */

static char *number(char *buf, char *end, long long num, int base, int size, int precision, int type)
{
        char c, sign, tmp[66];
        const char *digits;
        const char small_digits[] = "0123456789abcdefghijklmnopqrstuvwxyz";
        const char large_digits[] = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";
        int i;

        digits = (type & LARGE) ? large_digits : small_digits;
        if (type & LEFT)
                type &= ~ZEROPAD;
        if (base < 2 || base > 36)
                return buf;
        c = (type & ZEROPAD) ? '0' : ' ';
        sign = 0;
        if (type & SIGN) {
                if (num < 0) {
                        sign = '-';
                        num = -num;
                        size--;
                } else if (type & PLUS) {
                        sign = '+';
                        size--;
                } else if (type & SPACE) {
                        sign = ' ';
                        size--;
                }
        }
        if (type & SPECIAL) {
                if (base == 16)
                        size -= 2;
                else if (base == 8)
                        size--;
        }
        i = 0;
        if (num == 0) {
                tmp[i++] = '0';
        } else {
                /* XXX KAF: force unsigned mod and div. */
                /* XXX kernel does not support long long division */
                unsigned long long num2 = (unsigned long long)num;
                unsigned int base2 = (unsigned int)base;
                while (num2 != 0) {
                        tmp[i++] = digits[num2 % base2];
                        num2 /= base2;
                }
        }
        if (i > precision)
                precision = i;
        size -= precision;
        if (!(type & (ZEROPAD + LEFT))) {
                while (size-- > 0) {
                        if (buf <= end)
                                *buf = ' ';
                        ++buf;
                }
        }
        if (sign) {
                if (buf <= end)
                        *buf = sign;
                ++buf;
        }
        if (type & SPECIAL) {
                if (base == 8) {
                        if (buf <= end)
                                *buf = '0';
                        ++buf;
                } else if (base == 16) {
                        if (buf <= end)
                                *buf = '0';
                        ++buf;
                        if (buf <= end)
                                *buf = digits[33];
                        ++buf;
                }
        }
        if (!(type & LEFT)) {
                while (size-- > 0) {
                        if (buf <= end)
                                *buf = c;
                        ++buf;
                }
        }
        while (i < precision--) {
                if (buf <= end)
                        *buf = '0';
                ++buf;
        }
        while (i-- > 0) {
                if (buf <= end)
                        *buf = tmp[i];
                ++buf;
        }
        while (size-- > 0) {
                if (buf <= end)
                        *buf = ' ';
                ++buf;
        }
        return buf;
}


/* TODO This is a copy of the kernel vsnprintf. It has (almost)
 * no error checking. It also is completely unable to handle
 * floating point. - alvin */
/**
* vsnprintf - Format a string and place it in a buffer
* @buf: The buffer to place the result into
* @size: The size of the buffer, including the trailing null space
* @fmt: The format string to use
* @args: Arguments for the format string
*
* Call this function if you are already dealing with a va_list.
* You probably want snprintf instead.
 */
int vsnprintf(char *buf, size_t size, const char *fmt, va_list args)
{
        int len;
        unsigned long long num;
        int i, base;
        char *str, *end, c;
        const char *s;

        int flags;          /* flags to number() */

        int field_width;    /* width of output field */
        int precision;              /* min. # of digits for integers; max
                                   number of chars for from string */
        int qualifier;              /* 'h', 'l', or 'L' for integer fields */
        /* 'z' support added 23/7/1999 S.H.    */
        /* 'z' changed to 'Z' --davidm 1/25/99 */

        str = buf;
        end = buf + size - 1;

        if (end < buf - 1) {
                end = ((void *) - 1);
                size = end - buf + 1;
        }

        for (; *fmt ; ++fmt) {
                if (*fmt != '%') {
                        if (str <= end)
                                *str = *fmt;
                        ++str;
                        continue;
                }

                /* process flags */
                flags = 0;
repeat:
                ++fmt;          /* this also skips first '%' */
                switch (*fmt) {
                        case '-': flags |= LEFT; goto repeat;
                        case '+': flags |= PLUS; goto repeat;
                        case ' ': flags |= SPACE; goto repeat;
                        case '#': flags |= SPECIAL; goto repeat;
                        case '0': flags |= ZEROPAD; goto repeat;
                }

                /* get field width */
                field_width = -1;
                if (isdigit(*fmt))
                        field_width = skip_atoi(&fmt);
                else if (*fmt == '*') {
                        ++fmt;
                        /* it's the next argument */
                        field_width = va_arg(args, int);
                        if (field_width < 0) {
                                field_width = -field_width;
                                flags |= LEFT;
                        }
                }

                /* get the precision */
                precision = -1;
                if (*fmt == '.') {
                        ++fmt;
                        if (isdigit(*fmt))
                                precision = skip_atoi(&fmt);
                        else if (*fmt == '*') {
                                ++fmt;
                                /* it's the next argument */
                                precision = va_arg(args, int);
                        }
                        if (precision < 0)
                                precision = 0;
                }

                /* get the conversion qualifier */
                qualifier = -1;
                if (*fmt == 'h' || *fmt == 'l' || *fmt == 'L' || *fmt == 'Z') {
                        qualifier = *fmt;
                        ++fmt;
                        if (qualifier == 'l' && *fmt == 'l') {
                                qualifier = 'L';
                                ++fmt;
                        }
                }
                if (*fmt == 'q') {
                        qualifier = 'L';
                        ++fmt;
                }

                /* default base */
                base = 10;

                switch (*fmt) {
                        case 'c':
                                if (!(flags & LEFT)) {
                                        while (--field_width > 0) {
                                                if (str <= end)
                                                        *str = ' ';
                                                ++str;
                                        }
                                }
                                c = (unsigned char) va_arg(args, int);
                                if (str <= end)
                                        *str = c;
                                ++str;
                                while (--field_width > 0) {
                                        if (str <= end)
                                                *str = ' ';
                                        ++str;
                                }
                                continue;

                        case 's':
                                s = va_arg(args, char *);
                                if (!s)
                                        s = "<NULL>";

                                len = strnlen(s, precision);

                                if (!(flags & LEFT)) {
                                        while (len < field_width--) {
                                                if (str <= end)
                                                        *str = ' ';
                                                ++str;
                                        }
                                }
                                for (i = 0; i < len; ++i) {
                                        if (str <= end)
                                                *str = *s;
                                        ++str; ++s;
                                }
                                while (len < field_width--) {
                                        if (str <= end)
                                                *str = ' ';
                                        ++str;
                                }
                                continue;

                        case 'p':
                                if (field_width == -1) {
                                        field_width = 2 * sizeof(void *);
                                        flags |= ZEROPAD;
                                }
                                str = number(str, end,
                                             (unsigned long) va_arg(args, void *),
                                             16, field_width, precision, flags);
                                continue;


                        case 'n':
                                /* FIXME:
                                 * What does C99 say about the overflow case here? */
                                if (qualifier == 'l') {
                                        long *ip = va_arg(args, long *);
                                        *ip = (str - buf);
                                } else if (qualifier == 'Z') {
                                        size_t *ip = va_arg(args, size_t *);
                                        *ip = (str - buf);
                                } else {
                                        int *ip = va_arg(args, int *);
                                        *ip = (str - buf);
                                }
                                continue;

                        case '%':
                                if (str <= end)
                                        *str = '%';
                                ++str;
                                continue;

                                /* integer number formats - set up the flags and "break" */
                        case 'o':
                                base = 8;
                                break;

                        case 'X':
                                flags |= LARGE;
                        case 'x':
                                base = 16;
                                break;

                        case 'd':
                        case 'i':
                                flags |= SIGN;
                        case 'u':
                                break;
                                /* Added - TODO - alvin */
                        case 'f':
                        case 'F':
                        case 'g':
                        case 'G':
                                return -1;


                        default:
                                if (str <= end)
                                        *str = '%';
                                ++str;
                                if (*fmt) {
                                        if (str <= end)
                                                *str = *fmt;
                                        ++str;
                                } else {
                                        --fmt;
                                }
                                continue;
                }
                if (qualifier == 'L')
                        num = va_arg(args, long long);
                else if (qualifier == 'l') {
                        num = va_arg(args, unsigned long);
                        if (flags & SIGN)
                                num = (signed long) num;
                } else if (qualifier == 'Z') {
                        num = va_arg(args, size_t);
                } else if (qualifier == 'h') {
                        num = (unsigned short) va_arg(args, int);
                        if (flags & SIGN)
                                num = (signed short) num;
                } else {
                        num = va_arg(args, unsigned int);
                        if (flags & SIGN)
                                num = (signed int) num;
                }

                str = number(str, end, num, base,
                             field_width, precision, flags);
        }
        if (str <= end)
                *str = '\0';
        else if (size > 0)
                /* don't write out a null byte if the buf size is zero */
                *end = '\0';
        /* the trailing null byte doesn't count towards the total
         * ++str;
         */
        return str - buf;
}

