/*
 * This is a special filesystem designed to be a test filesystem before s5fs has
 * been written.  It is an in-memory filesystem that supports almost all of the
 * vnode operations.  It has the following restrictions:
 *
 *    o File sizes are limited to a single page (8192 bytes) in order
 *      to keep the code simple.
 *
 *    o There is no support for fillpage, etc. (since we don't have VM yet anyway)
 *
 *    o There is a maximum directory size limit
 *
 *    o There is a maximum number of files/directories limit
 */

#include "mm/mm.h"
#include "kernel.h"
#include "globals.h"
#include "fs/vfs.h"
#include "fs/vnode.h"
#include "errno.h"
#include "util/string.h"
#include "util/printf.h"
#include "fs/stat.h"
#include "fs/dirent.h"
#include "util/debug.h"
#include "mm/kmalloc.h"

#include "fs/ramfs/ramfs.h"

/*
 * Filesystem operations
 */
static void ramfs_read_vnode(vnode_t *vn);
static void ramfs_delete_vnode(vnode_t *vn);
static int ramfs_query_vnode(vnode_t *vn);
static int ramfs_umount(fs_t *fs);

static fs_ops_t ramfs_ops = {
        .read_vnode   = ramfs_read_vnode,
        .delete_vnode = ramfs_delete_vnode,
        .query_vnode  = ramfs_query_vnode,
        .umount       = ramfs_umount
};

/*
 * vnode operations
 */
static int ramfs_read(vnode_t *file, off_t offset, void *buf, size_t count);
static int ramfs_write(vnode_t *file, off_t offset, const void *buf, size_t count);
/* getpage */
static int ramfs_create(vnode_t *dir, const char *name, size_t name_len,
                        vnode_t **result);
static int ramfs_mknod(struct vnode *dir, const char *name, size_t name_len,
                       int mode, devid_t devid);
static int ramfs_lookup(vnode_t *dir, const char *name, size_t name_len,
                        vnode_t **result);
static int ramfs_link(vnode_t *oldvnode, vnode_t *dir,
                      const char *name, size_t name_len);
static int ramfs_unlink(vnode_t *dir, const char *name, size_t name_len);
static int ramfs_mkdir(vnode_t *dir, const char *name, size_t name_len);
static int ramfs_rmdir(vnode_t *dir, const char *name, size_t name_len);
static int ramfs_readdir(vnode_t *dir, off_t offset, struct dirent *d);
static int ramfs_stat(vnode_t *file, struct stat *buf);

static vnode_ops_t ramfs_dir_vops = {
        .read = NULL,
        .write = NULL,
        .mmap = NULL,
        .create = ramfs_create,
        .mknod = ramfs_mknod,
        .lookup = ramfs_lookup,
        .link = ramfs_link,
        .unlink = ramfs_unlink,
        .mkdir = ramfs_mkdir,
        .rmdir = ramfs_rmdir,
        .readdir = ramfs_readdir,
        .stat = ramfs_stat,
        .acquire = NULL,
        .release = NULL,
        .fillpage = NULL,
        .dirtypage = NULL,
        .cleanpage = NULL
};

static vnode_ops_t ramfs_file_vops = {
        .read = ramfs_read,
        .write = ramfs_write,
        .mmap = NULL,
        .create = NULL,
        .mknod = NULL,
        .lookup = NULL,
        .link = NULL,
        .unlink = NULL,
        .mkdir = NULL,
        .rmdir = NULL,
        .stat = ramfs_stat,
        .acquire = NULL,
        .release = NULL,
        .fillpage = NULL,
        .dirtypage = NULL,
        .cleanpage = NULL
};

/*
 * The ramfs 'inode' structure
 */
typedef struct ramfs_inode {
        off_t     rf_size;       /* Total file size */
        ino_t     rf_ino;        /* Inode number */
        char     *rf_mem;        /* Memory for this file (1 page) */
        int       rf_mode;       /* Type of file */
        int       rf_linkcount;  /* Number of links to this file */
} ramfs_inode_t;

#define RAMFS_TYPE_DATA   0
#define RAMFS_TYPE_DIR    1
#define RAMFS_TYPE_CHR    2
#define RAMFS_TYPE_BLK    3

#define VNODE_TO_RAMFSINODE(vn) \
        ((ramfs_inode_t *)(vn)->vn_i)
#define VNODE_TO_RAMFS(vn) \
        ((ramfs_t *)(vn)->vn_fs->fs_i)
#define VNODE_TO_DIRENT(vn) \
        ((ramfs_dirent_t *)VNODE_TO_RAMFSINODE(vn)->rf_mem)

