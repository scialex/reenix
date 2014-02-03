#include "util/debug.h"

#include "main/interrupt.h"
#include "main/gdt.h"

#include "api/exec.h"
#include "api/binfmt.h"
#include "api/syscall.h"


/* Enters userland from the kernel. Call this for a process that has up to now
 * been a kernel-only process. Takes the registers to start userland execution
 * with. Does not return. Note that the regs passed in should be on the current
 * stack of execution.
 */
void userland_entry(const regs_t *regs)
{
        intr_disable();
        intr_setipl(IPL_LOW);
        /* We "return from the interrupt" to get into userland */
        __asm__ __volatile__(
                "movl %%eax, %%esp\n\t" /* Move stack pointer up to regs */
                "pop %%es\n\t"          /* Set userland data and extra segment appropriately */
                "pop %%ds\n\t"
                "popa\n\t"
                "add $8, %%esp\n\t"     /*
                                         * Move stack pointer up to the location of the
                                         * arguments automatically pushed by the processor
                                         * on an interrupt
                                         */
                "iret\n"
                /* We're now in userland! */
                : /* No outputs */
                : "a"(regs)
        );
}

int do_execve(const char *filename, char *const *argv, char *const *envp, struct regs *regs)
{
        uint32_t eip, esp;
        int ret = binfmt_load(filename, argv, envp, &eip, &esp);
        if (ret < 0) {
                return ret;
        }
        /* Make sure we "return" into the start of the newly loaded binary */
        regs->r_eip = eip;
        regs->r_useresp = esp;
        return 0;
}

void kernel_execve(const char *filename, char *const *argv, char *const *envp)
{
        uint32_t eip, esp;
        int ret = binfmt_load(filename, argv, envp, &eip, &esp);
        KASSERT(0 == ret); /* Should never fail to load the first binary */

        dbg(DBG_EXEC, "Entering userland with eip %#08x, esp %#08x\n", eip, esp);

        /* To enter userland, we build a set of saved registers to "trick" the processor
         * into thinking we were in userland before. Yes, it's horrible. c.f.
         * http://wiki.osdev.org/index.php?title=Getting_to_Ring_3&oldid=8195 */
        regs_t regs;
        /* Userland gdt entries (0x3 for ring 3) */
        regs.r_cs = GDT_USER_TEXT | 0x3;
        regs.r_ss = GDT_USER_DATA | 0x3;
        regs.r_ds = regs.r_ss;
        regs.r_es = regs.r_ss;

        /* Userland instruction pointer and stack pointer */
        regs.r_eip = eip;
        regs.r_useresp = esp;

#if 0
        uint32_t eflags;
        /* Get the current eflags register */
        __asm__ volatile(
                "pushfl\n\t"
                "popl %%eax"
                : "=a"(eflags)
        );

        regs.r_eflags = eflags;
#endif
        regs.r_eflags = 0x202;
        /* Zero everything else, just to be safe */
        regs.r_err = 0;
        regs.r_intr = 0;
        regs.r_edi = 0;
        regs.r_esi = 0;
        regs.r_eax = 0;
        regs.r_ebx = 0;
        regs.r_ecx = 0;
        regs.r_edx = 0;
        regs.r_ebp = 0;
        regs.r_esp = 0;
        userland_entry(&regs);
}
