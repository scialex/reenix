#pragma once

#include "types.h"

#define FMODE_READ    1
#define FMODE_WRITE   2
#define FMODE_APPEND  4

struct vnode;

typedef struct file {
        /*
         * The current position in the file. Can be modified by system calls
         * like lseek(2), read(2), and write(2) (and possibly others) as
         * described in the man pages of those calls.
         */
        off_t                   f_pos;

        /*
         * The mode in which this file was opened. This is a mask of the flags
         * FMODE_READ, FMODE_WRITE, and FMODE_APPEND. It is set when the file
         * is first opened, and use to restrict the operations that can be
         * performed on the underlying vnode.
         */
        int                     f_mode;

        /*
         * The number of references to this struct. This is nearly the same as
         * the vnode's vn_refcount member.
         */
        int                     f_refcount;

        /*
         * The vnode which corresponds to this file.
         */
        struct vnode            *f_vnode;
} file_t;

/*
 * Returns the file_t assiciated with the given file descriptor for the
 * current process. If there is no associated file_t, returns NULL.
 *
 * If the passed fd == -1, a new file_t, not associated with any process
 * or file descriptor, is created and returned with its refcount set to
 * 1.
 */
struct file *fget(int fd);

/*
 * Places the vnode vn inside of the given file, possibly calling the
 * acquire vnode operation if one exists.
 */
void facq(file_t *f, struct vnode *vn);

/*
 * fref() increments the reference count on the given file.
 */
void fref(file_t *f);

/*
 * fput() decrements the reference count on the given file.
 *
 * If the refcount reaches 0, the storage for the given file_t will be
 * released (f won't point to a valid memory address anymore), and the
 * refcount on the associated vnode (if any) will be decremented.
 * The vnode release operation will also be called if it exists.
 */
void fput(file_t *f);