/*
 * ramfs filesystem structure
 */
#define RAMFS_MAX_FILES   64

typedef struct ramfs {
        ramfs_inode_t *rfs_inodes[RAMFS_MAX_FILES];  /* Array of all files */
} ramfs_t;

/*
 * For directories, we simply store an array of (ino, name) pairs in the
 * memory portion of the inode.
 */
typedef struct ramfs_dirent {
        int             rd_ino;   /* Inode number of this entry */
        char            rd_name[NAME_LEN];   /* Name of this entry */
} ramfs_dirent_t;

#define RAMFS_MAX_DIRENT  ((off_t)(PAGE_SIZE/sizeof(ramfs_dirent_t)))

/* Helper functions */
static int
ramfs_alloc_inode(fs_t *fs, int type, devid_t devid)
{
        ramfs_t *rfs = (ramfs_t *) fs->fs_i;
        KASSERT((RAMFS_TYPE_DATA == type)
                || (RAMFS_TYPE_DIR == type)
                || (RAMFS_TYPE_CHR == type)
                || (RAMFS_TYPE_BLK == type));
        /* Find a free inode */
        int i;
        for (i = 0; i < RAMFS_MAX_FILES; i++) {
                if (NULL == rfs->rfs_inodes[i]) {
                        ramfs_inode_t *inode;
                        if (NULL == (inode = kmalloc(sizeof(ramfs_inode_t)))) {
                                return -ENOSPC;
                        }

                        if (RAMFS_TYPE_CHR == type || RAMFS_TYPE_BLK == type) {
                                /* Don't need any space in memory, so put devid in here */
                                inode->rf_mem = (char *) devid;
                        } else {
                                /* We allocate space for the file's contents immediately */
                                if (NULL == (inode->rf_mem = page_alloc())) {
                                        kfree(inode);
                                        return -ENOSPC;
                                }
                                memset(inode->rf_mem, 0, PAGE_SIZE);
                        }
                        inode->rf_size = 0;
                        inode->rf_ino = i;
                        inode->rf_mode = type;
                        inode->rf_linkcount = 1;

                        /* Install in table and return */
                        rfs->rfs_inodes[i] = inode;
                        return i;
                }
        }
        return -ENOSPC;
}

/*
 * Function implementations
 */

int
ramfs_mount(struct fs *fs)
{

        /* Allocate filesystem */
        ramfs_t *rfs = kmalloc(sizeof(ramfs_t));
        if (NULL == rfs)
                return -ENOMEM;

        memset(rfs->rfs_inodes, 0, sizeof(rfs->rfs_inodes));

        fs->fs_i = rfs;
        fs->fs_op = &ramfs_ops;

        /* Set up root inode */
        int root_ino;
        if (0 > (root_ino = ramfs_alloc_inode(fs, RAMFS_TYPE_DIR, 0))) {
                return root_ino;
        }
        KASSERT(0 == root_ino);
        ramfs_inode_t *root = rfs->rfs_inodes[root_ino];

        /* Set up '.' and '..' in the root directory */
        ramfs_dirent_t *rootdent = (ramfs_dirent_t *) root->rf_mem;
        rootdent->rd_ino = 0;
        strcpy(rootdent->rd_name, ".");
        rootdent++;
        rootdent->rd_ino = 0;
        strcpy(rootdent->rd_name, "..");

        /* Increase root inode size accordingly */
        root->rf_size = 2 * sizeof(ramfs_dirent_t);

        /* Put the root in the inode table */
        rfs->rfs_inodes[0] = root;

        /* And vget the root vnode */
        fs->fs_root = vget(fs, 0);

        return 0;
}

static void
ramfs_read_vnode(vnode_t *vn)
{
        ramfs_t *rfs = VNODE_TO_RAMFS(vn);
        ramfs_inode_t *inode = rfs->rfs_inodes[vn->vn_vno];
        KASSERT(inode && inode->rf_ino == vn->vn_vno);

        inode->rf_linkcount++;

        vn->vn_i = inode;
        vn->vn_len = inode->rf_size;

        switch (inode->rf_mode) {
                case RAMFS_TYPE_DATA:
                        vn->vn_mode = S_IFREG;
                        vn->vn_ops = &ramfs_file_vops;
                        break;
                case RAMFS_TYPE_DIR:
                        vn->vn_mode = S_IFDIR;
                        vn->vn_ops = &ramfs_dir_vops;
                        break;
                case RAMFS_TYPE_CHR:
                        vn->vn_mode = S_IFCHR;
                        vn->vn_ops = NULL;
                        vn->vn_devid = (devid_t)(inode->rf_mem);
                        break;
                case RAMFS_TYPE_BLK:
                        vn->vn_mode = S_IFBLK;
                        vn->vn_ops = NULL;
                        vn->vn_devid = (devid_t)(inode->rf_mem);
                        break;
                default:
                        panic("inode %d has unknown/invalid type %d!!\n",
                              (int)vn->vn_vno, (int)inode->rf_mode);
        }
}

