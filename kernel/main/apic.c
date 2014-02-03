#include "types.h"

#include "main/io.h"
#include "main/acpi.h"
#include "main/cpuid.h"

#include "mm/page.h"
#include "mm/pagetable.h"

#include "util/debug.h"

#define APIC_SIGNATURE (*(uint32_t*)"APIC")

#define TYPE_LAPIC 0
#define TYPE_IOAPIC 1

/* For disabling interrupts on the 8259 PIC, it needs to be
 * disabled to use the APIC
 */
#define PIC_COMPLETE_MASK 0xff

#define PIC1 0x20
#define PIC1_COMMAND PIC1
#define PIC1_DATA (PIC1+1)
#define PIC1_VECTOR 0x20

#define PIC2 0xa0
#define PIC2_COMMAND PIC2
#define PIC2_DATA (PIC2+1)
#define PIC2_VECTOR 0x28

#define ICW1_ICW4	0x01		/* ICW4 (not) needed */
#define ICW1_SINGLE	0x02		/* Single (cascade) mode */
#define ICW1_INTERVAL4	0x04		/* Call address interval 4 (8) */
#define ICW1_LEVEL	0x08		/* Level triggered (edge) mode */
#define ICW1_INIT	0x10		/* Initialization - required! */
 
#define ICW4_8086	0x01		/* 8086/88 (MCS-80/85) mode */
#define ICW4_AUTO	0x02		/* Auto (normal) EOI */
#define ICW4_BUF_SLAVE	0x08		/* Buffered mode/slave */
#define ICW4_BUF_MASTER	0x0C		/* Buffered mode/master */
#define ICW4_SFNM	0x10		/* Special fully nested (not) */



/* For enabling interrupts from the APIC rather than the
 * Master PIC, use the Interrupt Mode Configuration Register (IMCR)
 */

#define SELECT_REGISTER 0x22
#define IMCR_REGISTER 0x70
#define ENABLE_APIC 0x23
#define	ENABLE_APIC_PORT 0x01 

/* For Local APICS */
#define IA32_APIC_BASE_MSR 0x1b
#define IA32_APIC_BASE_MSR_ENABLE 0x800
#define LOCAL_APIC_SPURIOUS_REGISTER 0xf0
#define LOCAL_APIC_ENABLE_INTERRUPT 0x100

#define LOCAL_APIC_ID 0x20
#define LOCAL_APIC_VERSION 0x30
#define LOCAL_APIC_TASKPRIOR 0x80
#define LOCAL_APIC_EOI 0xb0
#define LOCAL_APIC_LDR 0xd0
#define LOCAL_APIC_DFR 0xe0
#define LOCAL_APIC_SPURIOUS 0xf0
#define LOCAL_APIC_ESR 0x280
#define LOCAL_APIC_ICRL 0x300
#define LOCAL_APIC_ICRH 0x310
#define LOCAL_APIC_LVT_TMR 0x320
#define LOCAL_APIC_LVT_PERF 0x340
#define LOCAL_APIC_LVT_LINT0 0x350
#define LOCAL_APIC_LVT_LINT1 0x360
#define LOCAL_APIC_LVT_ERR 0x370
#define LOCAL_APIC_TMRINITCNT 0x380
#define LOCAL_APIC_TMRCURRCNT 0x390
#define LOCAL_APIC_TMRDIV 0x3e0
#define LOCAL_APIC_LAST 0x38f
#define LOCAL_APIC_DISABLE 0x10000
#define LOCAL_APIC_SW_ENABLE 0x100
#define LOCAL_APIC_CPUFOCUS 0x200
#define LOCAL_APIC_NMI (4<<8)
#define LOCAL_APIC_TMR_PERIODIC 0x20000
#define LOCAL_APIC_TMR_BASEDIV (1<<20)

