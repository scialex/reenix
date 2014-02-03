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
#include "limits.h"
#include "stdarg.h"
#include "stddef.h"
#include "ctype.h"

static int skip_atoi(const char **s)
{
        int i = 0;

        while (isdigit(**s))
                i = i * 10 + *((*s)++) - '0';
        return i;
}

/**
 * simple_strtoul - convert a string to an unsigned long
 * @cp: The start of the string
 * @endp: A pointer to the end of the parsed string will be placed here
 * @base: The number base to use
 */
unsigned long simple_strtoul(const char *cp, char **endp, unsigned int base)
{
        unsigned long result = 0, value;

        if (!base) {
                base = 10;
                if (*cp == '0') {
                        base = 8;
                        cp++;
                        if ((*cp == 'x') && isxdigit(cp[1])) {
                                cp++;
                                base = 16;
                        }
                }
        }
        while (isxdigit(*cp) &&
               (value = isdigit(*cp) ? *cp - '0' : toupper(*cp) - 'A' + 10) < base) {
                result = result * base + value;
                cp++;
        }
        if (endp)
                *endp = (char *)cp;
        return result;
}

/**
 * simple_strtol - convert a string to a signed long
 * @cp: The start of the string
 * @endp: A pointer to the end of the parsed string will be placed here
 * @base: The number base to use
 */
long simple_strtol(const char *cp, char **endp, unsigned int base)
{
        if (*cp == '-')
                return -simple_strtoul(cp + 1, endp, base);
        return simple_strtoul(cp, endp, base);
}

/**
 * simple_strtoull - convert a string to an unsigned long long
 * @cp: The start of the string
 * @endp: A pointer to the end of the parsed string will be placed here
 * @base: The number base to use
 */
unsigned long long simple_strtoull(const char *cp, char **endp, unsigned int base)
{
        unsigned long long result = 0, value;

        if (!base) {
                base = 10;
                if (*cp == '0') {
                        base = 8;
                        cp++;
                        if ((*cp == 'x') && isxdigit(cp[1])) {
                                cp++;
                                base = 16;
                        }
                }
        }
        while (isxdigit(*cp) && (value = isdigit(*cp) ? *cp - '0' : (islower(*cp)
                                         ? toupper(*cp) : *cp) - 'A' + 10) < base) {
                result = result * base + value;
                cp++;
        }
        if (endp)
                *endp = (char *)cp;
        return result;
}

/**
 * simple_strtoll - convert a string to a signed long long
 * @cp: The start of the string
 * @endp: A pointer to the end of the parsed string will be placed here
 * @base: The number base to use
 */
long long simple_strtoll(const char *cp, char **endp, unsigned int base)
{
        if (*cp == '-')
                return -simple_strtoull(cp + 1, endp, base);
        return simple_strtoull(cp, endp, base);
}
/**
 * vsscanf - Unformat a buffer into a list of arguments
 * @buf:        input buffer
 * @fmt:        format of buffer
 * @args:       arguments
 */
