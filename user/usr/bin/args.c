/*
 * Does some basic checks to make sure arguments are
 * being passed to userland programs correctly.
 */

#include <unistd.h>
#include <fcntl.h>
#include <string.h>
#include <stdio.h>

int main(int argc, char **argv, char **envp)
{
        int     i;
        char buf[100];

        open("/dev/tty0", O_RDONLY, 0);
        open("/dev/tty0", O_WRONLY, 0);

        sprintf(buf, "Arguments: (argc = %d, argv = %p)\n", argc, argv);
        write(1, buf, strlen(buf));
        for (i = 0; argv[i]; i++) {
                sprintf(buf, "  %d \"%s\"\n", i, argv[i]);
                write(1, buf, strlen(buf));
        }
        sprintf(buf, "Environment: (envp = %p)\n", envp);
        write(1, buf, strlen(buf));
        for (i = 0; envp[i]; i++) {
                sprintf(buf, "  %d \"%s\"\n", i, envp[i]);
                write(1, buf, strlen(buf));
        }

        return 0;
}