#define LOCAL_APIC_SPUR_ADDR (*(volatile uint32)t*)(apic->at_addr + LOCAL_APIC_SPURIOUS)
#define LAPICID (*(volatile uint32_t*)(apic->at_addr + LOCAL_APIC_ID))
#define LAPICVER (*(volatile uint32_t*)(apic->at_addr + LOCAL_APIC_VERSION))
#define LAPICTPR (*(volatile uint32_t*)(apic->at_addr + LOCAL_APIC_TASKPRIOR))
#define LAPICSPUR (*(volatile uint32_t*)(apic->at_addr + LOCAL_APIC_SPURIOUS))
#define LAPICEOI (*(volatile uint32_t*)(apic->at_addr + LOCAL_APIC_EOI))

/* IO APIC */
#define IOAPIC_IOWIN 0x10

/* Some configuration for the IO APIC */
#define IOAPIC_ID 0x00
#define IOAPIC_VER 0x01
#define IOAPIC_ARB 0x02
#define IOAPIC_REDTBL 0x03

/* Helpful Macros for IO APIC programming */
#define BIT_SET(data,bit) do { (data) = ((data)|(0x1<<(bit))); } while(0);
#define BIT_UNSET(data,bit) do { (data) = ((data)&~(0x1<<(bit))); } while(0);

#define IRQ_TO_OFFSET(irq,part) (0x10 + (irq * 2) + part)

struct apic_table {
        struct acpi_header at_header;
        uint32_t at_addr;
        uint32_t at_flags;
};

struct lapic_table {
        uint8_t at_type;
        uint8_t at_size;
        uint8_t at_procid;
        uint8_t at_apicid;
        uint32_t at_flags;
};

struct ioapic_table {
        uint8_t at_type;
        uint8_t at_size;
        uint8_t at_apicid;
        uint8_t at_reserved;
        uint32_t at_addr;
        uint32_t at_inti;
};

static struct apic_table *apic = NULL;
static struct lapic_table *lapic = NULL;
static struct ioapic_table *ioapic = NULL;


static uint32_t __lapic_getid(void)
{
	return (LAPICID >> 24) & 0x0f;
}

static uint32_t __lapic_getver(void)
{
	return LAPICVER & 0xff;
}

static void __lapic_setspur(uint8_t intr)
{
        uint32_t data = LAPICSPUR;
        ((uint8_t *)&data)[0] = intr | LOCAL_APIC_SW_ENABLE;
        LAPICSPUR = data;
}

static uint32_t ioapic_read(uintptr_t ioapic_addr, uint8_t reg_offset) {
	/* Tell IOREGSEL where we want to read from */
	*(volatile uint32_t*)(ioapic_addr) = reg_offset;
	return *(uint32_t*)(ioapic_addr + IOAPIC_IOWIN);
}

static void ioapic_write(uintptr_t ioapic_addr, uint8_t reg_offset, uint32_t value) {
	/* Tell IOREGSEL where to write to */
	*(uint32_t*)(ioapic_addr) = reg_offset;
	/* Write the value to IOWIN */
	*(uint32_t*)(ioapic_addr + IOAPIC_IOWIN) = value;
}

static uint32_t __ioapic_getid(void) {
	return (ioapic_read((uintptr_t)ioapic->at_addr, IOAPIC_ID) >> 24) & 0x0f;
}

static uint32_t __ioapic_getver(void) {
	return (ioapic_read((uintptr_t)ioapic->at_addr, IOAPIC_VER) & 0xff);
}

static uint32_t __ioapic_getmaxredir(void) {
	return (ioapic_read((uintptr_t)ioapic->at_addr, IOAPIC_VER) >> 16) & 0xff;
}

