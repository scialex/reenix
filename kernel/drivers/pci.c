#include "drivers/pci.h"

list_t pci_list;

/*
 * This function is the low level interface for reading from the pci tables
 */
static uint32_t pci_config_read(uint16_t bus, uint8_t device, uint8_t func, uint8_t reg_off, uint8_t length) {
	uint8_t reg = reg_off & 0xFC;
	uint8_t offset = reg_off % 0x04;

	outl(PCI_CONFIGURATION_ADDRESS,
		0x80000000
		| (bus << 16)
		| (device << 11)
		| (func << 8)
		| reg);

	uint32_t readVal = inl(PCI_CONFIGURATION_DATA) >> (8 * offset);

	switch(length) {
		case 1:
			readVal &= 0x000000FF;
			break;
		case 2:
			readVal &= 0x0000FFFF;
			break;
		case 4:
			readVal &= 0xFFFFFFFF;
			break;
	}
	return readVal;
}

/*
 * This function is the low level interface for writing to the pci tables,
 * Basically used to configure a device, in practice right now only the dick
 */
static void pci_config_write_byte(uint8_t bus, uint8_t device, uint8_t func, uint8_t reg, uint8_t val) {
	outl(PCI_CONFIGURATION_ADDRESS,
		0x80000000
		| (bus << 16)
		| (device << 11)
		| (func << 8)
		| (reg & 0xfc));
	outb(PCI_CONFIGURATION_DATA + (reg & 0x03), val);
}

/*
 * This function is the low level interface for writing to the pci tables,
 * Basically used to configure a device, in practice right now only the dick
 */
static void pci_config_write_short(uint8_t bus, uint8_t device, uint8_t func, uint8_t reg, uint16_t val) {
	outl(PCI_CONFIGURATION_ADDRESS,
		0x80000000
		| (bus << 16)
		| (device << 11)
		| (func << 8)
		| (reg & 0xfc));

	outw(PCI_CONFIGURATION_DATA, val);
}

/*
 * This function is the low level interface for writing to the pci tables,
 * Basically used to configure a device, in practice right now only the dick
 */
static void pci_config_write_word(uint8_t bus, uint8_t device, uint8_t func, uint8_t reg, uint32_t val) {
	outl(PCI_CONFIGURATION_ADDRESS,
		0x80000000
		| (bus << 16)
		| (device << 11)
		| (func << 8)
		| (reg & 0xfc));

	outl(PCI_CONFIGURATION_DATA, val);
}

/* builds the list of all PCI devices currently hooked up
 * this get initialized when pci_init() is called
 */
