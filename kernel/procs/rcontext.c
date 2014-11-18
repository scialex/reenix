#include "types.h"

struct rustctx {
    uintptr_t c_eip;
    uintptr_t c_esp;
    uintptr_t c_ebp;
};

/* This is used in Rust to get around the fact that LLVM doesn't seem to like the inline assembly for some reason. */
void do_real_context_switch(struct rustctx* oldc, struct rustctx* newc) {
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