static void __ioapic_setredir(uint32_t irq, uint8_t intr) {
	/* Read in the redirect table lower register first */
	uint32_t data = ioapic_read((uintptr_t)ioapic->at_addr, IRQ_TO_OFFSET(irq, 0));
	/* Set the interrupt vector */
	((uint8_t*)&data)[0] = intr;
	/* Unset bits 8,9,10 to set interrupt delivery mode to fixed */
	BIT_UNSET(data, 8);
	BIT_UNSET(data, 9);
	BIT_UNSET(data, 10);
	/* Unset bit 11 to set the destination mode to a physical destination */
	BIT_UNSET(data, 11);
	/* Unset bit 13 to set the pin polarity to Active High */
	BIT_UNSET(data, 13);
	/* Unset bit 15 to set the trigger mode to Edge */
	BIT_UNSET(data, 15);
	/* Write this value to the apic */
	ioapic_write((uintptr_t)ioapic->at_addr, IRQ_TO_OFFSET(irq, 0), data);
	/* Now deal with the higher order register */
	data = ioapic_read((uintptr_t)ioapic->at_addr, IRQ_TO_OFFSET(irq, 1));
	((uint8_t *)&data)[3] = lapic->at_apicid;
	ioapic_write((uintptr_t)ioapic->at_addr, IRQ_TO_OFFSET(irq, 1), data);
}

static void __ioapic_setmask(uint32_t irq, int mask) {
	uint32_t data = ioapic_read((uintptr_t)ioapic->at_addr, IRQ_TO_OFFSET(irq, 0));
	if (mask) {
		BIT_SET(data, 16);
	} else {
		BIT_UNSET(data, 16);
	}
	ioapic_write((uintptr_t)ioapic->at_addr, IRQ_TO_OFFSET(irq, 0), data);
}



static uint32_t apic_exists(void) {
	uint32_t eax, edx;
	cpuid(1, &eax, &edx);
	return edx & CPUID_FEAT_EDX_APIC;
}

static void apic_set_base(uintptr_t apic) {
	uint32_t edx = 0;
	uint32_t eax = (apic & 0xfffff000) | IA32_APIC_BASE_MSR_ENABLE;
	edx = 0;
	cpuid_set_msr(IA32_APIC_BASE_MSR, eax, edx);
}

static uintptr_t apic_get_base(void) {
	uint32_t eax, edx;
	cpuid_get_msr(IA32_APIC_BASE_MSR, &eax, &edx);
	return (eax & 0xfffff000);
}

static void apic_enable() {
	*(uint32_t*)(apic->at_addr + LOCAL_APIC_DFR) = 0xffffffff;
	*(uint32_t*)(apic->at_addr + LOCAL_APIC_LDR) = (*(uint32_t*)(apic->at_addr + LOCAL_APIC_LDR) & 0x00ffffff) | 1;
	*(uint32_t*)(apic->at_addr + LOCAL_APIC_LVT_TMR) = LOCAL_APIC_DISABLE;
	*(uint32_t*)(apic->at_addr + LOCAL_APIC_LVT_PERF) = LOCAL_APIC_NMI;
	*(uint32_t*)(apic->at_addr + LOCAL_APIC_LVT_LINT0) = LOCAL_APIC_DISABLE;
	*(uint32_t*)(apic->at_addr + LOCAL_APIC_LVT_LINT1) = LOCAL_APIC_DISABLE;
	*(uint32_t*)(apic->at_addr + LOCAL_APIC_TASKPRIOR) = 0;
	apic_set_base(apic_get_base());
}

void apic_disable_periodic_timer() {
	*(uint32_t*)(apic->at_addr + LOCAL_APIC_LVT_TMR) = LOCAL_APIC_DISABLE;
	*(uint32_t*)(apic->at_addr + LOCAL_APIC_LVT_PERF) = LOCAL_APIC_NMI;
	*(uint32_t*)(apic->at_addr + LOCAL_APIC_LVT_LINT0) = LOCAL_APIC_DISABLE;
	*(uint32_t*)(apic->at_addr + LOCAL_APIC_LVT_LINT1) = LOCAL_APIC_DISABLE;
	*(uint32_t*)(apic->at_addr + LOCAL_APIC_TASKPRIOR) = 0;
}

