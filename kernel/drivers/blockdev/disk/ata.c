#include "types.h"
#include "drivers/pci.h"
#include "util/debug.h"

uint16_t ata_setup_busmaster_simple(uint8_t channel) {
  /* First step is to read the command register and see what's there */
	pcidev_t* ide = pci_lookup(0x01, 0x01, 0x80);

	if (ide == NULL) {
		panic("Could not find ide device\n");
	}

	uint32_t command = pci_read_config(ide, PCI_COMMAND, 2);
	/* set the busmaster bit to 1 to enable busmaster */
	command |= 0x4;
	/* clear bit 10 to make sure that interrupts are enabled */
	command &= 0xfdff;

	pci_write_config(ide, PCI_COMMAND, command, 2);
	/* read BAR4 and return the address of the busmaster register */
	uint32_t busmaster_base = ide->pci_bar[4].base_addr + (channel * 8);

	if (busmaster_base == 0) {
		panic("No valid busmastering address\n");
	}

	KASSERT(busmaster_base != 0 && "Disk device should not have 0 for the busmaster register");

	return (uint16_t)busmaster_base;
}
