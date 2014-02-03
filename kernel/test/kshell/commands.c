#include "commands.h"

#include "command.h"
#include "errno.h"
#include "priv.h"

#ifdef __VFS__
#include "fs/fcntl.h"
#include "fs/file.h"
#include "fs/vfs_syscall.h"
#include "fs/vnode.h"
#endif

#include "test/kshell/io.h"

#include "util/debug.h"
#include "util/string.h"

int kshell_help(kshell_t *ksh, int argc, char **argv)
{
        /* Print a list of available commands */
        int i;

        kshell_command_t *cmd;
        char spaces[KSH_CMD_NAME_LEN];
        memset(spaces, ' ', KSH_CMD_NAME_LEN);

        kprintf(ksh, "Available commands:\n");
        list_iterate_begin(&kshell_commands_list, cmd, kshell_command_t,
                           kc_commands_link) {
                KASSERT(NULL != cmd);
                int namelen = strnlen(cmd->kc_name, KSH_CMD_NAME_LEN);
                spaces[KSH_CMD_NAME_LEN - namelen] = '\0';
                kprintf(ksh, "%s%s%s\n", cmd->kc_name, spaces, cmd->kc_desc);
                spaces[KSH_CMD_NAME_LEN - namelen] = ' ';
        } list_iterate_end();

        return 0;
}

int kshell_exit(kshell_t *ksh, int argc, char **argv)
{
        panic("kshell: kshell_exit should NEVER be called");
        return 0;
}

int kshell_echo(kshell_t *ksh, int argc, char **argv)
{
        if (argc == 1) {
                kprintf(ksh, "\n");
        } else {
                int i;

                for (i = 1; i < argc - 1; ++i) {
                        kprintf(ksh, "%s ", argv[i]);
                }
                kprintf(ksh, "%s\n", argv[argc - 1]);
        }

        return 0;
}

#ifdef __VFS__
int kshell_cat(kshell_t *ksh, int argc, char **argv)
{
        if (argc < 2) {
                kprintf(ksh, "Usage: cat <files>\n");
                return 0;
        }

        char buf[KSH_BUF_SIZE];
        int i;
        for (i = 1; i < argc; ++i) {
                int fd, retval;

                if ((fd = do_open(argv[i], O_RDONLY)) < 0) {
                        kprintf(ksh, "Error opening file: %s\n", argv[i]);
                        continue;
                }

                while ((retval = do_read(fd, buf, KSH_BUF_SIZE)) > 0) {
                        retval = kshell_write_all(ksh, buf, retval);
                        if (retval < 0) break;
                }
                if (retval < 0) {
                        kprintf(ksh, "Error reading or writing %s: %d\n", argv[i], retval);
                }

                if ((retval = do_close(fd)) < 0) {
                        panic("kshell: Error closing file: %s\n", argv[i]);
                }
        }

        return 0;
}

int kshell_ls(kshell_t *ksh, int argc, char **argv)
{
        int arglen, ret, fd;
        dirent_t dirent;
        struct stat statbuf;
        char direntname[KSH_BUF_SIZE];

        memset(direntname, '\0', KSH_BUF_SIZE);

        if (argc > 3) {
                kprintf(ksh, "Usage: ls <directory>\n");
                return 0;
        } else if (argc == 2) {
                if ((ret = do_stat(argv[1], &statbuf)) < 0) {
                        if (ret == -ENOENT) {
                                kprintf(ksh, "%s does not exist\n", argv[1]);
                                return 0;
                        } else {
                                return ret;
                        }
                }
                if (!S_ISDIR(statbuf.st_mode)) {
                        kprintf(ksh, "%s is not a directory\n", argv[1]);
                        return 0;
                }

                if ((fd = do_open(argv[1], O_RDONLY)) < 0) {
                        kprintf(ksh, "Could not find directory: %s\n", argv[1]);
                        return 0;
                }
                arglen = strnlen(argv[1], KSH_BUF_SIZE);
        } else {
                KASSERT(argc == 1);
                if ((fd = do_open(".", O_RDONLY)) < 0) {
                        kprintf(ksh, "Could not find directory: .\n");
                        return 0;
                }
                arglen = 1;
        }

        if (argc == 2)
                memcpy(direntname, argv[1], arglen);
        else
                direntname[0] = '.';

        direntname[arglen] = '/';
        direntname[arglen + NAME_LEN + 1] = '\0';

        while ((ret = do_getdent(fd, &dirent)) == sizeof(dirent_t)) {
                memcpy(direntname + arglen + 1, dirent.d_name, NAME_LEN + 1);
                if ((ret = do_stat(direntname, &statbuf)) < 0) {
                        kprintf(ksh, "Error stat\'ing %s\n", dirent.d_name);
                        continue;
                }
                if (S_ISDIR(statbuf.st_mode)) {
                        kprintf(ksh, "%s/\n", dirent.d_name);
                } else {
                        kprintf(ksh, "%s\n", dirent.d_name);
                }
        }

        do_close(fd);
        return ret;
}

