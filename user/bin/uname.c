/*
 *   FILE: uname.c
 * AUTHOR: kma
 *  DESCR: uname. whee!
 */

#include <unistd.h>
#include <sys/utsname.h>
#include <stdio.h>

char *TAS = "weenix brought to you by:\n"
            "1998: jal, dep, kma, mcc, cd, tor\n"
            "1999: mbe, tc, kma, mahrens, tor\n"
            "2000: ahl, mahrens, mba, pdemoreu, it\n"
            "2001: pgriess, pdemoreu, gc3, rmanches, dg\n"
            "2002: kit, eschrock\n"
            "2005: afenn\n"
            "2006: afenn\n"
            "2007: dap, joel\n"
            "and the number 0xe\n";

static int doflag(int c);
static struct utsname un;

int main(int argc, char **argv)
{
        int     ii;

        uname(&un);

        for (ii = 1; ii < argc ; ii++) {
                if (argv[ii][0] == '-') {
                        char *str;
                        str = &argv[ii][1];
                        while (*str) {
                                if (doflag(*str++) < 0)
                                        goto usage;
                        }
                }
        }

        if (argc == 1)
                doflag('s');
        fprintf(stdout, "\n");
        return 0;

usage:
        return 1;
}

static int doflag(int c)
{
        switch (c) {
                case 'a':
                        printf("%s", TAS);
                        printf("%s ", un.sysname);
                        printf("%s ", un.nodename);
                        printf("%s ", un.release);
                        printf("%s ", un.version);
                        printf("%s ", un.machine);
                        break;
                case 's':
                        printf("%s", un.sysname);
                        break;
                case 'n':
                        printf("%s", un.nodename);
                        break;
                case 'r':
                        printf("%s", un.release);
                        break;
                case 'T':
                        printf("%s", TAS);
                        break;
                case 'v':
                        printf("%s", un.version);
                        break;
                case 'm':
                        printf("%s", un.machine);
                        break;
                default:
                        return -1;
        }
        return 0;
}
