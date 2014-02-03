#pragma once

/* Returns the highest physical address of the range of usable
 * that start at kernel_start. The intention is that this will
 * be the largest available continuous range of physical
 * addresses. This function should only be used during booting
 * while the first megabyte of memory is identity mapped,
 * otherwise its behavior is undefined. */
uintptr_t phys_detect_highmem();