static void
ramfs_delete_vnode(vnode_t *vn)
{
        ramfs_inode_t *inode = VNODE_TO_RAMFSINODE(vn);
        ramfs_t *rfs = VNODE_TO_RAMFS(vn);

        if (0 == --inode->rf_linkcount) {
                KASSERT(rfs->rfs_inodes[vn->vn_vno] == inode);

                rfs->rfs_inodes[vn->vn_vno] = NULL;
                if (inode->rf_mode == RAMFS_TYPE_DATA
                    || inode->rf_mode == RAMFS_TYPE_DIR) {
                        page_free(inode->rf_mem);
                }
                /* otherwise, inode->rf_mem is a devid */

                kfree(inode);
        }
}

static int
ramfs_query_vnode(vnode_t *vn)
{
        return VNODE_TO_RAMFSINODE(vn)->rf_linkcount > 1;
}

static int
ramfs_umount(fs_t *fs)
{
        /* We don't need to do any flushing or anything as everything is in memory.
         * Just free all of our allocated memory */
        ramfs_t *rfs = (ramfs_t *) fs->fs_i;

        vput(fs->fs_root);

        /* Free all the inodes */
        int i;
        for (i = 0; i < RAMFS_MAX_FILES; i++) {
                if (NULL != rfs->rfs_inodes[i]) {
                        if (NULL != rfs->rfs_inodes[i]->rf_mem
                            && (rfs->rfs_inodes[i]->rf_mode == RAMFS_TYPE_DATA
                                || rfs->rfs_inodes[i]->rf_mode == RAMFS_TYPE_DIR)) {
                                page_free(rfs->rfs_inodes[i]->rf_mem);
                        }
                        kfree(rfs->rfs_inodes[i]);
                }
        }

        return 0;
}

static int
ramfs_create(vnode_t *dir, const char *name, size_t name_len, vnode_t **result)
{
        vnode_t *vn;
        off_t i;
        ramfs_dirent_t *entry;

        KASSERT(0 != ramfs_lookup(dir, name, name_len, &vn));

        /* Look for space in the directory */
        entry = VNODE_TO_DIRENT(dir);
        for (i = 0; i < RAMFS_MAX_DIRENT; i++, entry++) {
                if (!entry->rd_name[0])
                        break;
        }

        if (i == RAMFS_MAX_DIRENT) {
                return -ENOSPC;
        }

        /* Allocate an inode */
        int ino;
        if (0 > (ino = ramfs_alloc_inode(dir->vn_fs, RAMFS_TYPE_DATA, 0))) {
                return ino;
        }

        /* Get a vnode, set entry in directory */
        vn = vget(dir->vn_fs, (ino_t) ino);

        entry->rd_ino = vn->vn_vno;
        strncpy(entry->rd_name, name, MIN(name_len, NAME_LEN - 1));
        entry->rd_name[MIN(name_len, NAME_LEN - 1)] = '\0';

        VNODE_TO_RAMFSINODE(dir)->rf_size += sizeof(ramfs_dirent_t);

        *result = vn;

        return 0;
}


static int
ramfs_mknod(struct vnode *dir, const char *name, size_t name_len, int mode, devid_t devid)
{
        vnode_t *vn;
        off_t i;
        ramfs_dirent_t *entry;

        KASSERT(0 != ramfs_lookup(dir, name, name_len, &vn));

        /* Look for space in the directory */
        entry = VNODE_TO_DIRENT(dir);
        for (i = 0; i < RAMFS_MAX_DIRENT; i++, entry++) {
                if (!entry->rd_name[0])
                        break;
        }

        if (i == RAMFS_MAX_DIRENT) {
                return -ENOSPC;
        }

        int ino;
        if (S_ISCHR(mode)) {
                if (0 > (ino = ramfs_alloc_inode(dir->vn_fs, RAMFS_TYPE_CHR, devid))) {
                        return ino;
                }
        } else if (S_ISBLK(mode)) {
                if (0 > (ino = ramfs_alloc_inode(dir->vn_fs, RAMFS_TYPE_BLK, devid))) {
                        return ino;
                }
        } else {
                panic("Invalid mode!\n");
        }

        /* Set entry in directory */
        entry->rd_ino = ino;
        strncpy(entry->rd_name, name, MIN(name_len, NAME_LEN - 1));
        entry->rd_name[MIN(name_len, NAME_LEN - 1)] = '\0';

        VNODE_TO_RAMFSINODE(dir)->rf_size += sizeof(ramfs_dirent_t);

        return 0;
}

