/*
 *  FILE: vfs.c
 *  AUTH: mcc
 *  DESC:
 *  DATE: Wed Mar 18 15:44:25 1998
 *  $Id: vfs.c,v 1.8.2.1 2006/06/04 01:02:32 afenn Exp $
 */

#include "kernel.h"
#include "globals.h"
#include "util/string.h"
#include "util/printf.h"
#include "errno.h"

#ifdef __S5FS__
#include "fs/s5fs/s5fs.h"
#endif
#include "fs/vfs.h"
#include "fs/file.h"
#include "fs/vnode.h"
#include "fs/vfs_syscall.h"
#include "fs/ramfs/ramfs.h"

#include "fs/stat.h"
#include "fs/fcntl.h"
#include "mm/slab.h"
#include "mm/kmalloc.h"
#include "util/debug.h"

vnode_t *vfs_root_vn;

#ifdef __MOUNTING__
/* The fs listed here are only the non-root file systems */
list_t mounted_fs_list;

/*
 * Implementing this function is not required and strongly discouraged unless
 * you are absolutley sure your Weenix is perfect.
 *
 * The purpose of this function is to set up the pointers between the file
 * system struct and the vnode of the mount point. Remember to watch your
 * reference counts. (The exception here is when the vnode's vn_mount field
 * points to the mounted file system's root we do not increment the reference
 * count on the file system's root vnode. The file system is already keeping
 * a reference to the vnode which will not go away until the file system is
 * unmounted. If we kept a second such reference it would conflict with the
 * behavior of vfs_is_in_use(), make sure you understand why.)
 *
 * Once everything is set up add the file system to the list of mounted file
 * systems.
 *
 * Remember proper error handling.
 *
 * This function is not meant to mount the root file system.
 */
int
vfs_mount(struct vnode *mtpt, fs_t *fs)
{
        NOT_YET_IMPLEMENTED("MOUNTING: vfs_mount");
        return -EINVAL;
}

/*
 * Implementing this function is not required and strongly discouraged unless
 * you are absolutley sure your Weenix is perfect.
 *
 * The purpose of this function is to undo the setup done in vfs_mount(). Also
 * you should call the underlying file system's umount() function. Make sure
 * to keep track of reference counts. You should also kfree the fs struct at
 * the end of this method.
 *
 * Remember proper error handling. You might want to make sure that you do not
 * try to call this function on the root file system (this function is not meant
 * to unmount the root file system).
 */
int
vfs_umount(fs_t *fs)
{
        NOT_YET_IMPLEMENTED("MOUNTING: vfs_umount");
        return -EINVAL;
}
#endif /* __MOUNTING__ */

/*     Called by the idle process during system initialization.
 *     Performs the following work:
 *         - initializes vnode management
 *         - initializes file descriptor management
 *
 *         - mounts the root (and only) filesystem
 *             - create an fs_t, fs, to represent the root filesystem
 *             - initialize fs->fs_type to VFS_ROOTFS_TYPE and, if
 *               appropriate (see include/weenix/config.h), fs->fs_dev to
 *               VFS_ROOTFS_DEV.
 *
 *             - Use 'mountfunc(...)' to attempt to identify and call the
 *               appropriate mount function.
 *
 *             - set vfs_root_vn appropriately
 */
static __attribute__((unused)) void
vfs_init(void)
{
        int err;
        fs_t *fs;

        /* mount the root (and only) filesystem: */
        /*     create and init an fs_t for the fs: */
        fs = (fs_t *) kmalloc(sizeof(fs_t));
        KASSERT(fs
                && "shouldn\'t be running out of memory this early in "
                "the game");
        memset(fs, 0, sizeof(fs_t));
        strcpy(fs->fs_type, VFS_ROOTFS_TYPE);
        if (VFS_ROOTFS_DEV) {
                strcpy(fs->fs_dev, VFS_ROOTFS_DEV);
        }

        /*     attempt to find and call appropriate mount routine: */
        if (0 > (err = mountfunc(fs))) {
                panic("Failed to mount root fs of type \"%s\" on device "
                      "\"%s\" with errno of %d\n",
                      VFS_ROOTFS_TYPE, VFS_ROOTFS_DEV, -err);
        }

        vfs_root_vn = fs->fs_root;

#ifdef __MOUNTING__
        list_init(&mounted_fs_list);
        fs->fs_mtpt = vfs_root_vn;
#endif
}
init_func(vfs_init);
init_depends(vnode_init);
init_depends(file_init);

int
vfs_shutdown()
{
        /*
         * - unmount the root filesystem
         */
        fs_t *fs;
        vnode_t *vn;
        int ret = 0;

        KASSERT(vfs_root_vn);

#ifdef __MOUNTING__
        fs_t *mtfs;
        list_iterate_begin(&mounted_fs_list, mtfs, fs_t, fs_link) {
                int ret = vfs_umount(mtfs);
                KASSERT(0 <= ret);
        } list_iterate_end();
#endif


        vn = vfs_root_vn;
        fs = vn->vn_fs;

        /* 'vfs_shutdown' is called after there are no processes other than
         * idleproc running. idleproc does not have a p_cwd. Thus, there
         * should be no live vnodes */

        if (0 > vfs_is_in_use(fs)) {
                panic("vfs_shutdown: found active vnodes in root "
                      "filesystem!!! This shouldn't happen!!\n");
        }

        if (vn->vn_fs->fs_op->umount) {
                ret = vn->vn_fs->fs_op->umount(fs);
        } else {
                vput(vn);
        }

        KASSERT((!vnode_inuse(fs))
                && "should have been taken care of by unmount entry point "
                "or by the above vput of the root vnode");

        vfs_root_vn = NULL; /* not /really/ necessary... */

        kfree(fs);

        return ret;
}

/*
 * Given an fs_t, we search through the list of known file systems
 * and call the proper mount function.
 */
int
mountfunc(fs_t *fs)
{
        static const struct {
                char *fstype;
                int (*mountfunc)(fs_t *);
        } types[] = {
#ifdef __S5FS__
                { "s5fs", s5fs_mount },
#endif
                { "ramfs", ramfs_mount },
        };
        unsigned i;

        for (i = 0; i < sizeof(types) / sizeof(types[0]); i++)
                if (strcmp(fs->fs_type, types[i].fstype) == 0)
                        return types[i].mountfunc(fs);

        return -EINVAL;
}
