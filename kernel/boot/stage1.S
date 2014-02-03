		.file "stage1.S"

		.section .stage1
		.code16

_start:
		jmp		loader

#include "stdio.h"
#include "a20.h"

message_stage1:
		.string "Starting stage1...\r\n"
message_stage2:
		.string "Jumping to stage2...\r\n"
message_a20_enabled:
		.string "A20 enabled\r\n"
message_a20_disabled:
		.string "A20 disabled\r\n"

loader:
		/* zeroing all necessary segments */
		xor		%ax, %ax
		mov		%ax, %ds	
		mov		%ax, %es	
		mov		%ax, %fs	
		mov		%ax, %gs	

		mov		$message_stage1, %si
		call	puts16

		/* test if A20 is enabled */
		call	check_a20

		cmp		$0, %ax
		jne		1f

		mov		$message_a20_disabled, %si
		call	puts16
		jmp		2f

1:
		mov		$message_a20_enabled, %si
		call	puts16

2:
		/* Load stage2 from the disk */
		/* int 0x13 function 0 resets the floppy
		 * disk state in order to guarantee that
		 * we are reading from the start of the disk */
1:
		mov		$0, %ah
		mov		$0, %dl
		int		$0x13
		jc		1b /* if carry flag is set there was an error */

		mov 	$0x0000, %bx /* es:bx indicates where we are reading sectors into */
		mov		%bx, %es

		mov		$0x7e00, %bx

1:
		/* uses int 0x13 function 2 reads sectors from the
		 * floppy disk to load stage2 */
		mov		$0x02, %ah
		mov		$0x02, %al /* number of sectors to read (2) */
		mov		$0x00, %ch /* start on track 0 */
		mov		$0x02, %cl /* start on sector 2 */
		mov		$0x00, %dh /* start on head 0 */
		mov		$0x00, %dl /* drive number */
		int		$0x13
		jc		1b /* if carry flag is set there was an error */

		/* jump into stage 2 that we just loaded */
		mov		$message_stage2, %si
		call	puts16

		jmp		stage2_start

		/* the last two bytes of this sector must be 0xaa55 in order
		 * to identify it as a boot sector to the BIOS */
		. = _start + 510
		.word 0xaa55
