#include <stdio.h>
#include <fcntl.h>
#include <unistd.h>
#include <stdlib.h>

/* TODO add options for different ways of forkbombing
   (kind of low priority but would be fun) */

int main(int argc, char **argv)
{
        int n = 1;
        pid_t pid;

        open("/dev/tty0", O_RDONLY, 0);
        open("/dev/tty0", O_WRONLY, 0);
        printf("Forking up a storm!\n");
        printf("If this runs for 10 minutes without crashing, then you ");
        printf("probably aren't \nleaking resources\n");
        if (!fork()) {
                for (;;) {
                        printf("I am fork number %d\n", n);
                        if ((pid = fork())) {
                                /* parent */
                                /* pid should be > 2 or pid should be -1 if
                                 * the fork failed */
                                if (-1 != pid) {
                                        exit(0);
                                } else {
                                        printf("%d-th fork failed. "
                                               "forkbomb stopping.", n);
                                        exit(1);
                                }
                        }
                        ++n;
                }
        } else {
                int status;
                while (wait(&status) > 0)
                        ;
        }
        return 0;
}
