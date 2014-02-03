/*
 *  FILE: file.c
 *  AUTH: mcc | mahrens
 *  DESC:
 *  DATE: Fri Apr  3 20:38:30 1998
 */

#include "kernel.h"
#include "util/init.h"
#include "util/debug.h"
#include "util/string.h"
#include "util/printf.h"
#include "globals.h"
#include "util/list.h"
#include "fs/file.h"
#include "fs/vfs.h"
#include "fs/vnode.h"
#include "proc/proc.h"
#include "mm/slab.h"
#include "config.h"

static slab_allocator_t *file_allocator;

static __attribute__((unused)) void
file_init(void)
{
        file_allocator = slab_allocator_create("file", sizeof(file_t));
}
init_func(file_init);

void
fref(file_t *f)
{
        KASSERT(f->f_mode >= 0 && f->f_mode < 8);
        KASSERT(f->f_pos >= -1);
        KASSERT(f->f_refcount >= 0);
        if (f->f_refcount != 0) KASSERT(f->f_vnode);

        f->f_refcount++;

        if (f->f_vnode) {
                dbg(DBG_FREF, "fref: 0x%p, 0x%p ino %lu, up to %d\n",
                    f, f->f_vnode->vn_fs, (unsigned long) f->f_vnode->vn_vno,
                    f->f_refcount);
        } else {
                dbg(DBG_FREF, "fref: 0x%p up to %d\n", f, f->f_refcount);
        }
}

/* Add vnode to file, and call acquire if available. */
void
facq(file_t *f, vnode_t *vn)
{
        KASSERT(f);
        KASSERT(f->f_vnode == NULL);
        KASSERT(vn);

        f->f_vnode = vn;
        if (vn->vn_ops->acquire) {
                vn->vn_ops->acquire(vn, f);
        }
}

/* Look in process fd table and return the file*. */
file_t *
fget(int fd)
{
        file_t *f;

        if (fd == -1) {
                f = slab_obj_alloc(file_allocator);
                if (f) memset(f, 0, sizeof(file_t));
        } else {
                if (fd < 0 || fd >= NFILES)
                        return NULL;
                f = curproc->p_files[fd];
        }
        if (f) fref(f);

        return f;
}

/* - Decrement f_count.
 * - If f_count == 0, call release (if available), vput() and free it. */
void
fput(file_t *f)
{
        KASSERT(f);
        KASSERT(f->f_mode >= 0 && f->f_mode < 8);
        KASSERT(f->f_pos >= -1);
        KASSERT(f->f_refcount > 0);
        if (f->f_refcount != 1) KASSERT(f->f_vnode);

        f->f_refcount--;

        if (f->f_vnode) {
                dbg(DBG_FREF, "fput: 0x%p, 0x%p ino %lu, down to %d\n",
                    f, f->f_vnode->vn_fs, (unsigned long) f->f_vnode->vn_vno,
                    f->f_refcount);
        } else {
                dbg(DBG_FREF, "fput: 0x%p down to %d\n", f, f->f_refcount);
        }

        if (f->f_refcount == 0) {
                vnode_t *vn = f->f_vnode;
                if (vn) {
                        if (vn->vn_ops->release) {
                                vn->vn_ops->release(vn, f);
                        }
                        vput(vn);
                }
                slab_obj_free(file_allocator, f);
        }
}