static int
ramfs_lookup(vnode_t *dir, const char *name, size_t namelen, vnode_t **result)
{
        off_t i;
        ramfs_inode_t *inode = VNODE_TO_RAMFSINODE(dir);
        ramfs_dirent_t *entry = (ramfs_dirent_t *)inode->rf_mem;

        for (i = 0; i < RAMFS_MAX_DIRENT; i++, entry++) {
                if (name_match(entry->rd_name, name, namelen)) {
                        *result = vget(dir->vn_fs, entry->rd_ino);
                        return 0;
                }
        }

        return -ENOENT;
}

static int
ramfs_link(vnode_t *oldvnode, vnode_t *dir,
           const char *name, size_t name_len)
{
        vnode_t *vn;
        off_t i;
        ramfs_dirent_t *entry;

        KASSERT(oldvnode->vn_fs == dir->vn_fs);
        KASSERT(0 != ramfs_lookup(dir, name, name_len, &vn));

        /* Look for space in the directory */
        entry = VNODE_TO_DIRENT(dir);
        for (i = 0; i < RAMFS_MAX_DIRENT; i++, entry++) {
                if (!entry->rd_name[0])
                        break;
        }

        if (i == RAMFS_MAX_DIRENT) {
                return -ENOSPC;
        }

        /* Set entry in parent */
        entry->rd_ino = oldvnode->vn_vno;
        strncpy(entry->rd_name, name, MIN(name_len, NAME_LEN - 1));
        entry->rd_name[MIN(name_len, NAME_LEN - 1)] = '\0';

        VNODE_TO_RAMFSINODE(dir)->rf_size += sizeof(ramfs_dirent_t);

        /* Increase linkcount */
        VNODE_TO_RAMFSINODE(oldvnode)->rf_linkcount++;
        return 0;
}

static int
ramfs_unlink(vnode_t *dir, const char *name, size_t namelen)
{
        vnode_t *vn;
        int ret;
        off_t i;
        ramfs_dirent_t *entry;

        ret = ramfs_lookup(dir, name, namelen, &vn);
        KASSERT(0 == ret);
        KASSERT(!S_ISDIR(vn->vn_mode));

        /* And then remove the entry from the directory */
        entry = VNODE_TO_DIRENT(dir);
        for (i = 0; i < RAMFS_MAX_DIRENT; i++, entry++) {
                if (name_match(entry->rd_name, name, namelen)) {
                        entry->rd_name[0] = '\0';
                        break;
                }
        }

        VNODE_TO_RAMFSINODE(dir)->rf_size -= sizeof(ramfs_dirent_t);

        VNODE_TO_RAMFSINODE(vn)->rf_linkcount--;
        vput(vn);

        return 0;
}

static int
ramfs_mkdir(vnode_t *dir, const char *name, size_t name_len)
{
        vnode_t *vn;
        off_t i;
        ramfs_dirent_t *entry;

        KASSERT(0 != ramfs_lookup(dir, name, name_len, &vn));

        /* Look for space in the directory */
        entry = VNODE_TO_DIRENT(dir);
        for (i = 0; i < RAMFS_MAX_DIRENT; i++, entry++) {
                if (!entry->rd_name[0])
                        break;
        }

        if (i == RAMFS_MAX_DIRENT) {
                return -ENOSPC;
        }

        /* Allocate an inode */
        int ino;
        if (0 > (ino = ramfs_alloc_inode(dir->vn_fs, RAMFS_TYPE_DIR, 0))) {
                return ino;
        }

        /* Set entry in parent */
        entry->rd_ino = ino;
        strncpy(entry->rd_name, name, MIN(name_len, NAME_LEN - 1));
        entry->rd_name[MIN(name_len, NAME_LEN - 1)] = '\0';

        VNODE_TO_RAMFSINODE(dir)->rf_size += sizeof(ramfs_dirent_t);

        /* Set up '.' and '..' in the directory */
        entry = (ramfs_dirent_t *) VNODE_TO_RAMFS(dir)->rfs_inodes[ino]->rf_mem;
        entry->rd_ino = ino;
        strcpy(entry->rd_name, ".");
        entry++;
        entry->rd_ino = dir->vn_vno;
        strcpy(entry->rd_name, "..");

        /* Increase inode size accordingly */
        VNODE_TO_RAMFS(dir)->rfs_inodes[ino]->rf_size = 2 * sizeof(ramfs_dirent_t);

        return 0;
}

