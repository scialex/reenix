/*
 *  FILE: pipe.c
 *  AUTH: eric
 *  DESC: Implementation of pipe(2) system call.
 *  DATE: Thu Dec 26 17:08:34 2013
 */

#include "errno.h"
#include "globals.h"

#include "fs/file.h"
#include "fs/open.h"
#include "fs/pipe.h"
#include "fs/stat.h"
#include "fs/vfs_syscall.h"
#include "fs/vfs.h"
#include "fs/vnode.h"

#include "mm/slab.h"
#include "mm/kmalloc.h"

#include "proc/sched.h"

#include "util/debug.h"
#include "util/string.h"

#define PIPE_BUF_SIZE 4096

static void pipe_read_vnode(vnode_t *vnode);
static void pipe_delete_vnode(vnode_t *vnode);
static int  pipe_query_vnode(vnode_t *vnode);

static fs_ops_t pipe_fsops = {
        .read_vnode = pipe_read_vnode,
        .delete_vnode = pipe_delete_vnode,
        .query_vnode = pipe_query_vnode,
        /* We don't need a umount because pipefs is never actually mounted. */
        .umount = NULL
};

static fs_t pipe_fs = {
        .fs_dev = "pipe",
        .fs_type = "pipe",
        .fs_op = &pipe_fsops,
        .fs_root = NULL,
        .fs_i = NULL
};

static int pipe_read(vnode_t *vnode, off_t offset, void *buf, size_t len);
static int pipe_write(vnode_t *vnode, off_t offset, const void *buf, size_t len);
static int pipe_stat(vnode_t *vnode, struct stat *ss);
static int pipe_acquire(vnode_t *vnode, file_t *file);
static int pipe_release(vnode_t *vnode, file_t *file);

static vnode_ops_t pipe_vops = {
        .read = pipe_read,
        .write = pipe_write,
        .mmap = NULL,
        .create = NULL,
        .mknod = NULL,
        .lookup = NULL,
        .link = NULL,
        .unlink = NULL,
        .mkdir = NULL,
        .rmdir = NULL,
        .readdir = NULL,
        .stat = pipe_stat,
        .acquire = pipe_acquire,
        .release = pipe_release,
        .fillpage = NULL,
        .dirtypage = NULL,
        .cleanpage = NULL
};

/* struct pipe defines some data specific to pipes. One of these
   should be present in the vn_i field of each pipe vnode. */
typedef struct pipe {
        /* Buffer for data in the pipe, which has been written but not yet read. */
        char      *pv_buf;
        /*
         * Position of the head and number of characters in the buffer. You can
         * write in characters at position head so long as size does not grow beyond
         * the pipe buffer size.
         */
        off_t      pv_head;
        size_t     pv_size;
        /* Number of file descriptors using this pipe for read and write. */
        int        pv_readers;
        int        pv_writers;
        /*
         * Mutexes for reading and writing. Without these, readers might get non-
         * contiguous reads in a single call (for example, if they empty the buffer
         * but still have more to read, then the writer continues writing, waking up
         * a different thread first) and similarly for writers.
         */
        kmutex_t   pv_rdlock;
        kmutex_t   pv_wrlock;
        /*
         * Waitqueues for threads attempting to read from an empty buffer, or
         * write to a full buffer. When the pipe becomes non-empty (or non-full)
         * then the corresponding waitq should be broadcasted on to make sure all
         * of the threads get a chance to go.
         */
        ktqueue_t  pv_read_waitq;
        ktqueue_t  pv_write_waitq;
} pipe_t;

#define VNODE_TO_PIPE(vn) ((pipe_t *)((vn)->vn_i))

static slab_allocator_t *pipe_allocator = NULL;
static int next_pno = 0;

static __attribute__((unused)) void
pipe_init(void)
{
        pipe_allocator = slab_allocator_create("pipe", sizeof(pipe_t));
        KASSERT(pipe_allocator != NULL);
}
init_func(pipe_init);
init_depends(vfs_init);

/*
 * Create a pipe struct here. You are going to need to allocate all
 * of the necessary structs and buffers, and then initialize all of
 * the necessary fields (head, size, readers, writers, and the locks
 * and queues.)
 */
static pipe_t *
pipe_create(void)
{
        NOT_YET_IMPLEMENTED("PIPES: pipe_create");
        return NULL;
}

/*
 * Free all necessary memory.
 */
static void
pipe_destroy(pipe_t *pipe)
{
        NOT_YET_IMPLEMENTED("PIPES: pipe_destroy");
}

