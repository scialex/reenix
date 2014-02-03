#pragma once

#include "types.h"
#include "kernel.h"

#define BIT(n) (1<<(n))

static inline void
bit_flip(void *addr, uintptr_t bit)
{
        uint32_t *map = (uint32_t *)addr;
        map += (bit >> 5);
        *map ^= (uint32_t)(1 << (bit & 0x1f));
}

static inline int
bit_check(const void *addr, uintptr_t bit)
{
        const uint32_t *map = (const uint32_t *)addr;
        map += (bit >> 5);
        return (*map & (1 << (bit & 0x1f)));
}

