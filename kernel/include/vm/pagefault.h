#pragma once

#include "types.h"

#define FAULT_PRESENT  0x01
#define FAULT_WRITE    0x02
#define FAULT_USER     0x04
#define FAULT_RESERVED 0x08
#define FAULT_EXEC     0x10

void handle_pagefault(uintptr_t vaddr, uint32_t cause);