void apic_enable_periodic_timer(uint32_t freq) {
	uint32_t tmp;
	uint32_t cpubusfreq;

	dbgq(DBG_CORE, "--- Enabling APIC Timer ---\n");

	*(uint32_t*)(apic->at_addr + LOCAL_APIC_TMRDIV) = 0x03;
	/* Initialize PIT Ch 2 in one-shot mode */
	/* Some crazy magic numbers here */
	outb(0x61, (inb(0x61) & 0xfd) | 1);
	outb(0x43, 0xb2);
	outb(0x42, 0x9b);
	inb(0x60);
	outb(0x42, 0x2e);
	
	/* reset PIT one-shot counter (start counting) */
	tmp = (uint32_t)(inb(0x61) & 0xfe);
	outb(0x61, (uint8_t)tmp);
	outb(0x61, (uint8_t)tmp | 1);
	/* reset APIC timer (set counter to -1) */
	*(uint32_t*)(apic->at_addr + LOCAL_APIC_TMRINITCNT) = 0xffffffff;
	/* wait until the PIT reaches zero */
	while(!(inb(0x61) & 0x20));
	/* Stop the APIC timer */
	*(uint32_t*)(apic->at_addr + LOCAL_APIC_LVT_TMR) = LOCAL_APIC_DISABLE;
	/* some math */
	cpubusfreq = ((0xffffffff - *(uint32_t*)(apic->at_addr + LOCAL_APIC_TMRINITCNT)) + 1) * 16 * 100;
	tmp = (cpubusfreq / freq / 16) * 10000;
	dbgq(DBG_CORE, "CPU Bus Freq: %u\n", cpubusfreq);
	dbgq(DBG_CORE, "APIC Timer initial count %u\n", tmp);
	/* Set up the APIC timer for periodic mode */
	*(uint32_t*)(apic->at_addr + LOCAL_APIC_TMRINITCNT) = (tmp < 16 ? 16 : tmp);
	*(uint32_t*)(apic->at_addr + LOCAL_APIC_LVT_TMR) = 32 | LOCAL_APIC_TMR_PERIODIC;
	*(uint32_t*)(apic->at_addr + LOCAL_APIC_TMRDIV) = 0x03;
}

static void apic_disable_8259() {
	dbgq(DBG_CORE, "--- DISABLE 8259 PIC ---\n");
  /* disable 8259 PICs by initializing them and masking all interrupts */
	/* the first step is initialize them normally */
	outb(PIC1_COMMAND, ICW1_INIT + ICW1_ICW4);
	io_wait();
	outb(PIC2_COMMAND, ICW1_INIT + ICW1_ICW4);
	io_wait();
	outb(PIC1_DATA, PIC1_VECTOR);
	io_wait();
	outb(PIC2_DATA, PIC2_VECTOR);
	io_wait();
	outb(PIC1_DATA, 0x04);
	io_wait();
	outb(PIC2_DATA, 0x02);
	io_wait();
	outb(PIC1_DATA, ICW4_8086);
	io_wait();
	outb(PIC2_DATA, ICW4_8086);
	/* Now mask all interrupts */
	dbgq(DBG_CORE, "Masking all interrupts on the i8259 PIC\n");
	outb(PIC1_DATA, PIC_COMPLETE_MASK);
	outb(PIC2_DATA, PIC_COMPLETE_MASK);
}

