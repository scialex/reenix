/*
 *  File: smacros.h
 *  Date: 14 March 1998
 *  Acct: David Powell (dep)
 *  Desc: Some additional SPARC assembly macros
 */

#ifndef _smacros_h_
#define _smacros_h_

#ifdef  __cplusplus
extern "C" {
#endif


        /* Local entry points */

#define LENTRY(x)                       \
        .section        ".text";        \
        .align  4;                      \
        .type   x, #function;           \
x:

#define ALTLENTRY(x)                    \
        .type   x, #function;           \
x:


        /* This macro assumes you don't care what happens to %o7 */

#define GET_GOT(x)                                              \
        call    1f;                                             \
        sethi   %hi(_GLOBAL_OFFSET_TABLE_ + 4), x;              \
1:                                                              \
        or      x, %lo(_GLOBAL_OFFSET_TABLE_ + 8), x;           \
        add     %o7, x, x



#ifdef  __cplusplus
}
#endif

#endif /* _smacros_h_ */

