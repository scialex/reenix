#pragma once

#include "lseek.h"
#include "stddef.h"
#include "stdarg.h"
#include "sys/types.h"

/* Output not buffered */
#define __IONBF 1

#ifndef EOF
#define EOF     (-1)
#endif

#ifndef NULL
#define NULL    0
#endif

/* For now, just store a file descriptor */
typedef int FILE;
typedef off_t fpos_t;
extern FILE *stdin;
extern FILE *stdout;
extern FILE *stderr;

/* ANSI C89 */
int printf(const char *fmt, ...)
        __attribute__((__format__(printf, 1, 2)))
        __attribute__((__nonnull__(1)));
int fprintf(FILE *stream, const char *fmt, ...)
        __attribute__((__format__(printf, 2, 3)))
        __attribute__((__nonnull__(2)));
int sprintf(char *buf, const char *fmt, ...)
        __attribute__((__format__(printf, 2, 3)))
        __attribute__((__nonnull__(2)));

int fflush(FILE *stream);

int vprintf(const char *fmt, va_list args)
        __attribute__((__format__(printf, 1, 0)))
        __attribute__((__nonnull__(1)));
int vfprintf(FILE *stream, const char *fmt, va_list args)
        __attribute__((__format__(printf, 2, 0)))
        __attribute__((__nonnull__(2)));
int vsprintf(char *buf, const char *fmt, va_list args)
        __attribute__((__format__(printf, 2, 0)))
        __attribute__((__nonnull__(2)));

/* Other */
int snprintf(char *buf, size_t size, const char *fmt, ...)
        __attribute__((__format__(printf, 3, 4)))
        __attribute__((__nonnull__(3)));
int vsnprintf(char *buf, size_t size, const char *fmt, va_list args)
        __attribute__((__format__(printf, 3, 0)))
        __attribute__((__nonnull__(3)));

int sscanf(const char *buf, const char *fmt, ...)
        __attribute__((__format__(scanf, 2, 3)))
        __attribute__((__nonnull__(2)));
int vsscanf(const char *buf, const char *fmt, va_list args)
        __attribute__((__nonnull__(2)));
