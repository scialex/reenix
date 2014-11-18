/* entry.c */

void kmain(void);

/* This is the first C function ever called.
 * it gets called from the boot loader assembly */
void entry()
{
        kmain();
        __asm__("cli\n\t"
                "hlt");
}
