/*
 *   FILE: config.h
 * AUTHOR: kma
 *  DESCR: tunable kernel parameters
 */

#pragma once

/* Kernel and user header (via symlink) */

/*
 * kernel configuration parameters
 */
#define DEFAULT_STACK_SIZE      (56*1024) /* size of stacks */
#define TICK_MSECS              10        /* msecs between clock interrupts */

/*
 * Memory-management-related:
 */

/*
 *     finds fraction of available page frames that will be dedicated to kmem
 *     the rest are given to the vm system
 */
#define KMEM_FRAC(x)               (((x)>>2)+((x)>>3)) /* 37.5%-ish */

/*     pframe/mmobj-system-related: */
#define PF_HASH_SIZE                  17 /* Number of buckets in pn/mmobj->pframe hash */
/*         Pageout-related: */
#define PAGEOUTD_FREE_TARGET_SHIFT     5 /* 3.125% */
#define PAGEOUTD_FREE_MIN_SHIFT        4 /* 6.25% */


/*
 * filesystem/vfs configuration parameters
 */

#define MAXPATHLEN              1024    /* maximum size of a pathname */
#define MAX_FILES               1024    /* max number of files */
#define MAX_VFS                 8       /* max # of vfses */
#define MAX_VNODES              1024    /* max number of in-core vnodes */
#define NAME_LEN                28      /* maximum directory entry length */
#define NFILES                  32      /* maximum number of open files */

/* Note: if rootfs is ramfs, this is completely ignored */
#define VFS_ROOTFS_DEV  "disk0" /* device containing root filesystem */

#ifdef __S5FS__
/* root filesystem type - either "ramfs" or "s5fs" */
#    define VFS_ROOTFS_TYPE "s5fs"
#else
#    define VFS_ROOTFS_TYPE "ramfs"
#endif
