#include "config.h"

#include "proc/context.h"
#include "proc/kthread.h"

#include "main/apic.h"
#include "main/interrupt.h"
#include "main/gdt.h"

#include "mm/page.h"
#include "mm/pagetable.h"

#include "util/debug.h"

static void
__context_initial_func(context_func_t func, int arg1, void *arg2)
{
        apic_setipl(IPL_LOW);
        intr_enable();

        void *result = func(arg1, arg2);
        kthread_exit(result);

        panic("\nReturned from kthread_exit.\n");
}

void
context_setup(context_t *c, context_func_t func, int arg1, void *arg2,
              void *kstack, size_t kstacksz, pagedir_t *pdptr)
{
        KASSERT(NULL != pdptr);
        KASSERT(PAGE_ALIGNED(kstack));

        c->c_kstack = (uintptr_t)kstack;
        c->c_kstacksz = kstacksz;
        c->c_pdptr = pdptr;

        /* put the arguments for __contect_initial_func onto the
         * stack, leave room at the bottom of the stack for a phony
         * return address (we should never return from the lowest
         * function on the stack */
        c->c_esp = (uintptr_t)kstack + kstacksz;
        c->c_esp -= sizeof(arg2);
        *(void **)c->c_esp = arg2;
        c->c_esp -= sizeof(arg1);
        *(int *)c->c_esp = arg1;
        c->c_esp -= sizeof(context_func_t);
        *(context_func_t *)c->c_esp = func;
        c->c_esp -= sizeof(uintptr_t);

        c->c_ebp = c->c_esp;
        c->c_eip = (uintptr_t)__context_initial_func;
}

void
context_make_active(context_t *c)
{
        gdt_set_kernel_stack((void *)((uintptr_t)c->c_kstack + c->c_kstacksz));
        pt_set(c->c_pdptr);

        /* Switch stacks and run the thread */
        __asm__ volatile(
                "movl %0,%%ebp\n\t"     /* update ebp */
                "movl %1,%%esp\n\t"     /* update esp */
                "push %2\n\t"           /* save eip   */
                "ret"                   /* jump to new eip */
                :: "m"(c->c_ebp), "m"(c->c_esp), "m"(c->c_eip)
        );
}

void
context_switch(context_t *oldc, context_t *newc)
{
        gdt_set_kernel_stack((void *)((uintptr_t)newc->c_kstack + newc->c_kstacksz));
        pt_set(newc->c_pdptr);

        /*
         * Save the current value of the stack pointer and the frame pointer into
         * the old context. Set the instruction pointer to the return address
         * (whoever called us).
         */
        __asm__ __volatile__(
                "pushfl           \n\t" /* save EFLAGS on the stack */
                "pushl  %%ebp     \n\t"
                "movl   %%esp, %0 \n\t" /* save ESP into oldc */
                "movl   %2, %%esp \n\t" /* restore ESP from newc */
                "movl   $1f, %1   \n\t" /* save EIP into oldc */
                "pushl  %3        \n\t" /* restore EIP */
                "ret              \n\t"
                "1:\t"                  /* this is where oldc starts executing later */
                "popl   %%ebp     \n\t"
                "popfl"                 /* restore EFLAGS */
                :"=m"(oldc->c_esp), "=m"(oldc->c_eip)
                :"m"(newc->c_esp), "m"(newc->c_eip)
        );
}
