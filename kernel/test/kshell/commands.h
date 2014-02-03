#pragma once

#include "test/kshell/kshell.h"

#define KSHELL_CMD(name) \
        int kshell_ ## name(kshell_t *ksh, int argc, char **argv)

KSHELL_CMD(help);
KSHELL_CMD(exit);
KSHELL_CMD(echo);
#ifdef __VFS__
KSHELL_CMD(cat);
KSHELL_CMD(ls);
KSHELL_CMD(cd);
KSHELL_CMD(rm);
KSHELL_CMD(link);
KSHELL_CMD(rmdir);
KSHELL_CMD(mkdir);
KSHELL_CMD(stat);
#endif