/* pipefs vnode operations */
static void
pipe_read_vnode(vnode_t *vnode)
{
        vnode->vn_ops = &pipe_vops;
        vnode->vn_mode = S_IFIFO;
        vnode->vn_len = 0;
        vnode->vn_i = NULL;
}

static void
pipe_delete_vnode(vnode_t *vnode)
{
        pipe_t *p = VNODE_TO_PIPE(vnode);
        if (p) {
                pipe_destroy(p);
        }
}

static int
pipe_query_vnode(vnode_t *vnode)
{
        /*
         * Since we are not using the VM subsystem for
         * this type of vnode, there is no reason for
         * us to return 0 -- there will never be pages
         * to clean up.
         */
        return 1;
}

/*
 * Gets a new vnode representing a pipe. The reason
 * why we don't just do this setup in pipe_read_vnode
 * is that the creation of the pipe data might fail, since
 * there is memory allocation going on in there. Thus,
 * we split it into two steps, the first of which relies on
 * pipe_read_vnode to do some setup, and then the pipe_create
 * call, at which point we can safely vput the allocated
 * vnode if pipe_create fails.
 */
static vnode_t *
pget(void)
{
        NOT_YET_IMPLEMENTED("PIPES: pget");
        return NULL;
}

/*
 * An implementation of the pipe(2) system call. You really
 * only have to worry about a few things:
 *   o Running out of memory when allocating the vnode, at which
 *     point you should fail with ENOMEM;
 *   o Running out of file descriptors, in which case you should
 *     fail with EMFILE.
 * Once all of the structures are set up, just put the read-end
 * file descriptor of the pipe into pipefd[0], and the write-end
 * descriptor into pipefd[1].
 */
int
do_pipe(int pipefd[2])
{
        NOT_YET_IMPLEMENTED("PIPES: do_pipe");
        return -ENOTSUP;
}

/*
 * When reading from a pipe, you should make sure there are enough characters in
 * the buffer to read. If there are, grab them and move up the tail by subtracting
 * from size. offset is ignored. Also, remember to take the reader lock to prevent
 * other threads from reading while you are waiting for more characters.
 *
 * This might block, e.g. if there are no or not enough characters to read.
 * It might be the case that there are no more writers and we aren't done reading.
 * However, in situations like this, there is no way to open the pipe for writing
 * again so no more writers will ever put characters in the pipe. The reader should
 * just take as much as it needs (or barring that, as much as it can get) and
 * return with a partial buffer.
 */
static int
pipe_read(vnode_t *vnode, off_t offset, void *buf, size_t len)
{
        NOT_YET_IMPLEMENTED("PIPES: pipe_read");
        return -EINVAL;
}

/*
 * Writing to a pipe is the dual of reading: if there is room, we can write our
 * data and go, but if not, we have to wait until there is more room and alert
 * any potential readers. Like above, you should take the writer lock to make
 * sure your write is contiguous.
 *
 * If there are no more readers, we have a broken pipe, and should fail with
 * the EPIPE error number.
 */
static int
pipe_write(vnode_t *vnode, off_t offset, const void *buf, size_t len)
{
        NOT_YET_IMPLEMENTED("PIPES: pipe_write");
        return -EINVAL;
}

/*
 * It's still possible to stat a pipe using the fstat call, which takes a file descriptor.
 * Pipes don't have too much information, though. The only ones that matter here are
 * st_mode and st_ino, though you want to zero out some of the others.
 */
static int
pipe_stat(vnode_t *vnode, struct stat *ss)
{
        NOT_YET_IMPLEMENTED("PIPES: pipe_stat");
        return -EINVAL;
}

/*
 * If someone is opening the read end of the pipe, we need to increment
 * the reader count, and the same for the writer count if a file open
 * for writing is acquiring this vnode. This count needs to be accurate
 * for correct reading and writing behavior.
 */
static int
pipe_acquire(vnode_t *vnode, file_t *file)
{
        NOT_YET_IMPLEMENTED("PIPES: pipe_acquire");
        return 0;
}

/*
 * Subtract from the reader or writer count as necessary here. If either
 * count hits zero, you are going to need to wake up the other group of
 * threads so they can either return with their partial read or notice
 * the broken pipe.
 */
static int
pipe_release(vnode_t *vnode, file_t *file)
{
        NOT_YET_IMPLEMENTED("PIPES: pipe_release");
        return 0;
}
