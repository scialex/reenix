STARTUP(entry/entry.o)

SECTIONS
{
		. = 0x7c00;
		.stage1 : { *(.stage1) }

		. = 0x7e00;
		.stage2 : { *(.stage2) }

		. = 0xc0000000;
		. = ALIGN(0x1000);

		kernel_start = .;
		kernel_start_text = .;

		.text : { *(.text) }

		kernel_start_init = .;
		.init : { *(.init) }
		kernel_end_init = .;

		. = ALIGN(0x1000);
		kernel_end_text = .;
		kernel_start_data = .;

		.data : { *(.data) }

		kernel_end_data = .;
		kernel_start_bss = .;

		.bss : { *(.bss) }

		. = ALIGN(0x1000);
		kernel_end_bss = .;
		kernel_end = .;

		/* tells the bootloader how many sectors it needs
		 * to load from the floppy to get the whole kernel */
		kernel_text_sectors = ((kernel_end - kernel_start) / 512) + 1;
}
