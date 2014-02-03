/*
 *  File: bs_i386.S
 *  Date: Oct 22 2002
 *  Acct: Rob Manchester (rmanches)
 *  Desc: x86 elf runtime linker
 */

#include "asm.h"

#if 0
#ifdef __PIC__
#undef __i686
.globl __i686.get_pc_thunk.bx
__i686.get_pc_thunk.bx:
	movl (%esp), %ebx
	ret
#define __i686 1
#endif
#endif

#if 0
ENTRY(_bootstrap)
        xorl    %ebp, %ebp
        movl    %ebp, (%esp)            # set our return address to 0x0
        subl    $12, %esp               # make some room for args.
        call     _ldloadrtld            # do the dead
        call    *%eax                   # call main
        addl    $12, %esp               # get rid of the args.
        pushl   %eax                    # push return number
        call    exit                    # should never return.
        ret                             # we return to 0x0
END(_bootstrap)
#endif
.globl _bootstrap


/* TODO: fix the call to _ldloadrtld to use PLT or something (right now
 * it only works b/c it is a near call */

_bootstrap:
		addl    $4, %esp               # Get rid of junk return address
		call    _ldloadrtld            # Dynamic linking!!!
		call    *%eax                  # Call executable main
		pushl   %eax                   # Push return of executable
		call    _ldcleanup             # and exit

/*-
 * stolen from freebsd /src/libexec/rtld-elf/i386/rtld_start.S
 *
 * Copyright 1996-1998 John D. Polstra.
 * All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 *
 * THIS SOFTWARE IS PROVIDED BY THE AUTHOR ``AS IS'' AND ANY EXPRESS OR
 * IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES
 * OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED.
 * IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY DIRECT, INDIRECT,
 * INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT
 * NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE,
 * DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY
 * THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
 * (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF
 * THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 *
 */

ENTRY(_ld_bind)
        pushf                           # Save eflags
        pushl   %eax                    # Save %eax
        pushl   %edx                    # Save %edx
        pushl   %ecx                    # Save %ecx
        pushl   20(%esp)                # Copy reloff argument
        pushl   20(%esp)                # Copy module argument

        call    _rtresolve@PLT          # Transfer control to the binder
        /* Now %eax contains the entry point of the function being called. */


        addl    $8,%esp                 # Discard binder arguments
        movl    %eax,20(%esp)           # Store target over obj argument
        pushl   %ebx
        popl    %ebx
        popl    %ecx                    # Restore %ecx
        popl    %edx                    # Restore %edx
        popl    %eax                    # Restore %eax
        popf                            # Restore eflags
        leal    4(%esp),%esp            # Discard reloff, do not change eflags
        ret                             # "Return" to target address
END(_ld_bind)
