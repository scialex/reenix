/*
 * taken from glibc.
 */

/* Assembler macros for i386.
   Copyright (C) 1991, 92, 93, 95, 96, 98 Free Software Foundation, Inc.
   This file is part of the GNU C Library.

   The GNU C Library is free software; you can redistribute it and/or
   modify it under the terms of the GNU Lesser General Public
   License as published by the Free Software Foundation; either
   version 2.1 of the License, or (at your option) any later version.

   The GNU C Library is distributed in the hope that it will be useful,
   but WITHOUT ANY WARRANTY; without even the implied warranty of
   MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU
   Lesser General Public License for more details.

   You should have received a copy of the GNU Lesser General Public
   License along with the GNU C Library; if not, write to the Free
   Software Foundation, Inc., 59 Temple Place, Suite 330, Boston, MA
   02111-1307 USA.  */

#ifdef  __ASSEMBLY__

/* FIXME akerber: Modified a couple macros, old versions commented out, not sure
 * if this new version actually works */

#define L(name) .L##name

#define ENTRY(name)                                             \
        .text;                                                  \
        .align ALIGNARG(4);                                     \
        STABS_CURRENT_FILE1("")                                 \
        STABS_CURRENT_FILE(name)                                \
        STABS_FUN(name)                                         \
        .globl name;                                            \
        .type name,@function;                                   \
        name:
/*      name##: */

#define END(name)                                               \
        .size name,.-name;                                      \
        STABS_FUN_END(name)                                     \
         

#define ALIGNARG(log2) (1<<log2)

#define STABS_CURRENT_FILE(name)                                \
        STABS_CURRENT_FILE1 (#name)

#define STABS_CURRENT_FILE1(name)                               \
        1: .stabs name,100,0,0,1b;

#define STABS_FUN_END(name)                                     \
        1: .stabs "",36,0,0,1b-name;

#define STABS_FUN(name)                                         \
        STABS_FUN2(name, name:F(0,1))
/*      STABS_FUN2(name, name##:F(0,1)) */

#define STABS_FUN2(name, namestr)                               \
        .stabs "int:t(0,1)=r(0,1);-2147483648;2147483647;",128,0,0,0; \
        .stabs #namestr,36,0,0,name;

#ifdef __PIC__
#define JUMPTARGET(name)        name##@PLT
#define SYSCALL_PIC_SETUP                                       \
        pushl %ebx;                                             \
        call 0f;                                                \
        0:  popl %ebx;                                          \
        addl $_GLOBAL_OFFSET_TABLE+[.-0b], %ebx;
#else
#define JUMPTARGET(name)        name
#define SYSCALL_PIC_SETUP       /* Nothing.  */
#endif

#endif /* __ASSEMBLY__ */
