#pragma once

#define init_func(func)                         \
        __asm__ (                               \
                ".pushsection .init\n\t"        \
                ".long " #func "\n\t"           \
                ".string \"" #func "\"\n\t"     \
                ".popsection\n\t"               \
        );
#define init_depends(name)                      \
        __asm__ (                               \
                ".pushsection .init\n\t"        \
                ".long 0\n\t"                   \
                ".string \"" #name "\"\n\t"     \
                ".popsection\n\t"               \
        );

typedef void (*init_func_t)();

void init_call_all(void);
