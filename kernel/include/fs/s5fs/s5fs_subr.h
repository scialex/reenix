/*
 *   FILE: s5fs_subr.h
 * AUTHOR: afenn
 *  DESCR: S5 low-level subroutines
 */

#pragma once

#include "types.h"

struct fs;
struct vnode;

int s5_alloc_inode(struct fs *fs, uint16_t type, devid_t devid);
void s5_free_inode(struct vnode *vnode);


int s5_read_file(struct vnode *vn, off_t seek, char *dest, size_t len);
int s5_write_file(struct vnode *vn, off_t seek, const char *bytes,
                  size_t len);

/* TA BLANK {{{ */
/* TODO: perhaps change the order of the arguments 'parent' and 'child' to
 * stay consistent with the order that is observed at the vfs level */
/* TA BLANK }}} */
int s5_link(struct vnode *parent, struct vnode *child,
            const char *name, size_t namelen);
int s5_find_dirent(struct vnode *vnode, const char *name, size_t namelen);
int s5_remove_dirent(struct vnode *vnode, const char *name, size_t namelen);
int s5_seek_to_block(struct vnode *vnode, off_t seekptr, int alloc);
int s5_inode_blocks(struct vnode *vnode);

#define VNODE_TO_S5FS(vn)       ( (s5fs_t *)((vn)->vn_fs->fs_i))
#define VNODE_TO_S5INODE(vn)    ( (s5_inode_t *)(vn)->vn_i )
#define S5FS_TO_VMOBJ(s5fs)     (&(s5fs)->s5f_bdev->bd_mmobj)

/* TA BLANK {{{ */
/* TODO: change args to be more natural for how things are arranged in this
 * experimental version of things */
/* TA BLANK }}} */
#define s5_dirty_inode(fs, inode)                                       \
        do {                                                            \
                pframe_t *p;                                            \
                int err;                                                \
                pframe_get(S5FS_TO_VMOBJ((fs)),                         \
                           S5_INODE_BLOCK((inode)->s5_number), &p);     \
                KASSERT(p);                                             \
                err = pframe_dirty(p);                                  \
                KASSERT(!err                                            \
                        && "shouldn\'t fail for a page belonging "      \
                        "to a block device");                           \
        } while (0)

/*
 * A Note from the Fennster:
 * I am intentionally not defining something like s5_dirty_block --> I
 * think this would make it too easy not to think about whether or not you
 * should be dirtying the page via the block device vm_object
 */
