#pragma once

#include "types.h"

/* The linker script will initialize these symbols. Note
 * that the linker does not actually allocate any space
 * for these variables (thus the void type) it only sets
 * the address that the symbol points to. So for example
 * the address where the kernel ends is &kernel_end,
 * NOT kernel_end.
 */
extern void *kernel_start;
extern void *kernel_start_text;
extern void *kernel_start_data;
extern void *kernel_start_bss;
extern void *kernel_end;
extern void *kernel_end_text;
extern void *kernel_end_data;
extern void *kernel_end_bss;
extern void *kernel_start_init;
extern void *kernel_end_init;

#define inline __attribute__ ((always_inline,used))
#define unlikely(x) __builtin_expect((x), 0)
#define likely(x) __builtin_expect((x), 1)

#define offsetof(type, member) \
        ((uint32_t)((char*)&((type *)(0))->member - (char*)0))

#define NOT_YET_IMPLEMENTED(f) \
        (dbg_print("Not yet implemented: %s, file %s, line %d\n", \
                   f, __FILE__, __LINE__))

#ifndef MIN
#define MIN(a,b)  ((a) < (b) ? (a) : (b))
#endif
#ifndef MAX
#define MAX(a,b)  ((a) > (b) ? (a) : (b))
#endif

#define CONTAINER_OF(obj, type, member) \
        ((type *)((char *)(obj) - offsetof(type, member)))

/* This truly atrocious macro hack taken from the wikipedia article on the C
 * preprocessor, use to "quote" the value (or name) of another macro:
 * QUOTE_BY_NAME(NTERMS) -> "NTERMS"
 * QUOTE(NTERMS) -> "3"
 *
 * These macros even made more atrocious by searching for "stringizing operator
 * comma".  The variable length macros account for comma separated symbols.
 */
#define QUOTE_BY_NAME(...) #__VA_ARGS__
#define QUOTE_BY_VALUE(x) QUOTE_BY_NAME(x)
/* By default, we quote by value */
#define QUOTE(...) QUOTE_BY_NAME(__VA_ARGS__)