int kshell_cd(kshell_t *ksh, int argc, char **argv)
{
        KASSERT(NULL != ksh);

        int ret;

        if (argc < 2) {
                kprintf(ksh, "Usage: cd <directory>\n");
                return 0;
        }

        if ((ret = do_chdir(argv[1])) < 0) {
                kprintf(ksh, "Error cd\'ing into %s\n", argv[1]);
        }
        return 0;
}

int kshell_rm(kshell_t *ksh, int argc, char **argv)
{
        KASSERT(NULL != ksh);

        int ret;

        if (argc < 2) {
                kprintf(ksh, "Usage: rm <file>\n");
                return 0;
        }

        if ((ret = do_unlink(argv[1])) < 0) {
                kprintf(ksh, "Error unlinking %s\n", argv[1]);
        }

        return 0;
}

int kshell_link(kshell_t *ksh, int argc, char **argv)
{
        KASSERT(NULL != ksh);

        int ret;

        if (argc < 3) {
                kprintf(ksh, "Usage: link <src> <dst>\n");
                return 0;
        }

        if ((ret = do_link(argv[1], argv[2])) < 0) {
                kprintf(ksh, "Error linking %s to %s: %d\n", argv[1], argv[2], ret);
        }

        return 0;
}

int kshell_rmdir(kshell_t *ksh, int argc, char **argv)
{
        KASSERT(NULL != ksh);
        KASSERT(NULL != argv);

        int i;
        int exit_val;
        int ret;

        if (argc < 2) {
                kprintf(ksh, "Usage: rmdir DIRECTORY...\n");
                return 1;
        }

        exit_val = 0;
        for (i = 1; i < argc; ++i) {
                if ((ret = do_rmdir(argv[i])) < 0) {
                        char *errstr = strerror(-ret);
                        kprintf(ksh, "rmdir: failed to remove `%s': %s\n",
                                argv[i], errstr);
                        exit_val = 1;
                }
        }

        return exit_val;
}

int kshell_mkdir(kshell_t *ksh, int argc, char **argv)
{
        KASSERT(NULL != ksh);
        KASSERT(NULL != argv);

        int i;
        int exit_val;
        int ret;

        if (argc < 2) {
                kprintf(ksh, "Usage: mkdir DIRECTORY...\n");
                return 1;
        }

        exit_val = 0;
        for (i = 1; i < argc; ++i) {
                if ((ret = do_mkdir(argv[i])) < 0) {
                        char *errstr = strerror(-ret);
                        kprintf(ksh,
                                "mkdir: cannot create directory `%s': %s\n",
                                argv[i], errstr);
                        exit_val = 1;
                }
        }

        return exit_val;
}

static const char *get_file_type_str(int mode)
{
        if (S_ISCHR(mode)) {
                return "character special file";
        } else if (S_ISDIR(mode)) {
                return "directory";
        } else if (S_ISBLK(mode)) {
                return "block special file";
        } else if (S_ISREG(mode)) {
                return "regular file";
        } else if (S_ISLNK(mode)) {
                return "symbolic link";
        } else {
                return "unknown";
        }
}

int kshell_stat(kshell_t *ksh, int argc, char **argv)
{
        KASSERT(NULL != ksh);
        KASSERT(NULL != argv);

        int i;
        int exit_val = 0;
        int ret;
        struct stat buf;

        if (argc < 2) {
                kprintf(ksh, "Usage: stat FILE...\n");
                return 1;
        }

        for (i = 1; i < argc; ++i) {
                if ((ret = do_stat(argv[i], &buf)) < 0) {
                        char *errstr = strerror(-ret);
                        kprintf(ksh, "Cannot state `%s': %s\n",
                                argv[i], errstr);
                        exit_val = 1;
                } else {
                        const char *file_type_str =
                                get_file_type_str(buf.st_mode);
                        kprintf(ksh, "File: `%s'\n", argv[i]);
                        kprintf(ksh, "Size: %d\n", buf.st_size);
                        kprintf(ksh, "Blocks: %d\n", buf.st_blocks);
                        kprintf(ksh, "IO Block: %d\n", buf.st_blksize);
                        kprintf(ksh, "%s\n", file_type_str);
                        kprintf(ksh, "Inode: %d\n", buf.st_ino);
                        kprintf(ksh, "Links: %d\n", buf.st_nlink);
                }
        }

        return exit_val;
}
#endif
