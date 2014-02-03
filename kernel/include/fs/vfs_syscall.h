#pragma once

#include "dirent.h"

#include "types.h"

#include "fs/open.h"
#include "fs/pipe.h"
#include "fs/stat.h"

int do_close(int fd);
int do_read(int fd, void *buf, size_t nbytes);
int do_write(int fd, const void *buf, size_t nbytes);
int do_dup(int fd);
int do_dup2(int ofd, int nfd);
int do_mknod(const char *path, int mode, unsigned devid);
int do_mkdir(const char *path);
int do_rmdir(const char *path);
int do_unlink(const char *path);
int do_link(const char *from, const char *to);
int do_rename(const char *oldname, const char *newname);
int do_chdir(const char *path);
int do_getdent(int fd, struct dirent *dirp);
int do_lseek(int fd, int offset, int whence);
int do_stat(const char *path, struct stat *uf);

#ifdef __MOUNTING__
/* for mounting implementations only, not required */
int do_mount(const char *source, const char *target, const char *type);
int do_umount(const char *target);
#endif