int vsscanf(const char *buf, const char *fmt, va_list args)
{
        const char *str = buf;
        char *next;
        char digit;
        int num = 0;
        int qualifier;
        int base;
        int field_width;
        int is_sign = 0;

        while (*fmt && *str) {
                /* skip any white space in format */
                /* white space in format matchs any amount of
                 * white space, including none, in the input.
                 */
                if (isspace(*fmt)) {
                        while (isspace(*fmt))
                                ++fmt;
                        while (isspace(*str))
                                ++str;
                }

                /* anything that is not a conversion must match exactly */
                if (*fmt != '%' && *fmt) {
                        if (*fmt++ != *str++)
                                break;
                        continue;
                }

                if (!*fmt)
                        break;
                ++fmt;

                /* skip this conversion.
                 * advance both strings to next white space
                 */
                if (*fmt == '*') {
                        while (!isspace(*fmt) && *fmt)
                                fmt++;
                        while (!isspace(*str) && *str)
                                str++;
                        continue;
                }

                /* get field width */
                field_width = -1;
                if (isdigit(*fmt))
                        field_width = skip_atoi(&fmt);

                /* get conversion qualifier */
                qualifier = -1;
                if (*fmt == 'h' || *fmt == 'l' || *fmt == 'L' ||
                    *fmt == 'Z' || *fmt == 'z') {
                        qualifier = *fmt++;
                        if (unlikely(qualifier == *fmt)) {
                                if (qualifier == 'h') {
                                        qualifier = 'H';
                                        fmt++;
                                } else if (qualifier == 'l') {
                                        qualifier = 'L';
                                        fmt++;
                                }
                        }
                }
                base = 10;
                is_sign = 0;

                if (!*fmt || !*str)
                        break;

                switch (*fmt++) {
                        case 'c': {
                                char *s = (char *) va_arg(args, char *);
                                if (field_width == -1)
                                        field_width = 1;
                                do {
                                        *s++ = *str++;
                                } while (--field_width > 0 && *str);
                                num++;
                        }
                        continue;
                        case 's': {
                                char *s = (char *) va_arg(args, char *);
                                if (field_width == -1)
                                        field_width = INT_MAX;
                                /* first, skip leading white space in buffer */
                                while (isspace(*str))
                                        str++;

                                /* now copy until next white space */
                                while (*str && !isspace(*str) && field_width--) {
                                        *s++ = *str++;
                                }
                                *s = '\0';
                                num++;
                        }
                        continue;
                        case 'n':
                                /* return number of characters read so far */
                        {
                                int *i = (int *)va_arg(args, int *);
                                *i = str - buf;
                        }
                        continue;
                        case 'o':
                                base = 8;
                                break;
                        case 'x':
                        case 'X':
                                base = 16;
                                break;
                        case 'i':
                                base = 0;
                        case 'd':
                                is_sign = 1;
                        case 'u':
                                break;
                        case '%':
                                /* looking for '%' in str */
                                if (*str++ != '%')
                                        return num;
                                continue;
                        default:
                                /* invalid format; stop here */
                                return num;
                }

                /* have some sort of integer conversion.
                 * first, skip white space in buffer.
                 */
                while (isspace(*str))
                        str++;

                digit = *str;
                if (is_sign && digit == '-')
                        digit = *(str + 1);

                if (!digit
                    || (base == 16 && !isxdigit(digit))
                    || (base == 10 && !isdigit(digit))
                    || (base == 8 && (!isdigit(digit) || digit > '7'))
                    || (base == 0 && !isdigit(digit)))
                        break;

                switch (qualifier) {
                        case 'H':       /* that's 'hh' in format */
                                if (is_sign) {
                                        signed char *s = (signed char *) va_arg(args, signed char *);
                                        *s = (signed char) simple_strtol(str, &next, base);
                                } else {
                                        unsigned char *s = (unsigned char *) va_arg(args, unsigned char *);
                                        *s = (unsigned char) simple_strtoul(str, &next, base);
                                }
                                break;
                        case 'h':
                                if (is_sign) {
                                        short *s = (short *) va_arg(args, short *);
                                        *s = (short) simple_strtol(str, &next, base);
                                } else {
                                        unsigned short *s = (unsigned short *) va_arg(args, unsigned short *);
                                        *s = (unsigned short) simple_strtoul(str, &next, base);
                                }
                                break;
                        case 'l':
                                if (is_sign) {
                                        long *l = (long *) va_arg(args, long *);
                                        *l = simple_strtol(str, &next, base);
                                } else {
                                        unsigned long *l = (unsigned long *) va_arg(args, unsigned long *);
                                        *l = simple_strtoul(str, &next, base);
                                }
                                break;
                        case 'L':
                                if (is_sign) {
                                        long long *l = (long long *) va_arg(args, long long *);
                                        *l = simple_strtoll(str, &next, base);
                                } else {
                                        unsigned long long *l = (unsigned long long *) va_arg(args, unsigned long long *);
                                        *l = simple_strtoull(str, &next, base);
                                }
                                break;
                        case 'Z':
                        case 'z': {
                                size_t *s = (size_t *) va_arg(args, size_t *);
                                *s = (size_t) simple_strtoul(str, &next, base);
                        }
                        break;
                        default:
                                if (is_sign) {
                                        int *i = (int *) va_arg(args, int *);
                                        *i = (int) simple_strtol(str, &next, base);
                                } else {
                                        unsigned int *i = (unsigned int *) va_arg(args, unsigned int *);
                                        *i = (unsigned int) simple_strtoul(str, &next, base);
                                }
                                break;
                }
                num++;

                if (!next)
                        break;
                str = next;
        }
        return num;
}


