        .file "boot.S"
        .global _start
        .global _high_half
        .global gdt_flush

#define ASM_FILE 1
#include "multiboot.h"
#undef ASM_FILE

.set FLAGS, MULTIBOOT_PAGE_ALIGN | MULTIBOOT_MEMORY_INFO
.set CHECKSUM, -(MULTIBOOT_HEADER_MAGIC + FLAGS)

/* This tells GRUB we can be run */
.section .multiboot, "", @progbits
.align 4
.long MULTIBOOT_HEADER_MAGIC
.long FLAGS
.long CHECKSUM

/* We need this in its own section because we need it to have the same virtual and physical addrs */
.section .setup, "ax", @progbits

#include "boot/config.h"
#include "paging.h"
#include "gdt.h"

_start:
    /* clear the interrupt flag so we are
     * not interrupted until we are ready */
    cli

    /* zeroing all necessary segments */
    xor     %ax, %ax
    mov     %ax, %ds
    mov     %ax, %es
    mov     %ax, %fs
    mov     %ax, %gs

    sti

    // Take the multiboot information and store it somewhere.

    movl    $sys_stack_bottom, %esp
    movl    %esp, %ebp
    pushl   %ebx /* Stash the meminfo for later */
    /* Set up the gdt. */
    call    install_gdt

    mov     $0x10, %ax /* setting data segments */
    mov     %ax, %ds
    mov     %ax, %ss
    mov     %ax, %es
    /* Add in the page tables. */
    call    install_page_table
    /* block interrupts until we are in protected mode with
     * our interrupt table set up properly */
    cli
    /* Start using pages */
    movl    %cr0, %eax
    orl     $0x80000000, %eax
    movl    %eax, %cr0

    popl    %ebx
    ljmp    $0x08, $_finish_boot

/* This is the stack until we actually set up a real one. */
sys_stack:
.align 4
.skip 0x1000
sys_stack_bottom:

_finish_boot:
    mov     $0x10, %ax /* setting data segments */
    mov     %ax, %ds
    mov     %ax, %ss
    mov     %ax, %es
    /* Copy the multiboot information */
    movl    %ebx, (boot_info)
    ljmp    $0x08, $kernel_start_text
    cli
    hlt

/* We will just give them 0.25 mb to hold all the page tables. This is almost certainly more than enough */
.align 0x1000
initial_page_table:
.skip 0x40000

