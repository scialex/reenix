/*
 *   FILE: s5fs.h
 * AUTHOR: kma
 *  DESCR: shared structures for the System V file system...
 */

#pragma once

#ifdef __FSMAKER__
#include <stdint.h>
#else
#include "config.h"

#include "proc/kmutex.h"
#include "fs/vfs.h"
#include "mm/page.h"
#include "drivers/blockdev.h"
#endif

#define S5_SUPER_BLOCK          0       /* the blockno of the superblock */
#define S5_IS_SUPER(blkno)      ( (blkno) == S5_SUPER_BLOCK )
#define S5_NBLKS_PER_FNODE      30
#define S5_BLOCK_SIZE           4096
#define S5_NDIRECT_BLOCKS       28
#define S5_INODES_PER_BLOCK     (S5_BLOCK_SIZE /  sizeof(s5_inode_t))
#define S5_DIRENTS_PER_BLOCK    (S5_BLOCK_SIZE / sizeof(s5_dirent_t))
#define S5_MAX_FILE_BLOCKS      (S5_NDIRECT_BLOCKS + (S5_BLOCK_SIZE / sizeof(uint32_t)))
#define S5_NAME_LEN             28

#define S5_TYPE_FREE            0x0
#define S5_TYPE_DATA            0x1
#define S5_TYPE_DIR             0x2
#define S5_TYPE_CHR             0x4
#define S5_TYPE_BLK             0x8

#define S5_MAGIC                071177
#define S5_CURRENT_VERSION      3

/* Number of blocks stored in the indirect block */
#define S5_NIDIRECT_BLOCKS      (S5_BLOCK_SIZE / sizeof(uint32_t))

/* Given a file offset, returns the block number that it is in */
#define S5_DATA_BLOCK(seekptr)  ((seekptr) / S5_BLOCK_SIZE)

/* Given a file offset, returns the offset into the pointer's block */
#define S5_DATA_OFFSET(seekptr) ((seekptr) % S5_BLOCK_SIZE)

/* Given an inode number, tells the block that inode is stored in. */
#define S5_INODE_BLOCK(inum)    ((inum) / S5_INODES_PER_BLOCK + 1)

/*
 * Given an inode number, tells the offset (in units of s5_inode_t) of
 * that inode within the block returned by S5_INODE_BLOCK.
 */
#define S5_INODE_OFFSET(inum)  ((inum) % S5_INODES_PER_BLOCK)

/* Given an FS struct, get the S5FS (private data) struct. */
#define FS_TO_S5FS(fs)  ( (s5fs_t *)((fs)->fs_i))

/* each node of the free block list looks like this: */
/*
typedef struct s5_fbl_node {
        int free_blocks[S5_NBLKS_PER_FNODE-1];
        int more;
} s5_fbl_node_t;
*/

/* Note that all on-disk types need to have hard-coded sizes (to ensure
 * inter-machine compatibility of s5 disks) */

/* The contents of the superblock, as stored on disk. */
typedef struct s5_super {
        uint32_t s5s_magic;              /* the magic number */
        uint32_t s5s_free_inode;         /* the free inode pointer */
        uint32_t s5s_nfree;              /* number of blocks currently in
                                          * s5s_free_blocks */
        /** First "node" of free block list */
        uint32_t s5s_free_blocks[S5_NBLKS_PER_FNODE];

        uint32_t s5s_root_inode;         /* root inode */
        uint32_t s5s_num_inodes;         /* number of inodes */
        uint32_t s5s_version;            /* version of this disk format */
} s5_super_t;

/* The contents of an inode, as stored on disk. */
typedef struct s5_inode {
        union {
                uint32_t s5_next_free; /* inode free list ptr */
                uint32_t s5_size;      /* file size */
        } s5_un;
#define        s5_next_free s5_un.s5_next_free
#define        s5_size      s5_un.s5_size
        uint32_t   s5_number;              /* this inode's number */
        uint16_t   s5_type;         /* one of S5_TYPE_{FREE,DATA,DIR} */
        int16_t    s5_linkcount;    /* link count of this inode */
        uint32_t   s5_direct_blocks[S5_NDIRECT_BLOCKS];
        uint32_t   s5_indirect_block;
} s5_inode_t;

/* The contents of a directory entry, as stored on disk. */
typedef struct s5_dirent {
        uint32_t   s5d_inode;
        char       s5d_name[S5_NAME_LEN];
} s5_dirent_t;

#ifndef __FSMAKER__
/* Our in-memory representation of a s5fs filesytem (fs_i points to this) */
typedef struct s5fs {
        blockdev_t              *s5f_bdev;
        s5_super_t              *s5f_super;
        kmutex_t                s5f_mutex;
        fs_t                    *s5f_fs;
} s5fs_t;

int s5fs_mount(struct fs *fs);
#endif