static void pci_build_list(void) {
	uint32_t bus;
	uint16_t device, func;
	uint32_t class, subclass;
	uint16_t bus_addr;
	
	dbg(DBG_DISK, "=> PCI DEVICES\n");

	for (bus = 0; bus < PCIBUSES; ++bus) {
		for (device = 0; device < PCIDEVICES; ++device) {
			uint8_t headerType = pci_config_read(bus, device, 0, PCI_HEADERTYPE, 1);
			uint8_t funcCount = PCIFUNCS;
			if (!(headerType & 0x80)) {
				funcCount = 1;
			}
			for (func = 0; func < funcCount; ++func) {
				uint16_t vendorId = pci_config_read(bus, device, func, PCI_VENDOR_ID, 2);
				if (vendorId && vendorId != 0xFFFF) {
					pcidev_t* dev = kmalloc(sizeof(pcidev_t));
					if (dev == NULL) {
						panic("Ran our of meemory allocating PCI Devices\n");
					}
					/* add to the list of devices */
					list_insert_tail(&pci_list, &dev->pci_link);
					/* Set up the device struct */
					dev->pci_data = NULL;
					dev->pci_bus = bus;
					dev->pci_device = device;
					dev->pci_func = func;
					dev->pci_vendorid = vendorId;
					dev->pci_deviceid = pci_config_read(bus, device, func, PCI_DEVICE_ID, 2);
					dev->pci_classid = pci_config_read(bus, device, func, PCI_CLASS, 1);
					dev->pci_subclassid = pci_config_read(bus, device, func, PCI_SUBCLASS, 1);
					dev->pci_interfaceid = pci_config_read(bus, device, func, PCI_INTERFACE, 1);
					dev->pci_revid = pci_config_read(bus, device, func, PCI_REVISION, 1);
					dev->pci_irq = pci_config_read(bus, device, func, PCI_IRQLINE, 1);
					dbg(DBG_DISK, "DevID: %x, Class: %x, Subclass: %x, Interface: %x, IRQ Line: %x\n",
							dev->pci_deviceid, dev->pci_classid, dev->pci_subclassid, dev->pci_interfaceid, dev->pci_irq);
					/* Read BAR data */
					uint8_t i = 0;
					for (i = 0; i < 6; i++) {
						if (i < 2 || !(headerType & 0x01)) {
							dev->pci_bar[i].base_addr = pci_config_read(bus, device, func, PCI_BAR0 + i * 4, 4);
							if (dev->pci_bar[i].base_addr) {
								dev->pci_bar[i].mem_type = dev->pci_bar[i].base_addr & 0x01;
								if (dev->pci_bar[i].mem_type == 0) {
									dev->pci_bar[i].base_addr &= 0xfffffff0;
								} else {
									dev->pci_bar[i].base_addr &= 0xfffc;
								}
								/* interrupts should be disabled when this is called */
								dev->pci_bar[i].mem_size = (~(pci_config_read(bus, device, func, PCI_BAR0 + i * 4, 4)) | 0x0f) + 1;

							} else {
								dev->pci_bar[i].mem_type = PCI_INVALIDBAR;
							}
						} else {
							dev->pci_bar[i].mem_type = PCI_INVALIDBAR;
						}
					}
				}
			}
		}
	}
}

/* Initialize the PCI device list */
void pci_init(void) {
	list_init(&pci_list);
	pci_build_list();
}

/*
 * Given device attributes, return a pointer to the proper device struct if
 * one exists, otherwise return NULL
 */
pcidev_t* pci_lookup(uint8_t class, uint8_t subclass, uint8_t interface) {
	pcidev_t* dev = NULL;
	list_iterate_begin(&pci_list, dev, pcidev_t, pci_link) {
		/* verify the class subclass and interface are correct */	
		if (((class == PCI_LOOKUP_WILDCARD) || (dev->pci_classid == class)) &&
				((subclass == PCI_LOOKUP_WILDCARD) || (dev->pci_subclassid == subclass)) &&
				((interface == PCI_LOOKUP_WILDCARD) || (dev->pci_interfaceid == interface))) {
			return dev;
		}
	} list_iterate_end();

	return NULL;
}

/*
 * High level interface to reading from the PCI Tables
 */
uint32_t pci_read_config(pcidev_t* dev, uint8_t reg_off, uint8_t length) {
	return pci_config_read(dev->pci_bus, dev->pci_device, dev->pci_func, reg_off, length);
}

/*
 * High level interface to writing to the PCI Tables
 */
void pci_write_config(pcidev_t* dev, uint8_t reg_off, uint32_t val, uint8_t length) {
	/* only allow writing of a byte, a word, or a short */
	KASSERT(length == 1 || length == 4 || length == 2);
	switch (length) {
		case 1:
			pci_config_write_byte(dev->pci_bus, dev->pci_device, dev->pci_func, reg_off, (uint8_t)val);
			break;
		case 2:
			KASSERT(val == (uint16_t)val);
			pci_config_write_short(dev->pci_bus, dev->pci_device, dev->pci_func, reg_off, (uint16_t)val);
			break;
		case 4:
			pci_config_write_word(dev->pci_bus, dev->pci_device, dev->pci_func, reg_off, val);
			break;
		default:
			panic("Invalid pci_write_config\n");
			break;
	}
}
