#pragma once

#define PAGE_DIRECTORY_BASE initial_page_table
.set kernel_page_tables_plus_one, (kernel_page_tables + 1)

.global install_page_table
install_page_table:
		pusha
		mov		%es, %bx

/* first we zero 2 + kernel_pages * 2 pages to be used as our
 * initial page tables and page directory (2 so that we have an extra for the final page) */
		mov		$0, %ax
		mov		%ax, %es
		mov		$0, %eax /* What to set the memory to. */
        mov     $0x1000, %ecx /* TO_CLEAR = PAGE_SIZE * NUM_TABLES * 2 + 2 * PAGE_SIZE */
        imul    $kernel_page_tables, %ecx
        add     %ecx, %ecx
        add     $0x2000, %ecx
		mov		$PAGE_DIRECTORY_BASE, %edi /* Where we are clearing from */
		cld     /* Make sure direction is forwards */
		rep stosb /* memset(EDI, EAX, ECX) or memset(PAGE_DIRECTORY_BASE, 0, PAGE_SIZE * NUM_TABLES * 2 + PAGE_SIZE + PAGE_SIZE) */

/* identity map the first kernel_page_tables worth of memory using
 * the page tables we just zeroed */
		mov		$PAGE_DIRECTORY_BASE, %edi
		add		$0x1000, %edi /* calculate the location of the page table */
		mov		$0x400, %ecx /* we will fill this many entries per table */
        imul    $kernel_page_tables, %ecx /* We will fill this many tables */
		mov		$0x103, %eax /* start by mapping 0x0 => 0x0, the flags are
								priviledged only, global, and present */
1:
		mov		%eax, (%edi) /* store the pte */
		add		$4, %edi	 /* move to the next entry */
		add		$0x1000, %eax /* increment the mapping to the next memory area */
		dec		%ecx         /* decrement our counter */
		jnz		1b           /* if counter reaches 0 we are done */

/* map kernel_page_tables + 1 pages of memory for the kernel text, starting
 * with the mapping 0xc0000000 => KERNEL_PHYS_BASE */
		mov		$0x1000, %edi /* calculate the location of the page table */
        imul    $kernel_page_tables, %edi
        add     $0x1000, %edi /* Include the size of the page directory */
        add     $PAGE_DIRECTORY_BASE, %edi
		mov		$0x400, %ecx /* we will fill this many entries (whole table) */
        imul    $kernel_page_tables_plus_one, %ecx /* We fill an extra one to ensure that there is
                                                    * enough room for the kernel's own page tables */
		mov		$kernel_phys_base, %eax
		xor		$0x103, %eax /* start by mapping 0xc0000000 => kernel_phys_base, the flags are
                                priviledged only, global, and present */
1:
		mov		%eax, (%edi) /* store the pte */
		add		$4, %edi	 /* move to the next entry */
		add		$0x1000, %eax /* increment the mapping to the next memory area */
		dec		%ecx         /* decrement our counter */
		jnz		1b           /* if counter reaches 0 we are done */

/* put entries for these page tables into the
 * page directory */
		mov		$PAGE_DIRECTORY_BASE, %edi

        /* Store the entries for the identity mapped section */
		mov		%edi, %eax
		or		$0x103, %eax /* add the flags for privileged, global and present */
        mov     $0, %ecx
1:
		add		$0x1000, %eax /* calculate the location of the next page table */
		mov		%eax, (%edi, %ecx, 4) /* PAGE_DIRECTORY_BASE[ECX] = EAX; */
        inc     %ecx /* Move onto the next entry */
        cmp     $kernel_page_tables, %ecx /* If we have set the required number of tables exit. */
        jne     1b

        /* Store the entries for the higher half mapped section */
		mov		$kernel_start, %ecx
		shr		$22, %ecx /* The first page table entry */
        mov     %ecx, %edx
        add     $kernel_page_tables_plus_one, %edx /* EDX = One after the last page table entry */
1:
		add		$0x1000, %eax /* calculate the location of the next page table */
		mov		%eax, (%edi, %ecx, 4) /* PAGE_DIRECTORY_BASE[ECX] = EAX */
        inc     %ecx /* Move onto the next entry */
        cmp     %edx, %ecx /* If we have not just done the last one repeat */
        jne     1b

        /* set page table address */
		mov		$PAGE_DIRECTORY_BASE, %eax
		mov		%eax, %cr3

		mov		%bx, %es
		popa
		ret
