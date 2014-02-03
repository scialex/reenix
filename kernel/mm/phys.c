#include "types.h"
#include "kernel.h"

#include "mm/phys.h"

#include "boot/config.h"

#include "util/debug.h"

struct mmap_entry {
        uint32_t me_baselo;
        uint32_t me_basehi;
        uint32_t me_lenlo;
        uint32_t me_lenhi;
        uint32_t me_type;
        uint32_t me_reserved;
};

struct mmap_def {
        uint32_t           md_count;
        struct mmap_entry  md_ents[];
};

static char *type_strings[] = {
        "ERROR: type = 0",
        "Usable",
        "Reserved",
        "ACPI Reclaimable",
        "ACPI NVS"
};
static size_t type_count = sizeof(type_strings) / sizeof(char *);

uintptr_t
phys_detect_highmem(void)
{
        uint32_t i;
        struct mmap_def *mmap = (struct mmap_def *)MEMORY_MAP_BASE;
        dbgq(DBG_MM, "Physical Memory Map:\n");
        for (i = 0; i < mmap->md_count; ++i) {
                uint32_t base = mmap->md_ents[i].me_baselo;
                uint32_t length = mmap->md_ents[i].me_lenlo;
                uint32_t type = mmap->md_ents[i].me_type;
                dbgq(DBG_MM, "    0x%.8x-0x%.8x: %s\n", base, base + length,
                     (type < type_count) ? type_strings[type] : "UNDEF");

                if (1 /* Usable */ == type && KERNEL_PHYS_BASE >= base && KERNEL_PHYS_BASE < base + length) {
                        return (uintptr_t)(base + length);
                }
        }
        KASSERT(0 && "Failed to detect correct physical addresses.");
        return 0;
}

