/* entry.c */
#include "main/entry.h"

/* This is the first C function ever called.
 * it gets called from the boot loader assembly */
void entry()
{
        kmain2();
        __asm__("cli\n\t"
                "hlt");
}
