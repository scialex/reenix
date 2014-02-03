#pragma once

#include "types.h"

/*
 * A Weenix "device identifier" is the concatenation of:
 *     - a "driver number" or "device type" (major number)
 *     - a "device number" (minor number)
 *
 * The device identifiers for block devices and character devices are
 * independent. That is, you could have both a block device and a char device
 * with major 3, minor 5 (for example). They would be distinct.
 *
 * Weenix's device number allocation/assignment scheme is as follows:
 *
 *     - major 0 (byte or block), minor 0: reserved as an analogue of NULL
 *       for device id's
 *
 *     - char major 1:         Memory devices (mem)
 *         - minor 0:          /dev/null       The null device
 *         - minor 1:          /dev/zero       The zero device
 *
 *     - char major 2:         TTY devices (tty)
 *         - minor 0:          /dev/tty0       First TTY device
 *         - minor 1:          /dev/tty1       Second TTY device
 *         - and so on...
 *
 *     - block major 1:        Disk devices
 *         - minor 0:          first disk device
 *         - minor 1:          second disk device
 *         - and so on...
 */

#define MINOR_BITS              8
#define MINOR_MASK              ((1U << MINOR_BITS) - 1)
#define MAJOR(devid)            ((unsigned) ((devid) >> MINOR_BITS))
#define MINOR(devid)            ((unsigned) ((devid) & MINOR_MASK))
#define MKDEVID(major, minor)   (((major) << MINOR_BITS) | (minor))

/* convenience definition: the NULL device id: */
#define NULL_DEVID              (MKDEVID(0, 0))
#define MEM_NULL_DEVID          (MKDEVID(1, 0))
#define MEM_ZERO_DEVID          (MKDEVID(1, 1))

#define DISK_MAJOR 1

#define MEM_MAJOR       1
#define MEM_NULL_MINOR  0
#define MEM_ZERO_MINOR  1
