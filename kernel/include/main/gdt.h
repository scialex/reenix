#pragma once

#include "types.h"

#define GDT_COUNT 16

#define GDT_ZERO        0x00
#define GDT_KERNEL_TEXT 0x08
#define GDT_KERNEL_DATA 0x10
#define GDT_USER_TEXT   0x18
#define GDT_USER_DATA   0x20
#define GDT_TSS         0x28

void gdt_init(void);

void gdt_set_kernel_stack(void *addr);

void gdt_set_entry(uint32_t segment, uint32_t base, uint32_t limit,
                   uint8_t ring, int exec, int dir, int rw);
void gdt_clear(uint32_t segment);
