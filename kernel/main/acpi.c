#include "types.h"

#include "main/acpi.h"

#include "mm/page.h"
#include "mm/pagetable.h"
#include "mm/kmalloc.h"

#include "util/debug.h"
#include "util/string.h"

#define RSDT_SIGNATURE (*(uint32_t*)"RSDT")
#define FACP_SIGNATURE (*(uint32_t*)"FACP")
#define DSDT_SIGNATURE (*(uint32_t*)"DSDT")

#define RSDP_ALIGN (16)

#define EBDA_MIN (0x80000)
#define EBDA_MAX (0xa0000)
#define EBDA_PTR_LOC (0x040e)

static const uint8_t rsdp_sig[8] = {'R', 'S', 'D', ' ', 'P', 'T', 'R', ' '};

struct rsdp {
        uint8_t rp_sign[8];
        uint8_t rp_checksum;
        uint8_t rp_oemid[6];
        uint8_t rp_rev;
        uint32_t rp_addr;
};

struct rsd_table {
        struct acpi_header rt_header;
        uint32_t rt_other[];
};

static uint8_t __acpi_checksum(uint8_t *buf, int size)
{
        uint8_t sum = 0;
        int i;
        for (i = 0; i < size; ++i) {
                sum += buf[i];
        }
        return sum;
}

static void *__rsdp_search()
{
        struct rsdp *rsdp = NULL;

        /* detect the location of the EBDA from the BIOS data section */
        void *ebda = (void *)(((uint32_t) * (uint16_t *)(EBDA_PTR_LOC)) << 4);
        if (ebda >= (void *)EBDA_MIN && ebda < (void *)EBDA_MAX && 0 == ((uint32_t)ebda) % RSDP_ALIGN) {
                /* if EBDA pointer is valid search there first */
                uint8_t *p;
                for (p = ebda; (uint8_t *)p <= (uint8_t *)EBDA_MAX - sizeof(*rsdp); p += RSDP_ALIGN) {
                        if (0 == memcmp(p, rsdp_sig, sizeof(rsdp_sig)) && 0 == __acpi_checksum(p, sizeof(struct rsdp))) {
                                rsdp = (struct rsdp *)p;
                                break;
                        }
                }
        }

        if (NULL == rsdp) {
                /* if the RSDP was not found in the EDPA, but it could still be found
                 * between memory locations 0xe0000 and 0xfffff */
                uint8_t *p;
                for (p = (uint8_t *)0xe0000; (uint8_t *)p <= (uint8_t *)0xfffff - sizeof(*rsdp); p += RSDP_ALIGN) {
                        if (0 == memcmp(p, rsdp_sig, sizeof(rsdp_sig)) && 0 == __acpi_checksum(p, sizeof(struct rsdp))) {
                                rsdp = (struct rsdp *)p;
                                break;
                        }
                }
        }

        return rsdp;
}

static struct rsdp *rsd_ptr;
static struct rsd_table *rsd_table;

/* given the physical address of an ACPI table this function
 * allocates memory for that table and copies the table into
 * that memory, returning the new virtual address for that table */
static void *_acpi_load_table(uintptr_t paddr)
{
        struct acpi_header *tmp =
                (struct acpi_header *)(pt_phys_tmp_map((uintptr_t)PAGE_ALIGN_DOWN(paddr)) + (PAGE_OFFSET(paddr)));

        /* this function is not designed to handle tables which
         * cross page boundaries */
        KASSERT(PAGE_OFFSET(paddr) + tmp->ah_size < PAGE_SIZE);
        struct acpi_header *table = kmalloc(tmp->ah_size);
        memcpy(table, tmp, tmp->ah_size);
        return (void *)table;
}

void acpi_init()
{
        /* search memory for the RSDP, this should reside within the first 1mb of
         * of memory, which is identity mapped during initialization */
        rsd_ptr = __rsdp_search();
        KASSERT(NULL != rsd_ptr && "Could not find the ACPI Root Descriptor Table.");

        /* use the RSDP to find the RSDT, which will probably be in unmapped physical
         * memory, therefore we must use the phys_tmp_map functionallity of page tables */
        rsd_table = _acpi_load_table(rsd_ptr->rp_addr);
        KASSERT(RSDT_SIGNATURE == rsd_table->rt_header.ah_sign);
	/* Only support ACPI version 1.0 */
        KASSERT(0 == __acpi_checksum((void *)rsd_table, rsd_table->rt_header.ah_size) && "Weenix only supports ACPI 1.0");

        dbgq(DBG_CORE, "--- ACPI INIT ---\n");
        dbgq(DBG_CORE, "rsdp addr:  %p\n", rsd_ptr);
        dbgq(DBG_CORE, "rsdt addr:  %p\n", rsd_table);
        dbgq(DBG_CORE, "rev:        %i\n", (int)rsd_ptr->rp_rev);
        dbgq(DBG_CORE, "oem:        %s6\n", (char *)rsd_ptr->rp_oemid);

        /* search for all tables listed in the RSDT and checksum them */
        dbgq(DBG_CORE, "ents:\t");
        int len = (rsd_table->rt_header.ah_size - sizeof(rsd_table->rt_header));
        len /= sizeof(rsd_table->rt_other[0]);
        int i;
        for (i = 0; i < len; ++i) {
                struct acpi_header *h = _acpi_load_table(rsd_table->rt_other[i]);
                rsd_table->rt_other[i] = (uintptr_t)h;

                dbgq(DBG_CORE, "%.4s ", (char *)&h->ah_sign);
                KASSERT(0 == __acpi_checksum((void *)h, h->ah_size));
        }
        dbgq(DBG_CORE, "\n");
}

void *acpi_table(uint32_t signature, int index)
{
        KASSERT(index >= 0);

        int len = (rsd_table->rt_header.ah_size - sizeof(rsd_table->rt_header));
        len /= sizeof(rsd_table->rt_other[0]);

        int i;
        for (i = 0; i < len; ++i) {
                struct acpi_header *h = (struct acpi_header *)rsd_table->rt_other[i];
                if (h->ah_sign == signature && 0 == index--) {
                        return h;
                }
        }
        return NULL;
}
