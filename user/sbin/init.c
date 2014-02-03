/*
 * Forks a shell for each terminal and waits for them.
 * This is the final thing you should be executing
 * (with kernel_execve) in kernel-land once everything works.
 */


#include <sys/types.h>
#include <errno.h>
#include <stdlib.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>
#include <fcntl.h>
#include <dirent.h>

char *empty[] = { NULL };

const char      *hi = "init: starting shell on ";
const char      *sh = "/bin/sh";
const char      *ttystr = "tty";
const char      *home = "/";
const char      *alldone = "init: no remaining processes\n";

static int open_tty(char *tty)
{
        if (-1 == open(tty, O_RDONLY, 0)) {
                return -1;
        } else if (-1 == open(tty, O_WRONLY, 0)) {
                return -1;
        } else if (2 != dup(1)) {
                return -1;
        } else {
                return 0;
        }
}

static void spawn_shell_on(char *tty)
{
        if (!fork()) {
                close(0);
                close(1);
                close(2);
                if (-1 == open_tty(tty)) {
                        exit(1);
                }

                chdir(home);

                printf(hi);
                printf(tty);
                printf("\n");

                execve(sh, empty, empty);
                fprintf(stderr, "exec failed!\n");
        }
}

int main(int argc, char **argv, char **envp)
{
        int      devdir, ii;
        dirent_t d;
        int      status;

        for (ii = 0; ii < NFILES; ii++)
                close(ii);
        ii  = ii;

        if (-1 == open_tty("/dev/tty0")) {
                exit(1);
        }

        chdir("/dev");

        devdir = open("/dev", O_RDONLY, 0);
        while (getdents(devdir, &d, sizeof(d)) > 0) {
                if (0 == strncmp(d.d_name, ttystr, strlen(ttystr))) {
                        spawn_shell_on(d.d_name);
                }
        }
        close(devdir);

        int pid;
        while (0 <= (pid = wait(&status))) {
                if (EFAULT == status) {
                        printf("process %i faulted\n", pid);
                }
        }

        if (ECHILD != errno) {
                printf("error: wait: %s\n", strerror(errno));
                return 1;
        } else {
                printf(alldone);
                return 0;
        }
}
