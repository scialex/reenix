#pragma once

		/* install GDT */
install_gdt:
		cli				/* disable interrupts */
		pusha			/* save registers */
		lgdt .gdtdesc	/* load the gdt */
		sti				/* enable interrupts */
		popa			/* restore registers */
		ret

		/* our GDT */
.gdtdata:
		.word	0, 0
		.byte	0, 0, 0, 0

		/* kernel code segment */
		.word	0xFFFF, 0
		.byte	0, 0x9A, 0xCF, 0

		/* kernel data segment */
		.word	0xFFFF, 0
		.byte	0, 0x92, 0xCF, 0

.gdtdesc: 
		.word	0x27
		.long	.gdtdata





