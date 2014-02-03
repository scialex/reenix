#pragma once

#include "util/list.h"
#include "util/bits.h"
#include "main/io.h"
#include "util/debug.h"
#include "mm/kmalloc.h"

#define PCIDEVICES	256
#define PCIBUSES	32
#define PCIFUNCS	8

#define PCI_CONFIGURATION_ADDRESS 0X0CF8
#define PCI_CONFIGURATION_DATA 0x0CFC
#define PCI_VENDOR_ID   0x00
#define PCI_DEVICE_ID   0x02
#define PCI_COMMAND     0x04
#define PCI_STATUS      0x06
#define PCI_REVISION    0x08
#define PCI_CLASS       0x0B
#define PCI_SUBCLASS    0x0A
#define PCI_INTERFACE   0x09
#define PCI_HEADERTYPE  0x0E
#define PCI_BAR0        0x10
#define PCI_BAR1        0x14
#define PCI_BAR2        0x18
#define PCI_BAR3        0x1C
#define PCI_BAR4        0x20
#define PCI_BAR5        0x24
#define PCI_CAPLIST     0x34
#define PCI_IRQLINE     0x3C

#define PCI_CMD_IO		BIT(0)
#define PCI_CMD_MMIO		BIT(1)
#define PCI_CMD_BUSMASTER	BIT(2)

enum {
	PCI_MMIO, PCI_IO, PCI_INVALIDBAR
};

#define PCI_LOOKUP_WILDCARD 0xff

typedef struct pcibar {
	uint32_t base_addr;
	size_t mem_size;
	uint8_t mem_type;
} pcibar_t;

typedef struct pcidev {
	uint8_t pci_bus;
	uint8_t pci_device;
	uint8_t pci_func;
	uint16_t pci_vendorid;
	uint16_t pci_deviceid;
	uint8_t pci_classid;
	uint8_t pci_subclassid;
	uint8_t pci_interfaceid;
	uint8_t pci_revid;
	uint8_t pci_irq;
	pcibar_t pci_bar[6];
	void* pci_data;
	/* link in the list of pci devices */
	list_link_t pci_link;
} pcidev_t;

void pci_init(void);

pcidev_t* pci_lookup(uint8_t class, uint8_t subclass, uint8_t interface);

uint32_t pci_read_config(pcidev_t* dev, uint8_t reg_off, uint8_t length);

void pci_write_config(pcidev_t* dev, uint8_t reg_off, uint32_t val, uint8_t length);
