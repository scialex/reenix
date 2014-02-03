#pragma once

#include "sys/types.h"

#define inline __attribute__ ((always_inline,used))
#define unlikely(x) __builtin_expect((x), 0)
#define likely(x) __builtin_expect((x), 1)

#define offsetof(type, member) \
        ((uint32_t)((char*)&((type *)(0))->member - (char*)0))

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
 */
#define QUOTE_BY_NAME(x) #x
#define QUOTE_BY_VALUE(x) QUOTE_BY_NAME(x)
/* By default, we quote by value */
#define QUOTE(x) QUOTE_BY_NAME(x)
