#include <stdio.h>
#include <unistd.h>
#include <dirent.h>
#include <fcntl.h>
#include <sys/mman.h>
#include <sys/stat.h>

#include <errno.h>

static int do_ls(const char *dir)
{
        int             fd;
        struct dirent   *dirent;
        int             nbytes;
        char            tmpbuf[256];
        struct stat     sbuf;

        union {
                struct dirent   dirent;
                char            buf[4096];
        } lsb;

        fd = open(dir, O_RDONLY, 0600);
        if (fd < 0) {
                fprintf(stderr,
                        "ls: unable to open \"%s\": errno %d\n",
                        dir, errno);
                return 1;
        }

        while ((nbytes = getdents(fd, &lsb.dirent, sizeof(lsb))) > 0) {
                dirent = &lsb.dirent;

                if (nbytes % sizeof(struct dirent)) {
                        fprintf(stderr,
                                "ls: incorrect return value from getdents (%d):"
                                " not a multiple of sizeof(struct dirent) (%d)\n",
                                nbytes, sizeof(struct dirent));
                        return 1;
                }
                do {
                        int reclen;
                        int size;

                        snprintf(tmpbuf, 256, "%s/%s", dir, dirent->d_name);
                        if (0 == stat(tmpbuf, &sbuf))
                                size = sbuf.st_size;
                        else
                                size = 0;

                        reclen = sizeof(struct dirent);
                        fprintf(stdout, "%7d  %-20s   %d\n",
                                size, dirent->d_name, dirent->d_ino);
                        dirent = (struct dirent *)(((char *)dirent) + reclen);
                        nbytes -= reclen;
                } while (nbytes);
        }
        if (nbytes < 0) {
                if (errno == ENOTDIR)
                        fprintf(stdout, "%s\n", dir);
                else
                        fprintf(stderr,
                                "ls: couldn't list %s: errno %d\n",
                                dir, errno);
        }

        if (close(fd) < 0)
                fprintf(stderr, "ls: close %s: errno %d\n",
                        dir, errno);
        return 0;
}


int main(int argc, char **argv)
{
        int ret;

        if (argc < 2)
                ret = do_ls(".");
        else if (argc < 3)
                ret = do_ls(argv[1]);
        else {
                int error;
                int argn;

                error = 0;
                for (argn = 1; argn < argc; argn++) {
                        fprintf(stdout, "%s:\n", argv[argn]);
                        error += do_ls(argv[argn]);
                        fprintf(stdout, "\n");
                }
                ret = error;
        }
        return ret;
}
