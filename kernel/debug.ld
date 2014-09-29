ENTRY(_start)
STARTUP(entry/entry.o)

SECTIONS
{
        . = 0x100000;
        .setup ALIGN(0x1000) : { *(.multiboot) *(.setup) }
		. = ALIGN(0x1000);
        setup_end = .;

		. = 0xc0000000;

		kernel_start = .;
		kernel_start_text = .;

		.text ALIGN(4096) : { *(.multiboot) *(.text) }

		kernel_start_init = .;
		.init : { *(.init) }
		kernel_end_init = .;

		. = ALIGN(0x1000);
		kernel_end_text = .;
		kernel_start_data = .;

		.data   ALIGN(4096) : { *(.data) }
		.rodata ALIGN(4096) : { *(.rodata) }

		kernel_end_data = .;
		kernel_start_bss = .;

		.bss ALIGN(4096) : { *(.bss) }

		. = ALIGN(0x1000);
		kernel_end_bss = .;
		kernel_end = .;

		/* tells the bootloader how many sectors it needs
		 * to load from the floppy to get the whole kernel */
        kernel_phys_off = kernel_start - setup_end;
		kernel_text_sectors = ((kernel_end - kernel_start) / 512) + 1;
        kernel_page_tables = ((kernel_end - kernel_start) / 0x100000) + 1;
        kernel_phys_base = (kernel_start - kernel_phys_off);
}
