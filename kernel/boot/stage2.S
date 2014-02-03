		.file "stage2.S"

		.section .stage2
		.code16

.global stage2_start
stage2_start:
_start:
		jmp start

#include "boot/config.h"
#include "stdio.h"
#include "floppy.h"
#include "gdt.h"
#include "e820.h"
#include "paging.h"
		
message_loading:
		.string "Loading kernel..."
message_memmap:
		.string "Getting memory map...\n\r"

start:
		/* clear the interrupt flag so we are
		 * not interrupted until we are ready */
		cli
		
		/* zeroing all necessary segments */
		xor		%ax, %ax
		mov		%ax, %ds
		mov		%ax, %es	
		mov		%ax, %fs	
		mov		%ax, %gs	

		/* setting up a stack from 0x9000 to 0x9FFF */
		mov		$0x0900, %ax
		mov		%ax, %ss
		mov		$0x0FFF, %sp
		sti

		/* print the loading message */
		mov		$message_memmap, %si
		call	puts16
		
		/* read in the memory map */
		mov		$MEMORY_MAP_BASE, %di
		mov		$40, %bp
		call	build_memory_map
		
		/* print the loading message */
		mov		$message_loading, %si
		call	puts16
		
		/* es indicates where we are reading sectors into */
		mov		$0x0a00, %bx
		mov		%bx, %es
		mov		$0x0000, %bx
		/* kernel_text_sectors is set by the linker script */
		mov		$kernel_text_sectors, %cx
		mov		$0x00, %dl
		mov		$0x03, %ax
		
		call	read_sectors
		call	install_gdt
		call	install_page_table

		/* block interrupts until we are in protected mode with
		 * our interrupt table set up properly */
		cli
		/* enter protected mode by setting the first bit of
		 * cr0 to 1 */
		movl	%cr0, %eax
		orl		$0x80000001, %eax
		movl	%eax, %cr0

		ljmp	$0x08, $stage3

		.code32

stage3:
		mov		$0x10, %ax /* setting data segments */
		mov		%ax, %ds
		mov		%ax, %ss
		mov		%ax, %es
		movl	$0x10000, %esp

		mov		$0xa000, %esi
		mov		$kernel_start, %edi
		/* calculate the kernel size */
		mov		$kernel_start, %ecx
		neg		%ecx
		add		$kernel_end, %ecx
		/* perform a string copy */
		rep		movsb

		ljmp	$0x08, $kernel_start_text

		. = _start + 1024
		
