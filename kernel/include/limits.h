#pragma once

#define CHAR_BIT  8
#define CHAR_MAX  UCHAR_MAX
#define UCHAR_MAX ((unsigned char)(~0U))
#define SCHAR_MAX ((signed char)(UCHAR_MAX>>1))
#define SCHAR_MIN (-SCHAR_MAX - 1)
#define USHRT_MAX ((unsigned short)(~0U))
#define SHRT_MAX  ((signed short)(USHRT_MAX>>1))
#define SHRT_MIN  (-SHRT_MAX - 1)
#define UINT_MAX  ((unsigned int)(~0U))
#define INT_MAX   ((signed int)(UINT_MAX>>1))
#define INT_MIN   (-INT_MAX - 1)
#define ULONG_MAX ((unsigned long)(~0UL))
#define LONG_MAX  ((signed long)(ULONG_MAX>>1))
#define LONG_MIN  (-LONG_MAX - 1)

#define UPTR_MAX  UINT_MAX
