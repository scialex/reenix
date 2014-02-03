#pragma once

#include "types.h"

/* Starts the Programmable Interval Timer (PIT)
 * delivering periodic interrupts at 1000 Hz
 * (i.e., one every millisecond) to the given interrupt. */
void pit_init(uint8_t intr);
void pit_starttimer(uint8_t intr);