static int
ramfs_rmdir(vnode_t *dir, const char *name, size_t name_len)
{
        vnode_t *vn;
        int ret;
        off_t i;
        ramfs_dirent_t *entry;

        KASSERT(!name_match(".", name, name_len) &&
                !name_match("..", name, name_len));

        if ((ret = ramfs_lookup(dir, name, name_len, &vn)) != 0)
                return ret;

        if (!S_ISDIR(vn->vn_mode)) {
                vput(vn);
                return -ENOTDIR;
        }

        /* We have to make sure that this directory is empty */
        entry = VNODE_TO_DIRENT(vn);
        for (i = 0; i < RAMFS_MAX_DIRENT; i++, entry++) {
                if (!strcmp(entry->rd_name, ".") ||
                    !strcmp(entry->rd_name, ".."))
                        continue;

                if (entry->rd_name[0]) {
                        vput(vn);
                        return -ENOTEMPTY;
                }
        }

        /* Finally, remove the entry from the parent directory */
        entry = VNODE_TO_DIRENT(dir);
        for (i = 0; i < RAMFS_MAX_DIRENT; i++, entry++) {
                if (name_match(entry->rd_name, name, name_len)) {
                        entry->rd_name[0] = '\0';
                        break;
                }
        }
        VNODE_TO_RAMFSINODE(dir)->rf_size -= sizeof(ramfs_dirent_t);

        VNODE_TO_RAMFSINODE(vn)->rf_linkcount--;
        vput(vn);

        return 0;
}

static int
ramfs_read(vnode_t *file, off_t offset, void *buf, size_t count)
{
        int ret;
        ramfs_inode_t *inode = VNODE_TO_RAMFSINODE(file);

        KASSERT(!S_ISDIR(file->vn_mode));

        ret = MAX(0, MIN((off_t)count, inode->rf_size - offset));
        memcpy(buf, inode->rf_mem + offset, ret);

        return ret;
}

static int
ramfs_write(vnode_t *file, off_t offset, const void *buf, size_t count)
{
        int ret;
        ramfs_inode_t *inode = VNODE_TO_RAMFSINODE(file);

        KASSERT(!S_ISDIR(file->vn_mode));

        ret = MIN((off_t)count, (off_t)PAGE_SIZE - offset);
        memcpy(inode->rf_mem + offset, buf, ret);

        KASSERT(file->vn_len == inode->rf_size);
        file->vn_len = MAX(file->vn_len, offset + ret);
        inode->rf_size = file->vn_len;

        return ret;
}

static int
ramfs_readdir(vnode_t *dir, off_t offset, struct dirent *d)
{
        int ret = 0;
        ramfs_dirent_t *dir_entry, *targ_entry;

        KASSERT(S_ISDIR(dir->vn_mode));
        KASSERT(0 == offset % sizeof(ramfs_dirent_t));

        dir_entry = VNODE_TO_DIRENT(dir);
        dir_entry = (ramfs_dirent_t *)(((char *)dir_entry) + offset);
        targ_entry = dir_entry;

        while ((offset < (off_t)(RAMFS_MAX_DIRENT * sizeof(ramfs_dirent_t))) && (!targ_entry->rd_name[0])) {
                ++targ_entry;
                offset += sizeof(ramfs_dirent_t);
        }

        if (offset >= (off_t)(RAMFS_MAX_DIRENT * sizeof(ramfs_dirent_t)))
                return 0;

        ret = sizeof(ramfs_dirent_t) + (targ_entry - dir_entry) * sizeof(ramfs_dirent_t);

        d->d_ino = targ_entry->rd_ino;
        d->d_off = 0; /* unused */
        strncpy(d->d_name, targ_entry->rd_name, NAME_LEN - 1);
        d->d_name[NAME_LEN - 1] = '\0';
        return ret;
}

static int
ramfs_stat(vnode_t *file, struct stat *buf)
{
        ramfs_inode_t *i = VNODE_TO_RAMFSINODE(file);
        memset(buf, 0, sizeof(struct stat));
        buf->st_mode    = file->vn_mode;
        buf->st_ino     = (int) file->vn_vno;
        buf->st_dev     = 0;
        if (file->vn_mode == S_IFCHR || file->vn_mode == S_IFBLK) {
                buf->st_rdev  = (int) i->rf_mem;
        }
        buf->st_nlink   = i->rf_linkcount - 1;
        buf->st_size    = (int) i->rf_size;
        buf->st_blksize = (int) PAGE_SIZE;
        buf->st_blocks  = 1;

        return 0;
}
