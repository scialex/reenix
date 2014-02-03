/*
 * Runs the in-kernel shell.
 */

#include "weenix/syscall.h"
#include "weenix/trap.h"

int main(int argc, char **argv)
{
        return trap(SYS_kshell, (uint32_t)0);
}
