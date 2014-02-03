#include "test/kshell/io.h"

#include "priv.h"

#ifndef __VFS__
#include "drivers/bytedev.h"
#endif

#ifdef __VFS__
#include "fs/vfs_syscall.h"
#endif

#include "util/debug.h"
#include "util/printf.h"
#include "util/string.h"

/*
 * If VFS is enabled, we can just use the syscalls.
 *
 * If VFS is not enabled, then we need to explicitly call the byte
 * device.
 */

#ifdef __VFS__
int kshell_write(kshell_t *ksh, const void *buf, size_t nbytes)
{
        int retval;

        if ((size_t)(retval = do_write(ksh->ksh_out_fd, buf, nbytes)) != nbytes) {
                /*
                 * In general, do_write does not necessarily have to
                 * write the entire buffer. However, with our
                 * implementation of Weenix and S5FS, this should
                 * ALWAYS work. If it doesn't, it means that something
                 * is wrong.
                 *
                 * Note that we can still receive an error, for
                 * example if we try to write to an invalid file
                 * descriptor. We only panic if some bytes, but not
                 * all of them, are written.
                 */
                if (retval >= 0) {
                        panic("kshell: Write unsuccessfull. Expected %u, got %d\n",
                              nbytes, retval);
                }
        }

        return retval;
}

int kshell_read(kshell_t *ksh, void *buf, size_t nbytes)
{
        return do_read(ksh->ksh_in_fd, buf, nbytes);
}

int kshell_write_all(kshell_t *ksh, void *buf, size_t nbytes)
{
        /* See comment in kshell_write */
        return kshell_write(ksh, buf, nbytes);
}
#else
int kshell_read(kshell_t *ksh, void *buf, size_t nbytes)
{
        return ksh->ksh_bd->cd_ops->read(ksh->ksh_bd, 0, buf, nbytes);
}

int kshell_write(kshell_t *ksh, const void *buf, size_t nbytes)
{
        return ksh->ksh_bd->cd_ops->write(ksh->ksh_bd, 0, buf, nbytes);
}
#endif

void kprint(kshell_t *ksh, const char *fmt, va_list args)
{
        char buf[KSH_BUF_SIZE];
        int count;

        vsnprintf(buf, sizeof(buf), fmt, args);
        count = strnlen(buf, sizeof(buf));
        kshell_write(ksh, buf, count);
}

void kprintf(kshell_t *ksh, const char *fmt, ...)
{
        va_list args;
        va_start(args, fmt);
        kprint(ksh, fmt, args);
        va_end(args);
}
