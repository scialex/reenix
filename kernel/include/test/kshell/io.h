#pragma once

#include "test/kshell/kshell.h"

/*
 * When writing a kernel shell command, make sure to use the following
 * I/O functions.
 *
 * Before VFS is not enabled, the kernel shell will use functions from
 * bytedev.h to get a pointer the the bytedev_t struct for the TTY.
 *
 * When VFS is enabled, the kernel shell will use the functions from
 * vfs_syscall.h to open and close the TTY and perform I/O operations
 * on the TTY.
 *
 * If you use the functions below, this process will be completely
 * transparent.
 */

/**
 * Replacement for do_write.
 *
 * @param ksh the kshell to write to
 * @param buf the buffer to write out to the kshell
 * @param nbytes the maximum number of bytes to write
 * @return number of bytes written on sucess and <0 on error
 */
int kshell_write(kshell_t *ksh, const void *buf, size_t nbytes);

/**
 * Replacement for do_read.
 *
 * @param ksh the kshell to read from
 * @param buf the buffer to store data read from the kshell
 * @param nbytes the maximum number of bytes to read
 * @param number of bytes read on success and <0 on error
 */
int kshell_read(kshell_t *ksh, void *buf, size_t nbytes);

/* Unless an error occurs, guarantees that all of buf will be
 * written */
/**
 * Writes a specified number of bytes from a buffer to the
 * kshell. Unlike kshell_write, this function guarantees it will write
 * out the desired number of bytes.
 *
 * @param ksh the kshell to write to
 * @param buf the buffer to write out to the kshell
 * @param nbytes the number of bytes to write
 * @return number of bytes written on success and <0 on error
 */
int kshell_write_all(kshell_t *ksh, void *buf, size_t nbytes);

/* Replacement for printf */
/**
 * Write output to a kshell according to a format string.
 *
 * @param ksh the kshell to write to
 * @param fmt the format string
 */
void kprintf(kshell_t *ksh, const char *fmt, ...);
