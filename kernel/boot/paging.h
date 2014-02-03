#pragma once

#define PAGE_DIRECTORY_BASE 0x2000

install_page_table:
		pusha
		mov		%es, %bx
		
/* first we zero 3 pages to be used as our
 * initial page tables and page directory */
		mov		$0, %ax
		mov		%ax, %es
		mov		$0, %eax
		mov		$0x3000, %ecx
		mov		$PAGE_DIRECTORY_BASE, %edi
		cld     /* Make sure direction is forwards */
		rep stosb

/* identity map the first 1mb of memory using
 * one of the page tables we just zeroed */
		mov		$PAGE_DIRECTORY_BASE, %edi
		add		$0x1000, %edi /* calculate the location of the page table */
		mov		$0x400, %ecx /* we will fill this many entries (whole table) */
		mov		$0x103, %eax /* start by mapping 0x0 => 0x0, the flags are
								priviledged only, global, and present */
1:		
		mov		%eax, (%edi) /* store the pte */
		add		$4, %edi	 /* move to the next entry */
		add		$0x1000, %eax /* increment the mapping to the next memory area */
		dec		%ecx         /* decrement our counter */
		jnz		1b           /* if counter reaches 0 we are done */

/* map 1mb of memory for the kernel text, starting
 * with the mapping 0xc0000000 => KERNEL_PHYS_BASE */
		mov		$PAGE_DIRECTORY_BASE, %edi
		add		$0x2000, %edi /* calculate the location of the page table */
		mov		$0x400, %ecx /* we will fill this many entries (whole table) */
		mov		$KERNEL_PHYS_BASE, %eax
		xor		$0x103, %eax /* start by mapping 0xc0000000 => KERNEL_PHYS_BASE, the flags are
								   priviledged only, global, and present */
1:		
		mov		%eax, (%edi) /* store the pte */
		add		$4, %edi	 /* move to the next entry */
		add		$0x1000, %eax /* increment the mapping to the next memory area */
		dec		%ecx         /* decrement our counter */
		jnz		1b           /* if counter reaches 0 we are done */

/* put entries for these two page tables into the
 * page directory */
		mov		$PAGE_DIRECTORY_BASE, %edi
		
		mov		%edi, %eax
		add		$0x1000, %eax /* calculate the location of the first page table */
		or		$0x103, %eax /* add the flags for privileged, global and present */
		mov		%eax, (%edi)

		add		$0x1000, %eax /* calculate the location of the second page table */
		mov		$kernel_start, %ecx
		shr		$20, %ecx
		mov		%eax, (%ecx, %edi, 1)

		mov		$PAGE_DIRECTORY_BASE, %eax
		mov		%eax, %cr3
		
		mov		%bx, %es
		popa
		ret
