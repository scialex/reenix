#pragma once

/* "kernel" utility things */

/* fprintf */
#define fprintf(fd, fmt, args...) dbg(DBG_TEST, fmt, ## args)
#define printf(fmt, args...) dbg(DBG_TEST, fmt, ## args)

/* errno */
#define errno (curthr->kt_errno)

/* malloc/free */
#define malloc kmalloc
#define free kfree

/* The "kernel" system calls */
#define ksyscall(name, formal, actual)          \
        static int ksys_ ## name formal {       \
                int ret = do_ ## name actual ;  \
                if(ret < 0) {                   \
                        errno = -ret;           \
                        return -1;              \
                }                               \
                return ret;                     \
        }

ksyscall(close, (int fd), (fd))
ksyscall(read, (int fd, void *buf, size_t nbytes), (fd, buf, nbytes))
ksyscall(write, (int fd, const void *buf, size_t nbytes), (fd, buf, nbytes))
ksyscall(dup, (int fd), (fd))
ksyscall(dup2, (int ofd, int nfd), (ofd, nfd))
ksyscall(mkdir, (const char *path), (path))
ksyscall(rmdir, (const char *path), (path))
ksyscall(link, (const char *old, const char *new), (old, new))
ksyscall(unlink, (const char *path), (path))
ksyscall(rename, (const char *oldname, const char *newname), (oldname, newname))
ksyscall(chdir, (const char *path), (path))
ksyscall(lseek, (int fd, int offset, int whence), (fd, offset, whence))
ksyscall(getdent, (int fd, struct dirent *dirp), (fd, dirp))
ksyscall(stat, (const char *path, struct stat *uf), (path, uf))
ksyscall(open, (const char *filename, int flags), (filename, flags))
#define ksys_exit do_exit

/* Kill me now */
int ksys_getdents(int fd, struct dirent *dirp, unsigned int count)
{
        size_t numbytesread = 0;
        int nbr = 0;
        dirent_t tempdirent;

        if (count < sizeof(dirent_t)) {
                curthr->kt_errno = EINVAL;
                return -1;
        }

        while (numbytesread < count) {
                if ((nbr = do_getdent(fd, &tempdirent)) < 0) {
                        curthr->kt_errno = -nbr;
                        return -1;
                }
                if (nbr == 0) {
                        return numbytesread;
                }
                memcpy(dirp, &tempdirent, sizeof(dirent_t));

                KASSERT(nbr == sizeof(dirent_t));

                dirp++;
                numbytesread += nbr;
        }
        return numbytesread;
}

/*
 * Redirect system calls to kernel system calls.
 */
#define mkdir(a,b)      ksys_mkdir(a)
#define rmdir           ksys_rmdir
#define mount           ksys_mount
#define umount          ksys_umount
#define open(a,b,c)     ksys_open(a,b)
#define close           ksys_close
#define link            ksys_link
#define rename          ksys_rename
#define unlink          ksys_unlink
#define read            ksys_read
#define write           ksys_write
#define lseek           ksys_lseek
#define dup             ksys_dup
#define dup2            ksys_dup2
#define chdir           ksys_chdir
#define stat(a,b)       ksys_stat(a,b)
#define getdents(a,b,c) ksys_getdents(a,b,c)
#define exit(a)         ksys_exit(a)

/* Random numbers */
/* Random int between lo and hi inclusive */
#define RAND_MAX INT_MAX
#define RANDOM(lo,hi) ((lo)+(((hi)-(lo)+1)*(randseed = (randseed*4096+150889)%714025))/714025)

static unsigned long long randseed = 123456L;

static int rand(void)
{
        randseed = (randseed * 4096 + 150889) % RAND_MAX;
        return randseed;
}

static void srand(unsigned int seed)
{
        randseed = seed;
}