void apic_init()
{
        uint8_t *ptr = acpi_table(APIC_SIGNATURE, 0);
        apic = (struct apic_table *)ptr;
        KASSERT(NULL != apic && "APIC table not found in ACPI."
                "If you are using Bochs make sure you configured --enable-apic.");
	
        apic_disable_8259();

        dbgq(DBG_CORE, "--- APIC INIT ---\n");
        dbgq(DBG_CORE, "local apic paddr:     0x%x\n", apic->at_addr);
        dbgq(DBG_CORE, "PC-AT compatible:    %i\n", apic->at_flags & 0x1);
        KASSERT(PAGE_ALIGNED(apic->at_addr));
        apic->at_addr = pt_phys_perm_map(apic->at_addr, 1);

        /* Get the tables for the local APIC and IO APICS,
         * Weenix currently only supports one of each, in order
         * to enforce this a KASSERT will fail this if more than one
         * of each type is found */
        uint8_t off = sizeof(*apic);
        while (off < apic->at_header.ah_size) {
                uint8_t type = *(ptr + off);
                uint8_t size = *(ptr + off + 1);
                if (TYPE_LAPIC == type) {
                        KASSERT(apic_exists() && "Local APIC does not exist");
                        KASSERT(sizeof(struct lapic_table) == size);
                        KASSERT(NULL == lapic && "Weenix only supports a single local APIC");
                        lapic = (struct lapic_table *)(ptr + off);
                        dbgq(DBG_CORE, "LAPIC:\n");
                        dbgq(DBG_CORE, "   id:         0x%.2x\n", (uint32_t)lapic->at_apicid);
                        dbgq(DBG_CORE, "   processor:  0x%.3x\n", (uint32_t)lapic->at_procid);
                        dbgq(DBG_CORE, "   enabled:    %i\n", lapic->at_flags & 0x1);
                        KASSERT(lapic->at_flags & 0x1 && "The local APIC is disabled");
                } else if (TYPE_IOAPIC == type) {
                        KASSERT(apic_exists() && "IO APIC does not exist");
                        KASSERT(sizeof(struct ioapic_table) == size);
                        KASSERT(NULL == ioapic && "Weenix only supports a single IO APIC");
                        ioapic = (struct ioapic_table *)(ptr + off);
                        dbgq(DBG_CORE, "IOAPIC:\n");
                        dbgq(DBG_CORE, "   id:         0x%.2x\n", (uint32_t)ioapic->at_apicid);
                        dbgq(DBG_CORE, "   base paddr:  0x%.8x\n", ioapic->at_addr);
                        dbgq(DBG_CORE, "   inti addr:   0x%.8x\n", ioapic->at_inti);
                        KASSERT(PAGE_ALIGNED(ioapic->at_addr));
                        ioapic->at_addr = pt_phys_perm_map(ioapic->at_addr, 1);
                } else {
                        dbgq(DBG_CORE, "Unknown APIC type:  0x%x\n", (uint32_t)type);
                }
                off += size;
        }
        KASSERT(NULL != lapic && "Could not find a local APIC device");
        KASSERT(NULL != ioapic && "Could not find an IO APIC");

	dbgq(DBG_CORE, "--- Enabling APIC ---\n");
	apic_enable();

  dbgq(DBG_CORE, "Local APIC 0x%.2x Configuration:\n", __lapic_getid());
  dbgq(DBG_CORE, "    APIC Version:         0x%.2x\n", __lapic_getver());
  dbgq(DBG_CORE, "    Spurious Vector:      0x%.8x\n", LAPICSPUR);

  dbgq(DBG_CORE, "IO APIC 0x%.2x Configuration:\n", __ioapic_getid());
  dbgq(DBG_CORE, "    APIC Version:         0x%.2x\n", __ioapic_getver());
  dbgq(DBG_CORE, "    Maximum Redirection:  0x%.2x\n", __ioapic_getmaxredir());

}

uint8_t apic_getipl()
{
        return LAPICTPR & 0xff;
}

void apic_setipl(uint8_t ipl)
{
        LAPICTPR = ipl;
}

void apic_setspur(uint8_t intr)
{
        dbg(DBG_CORE, "mapping spurious interrupts to %hhu\n", intr);
        __lapic_setspur(intr);
}

void apic_eoi()
{
        LAPICEOI = 0x0;
}

void apic_setredir(uint32_t irq, uint8_t intr)
{
        dbg(DBG_CORE, "redirecting irq %u to interrupt %hhu\n", irq, intr);
        __ioapic_setredir(irq, intr);
        __ioapic_setmask(irq, 0);
}
