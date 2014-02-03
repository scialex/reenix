#pragma once

#include "sys/types.h"
#include "stddef.h"
#include "weenix/syscall.h"
#include "errno.h"

#define TRAP_INTR_STRING QUOTE(INTR_SYSCALL)

static inline int trap(uint32_t num, uint32_t arg)
{
        int ret;
        __asm__ volatile(
                "int $" TRAP_INTR_STRING
                : "=a"(ret)
                : "a"(num), "d"(arg)
        );
        /* Copy in errno */
        __asm__ volatile(
                "int $" TRAP_INTR_STRING
                : "=a"(errno)
                : "a"(SYS_errno)
        );
        return ret;
}
