#pragma once


#ifndef __KERNEL__
#include "unistd.h"
#include "sys/types.h"
#else
#include "types.h"
#endif

#include <stdarg.h>

#define test_assert(expr, fmt, args...) \
        _test_assert(expr, __FILE__, __LINE__, #expr, fmt, ## args)

#ifndef __KERNEL__
#define test_fork_begin()                       \
        do {                                    \
                pid_t __test_pid = fork();      \
                if (0 == __test_pid) {          \
                        do

#define test_fork_end(status)                   \
                        while (0);              \
                        exit(0);                \
                } /* if */                      \
                waitpid(__test_pid, 0, status); \
        } while (0);
#endif

void test_init(void);
void test_fini(void);

const char *test_errstr(int err);

typedef void (*test_pass_func_t)(int val, const char *file, int line, const char *name, const char *fmt, va_list args);
typedef void (*test_fail_func_t)(const char *file, int line, const char *name, const char *fmt, va_list args);

int _test_assert(int val, const char *file, int line, const char *name, const char *fmt, ...);
