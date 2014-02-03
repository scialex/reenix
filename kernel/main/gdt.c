#include "main/gdt.h"

#include "util/printf.h"
#include "util/debug.h"
#include "util/string.h"

struct tss_entry {
        uint32_t ts_link;
        uint32_t ts_esp0;
        uint32_t ts_ss0;
        uint32_t ts_esp1;
        uint32_t ts_ss1;
        uint32_t ts_esp2;
        uint32_t ts_ss2;
        uint32_t ts_cr3;
        uint32_t ts_eip;
        uint32_t ts_eflags;
        uint32_t ts_eax;
        uint32_t ts_ecx;
        uint32_t ts_edx;
        uint32_t ts_ebx;
        uint32_t ts_esp;
        uint32_t ts_ebp;
        uint32_t ts_esi;
        uint32_t ts_edi;
        uint32_t ts_es;
        uint32_t ts_cs;
        uint32_t ts_ss;
        uint32_t ts_ds;
        uint32_t ts_fs;
        uint32_t ts_gd;
        uint32_t ts_ldtr;
        uint32_t ts_iopb;
};

struct gdt_entry {
        uint16_t ge_limitlo;
        uint16_t ge_baselo;
        uint8_t  ge_basemid;
        uint8_t  ge_access;
        uint8_t  ge_flags;
        uint8_t  ge_basehi;
} __attribute__((packed));

struct gdt_location {
        uint16_t gl_size;
        uint32_t gl_offset;
} __attribute__((packed));

static struct gdt_entry gdt[GDT_COUNT];
static struct tss_entry tss;
static struct gdt_location gdtl = {
        .gl_size = GDT_COUNT * 8,
        .gl_offset = (uint32_t) &gdt
};

void gdt_init(void)
{
        struct gdt_location *data = &gdtl;

        memset(&gdt[0], 0, sizeof(gdt));

        gdt_set_entry(GDT_KERNEL_TEXT, 0x0, 0xFFFFF, 0, 1, 0, 1);
        gdt_set_entry(GDT_KERNEL_DATA, 0x0, 0xFFFFF, 0, 0, 0, 1);
        gdt_set_entry(GDT_USER_TEXT, 0x0, 0xFFFFF, 3, 1, 0, 1);
        gdt_set_entry(GDT_USER_DATA, 0x0, 0xFFFFF, 3, 0, 0, 1);

        __asm__ volatile("lgdt (%0)" :: "p"(data));

        gdt_set_entry(GDT_TSS, (uint32_t)&tss, sizeof(tss), 0, 1, 0, 0);
        gdt[GDT_TSS / 8].ge_access &= ~(0b10000);
        gdt[GDT_TSS / 8].ge_access |= 0b1;
        gdt[GDT_TSS / 8].ge_flags &= ~(0b10000000);

        memset(&tss, 0, sizeof(tss));
        tss.ts_ss0 = GDT_KERNEL_DATA;
        tss.ts_iopb = sizeof(tss);

        int segment = GDT_TSS;
        __asm__ volatile("ltr %0" :: "m"(segment));
}

void gdt_set_kernel_stack(void *addr)
{
        tss.ts_esp0 = (uint32_t)addr;
}

void gdt_set_entry(uint32_t segment, uint32_t base, uint32_t limit,
                   uint8_t ring, int exec, int dir, int rw)
{
        KASSERT(segment < GDT_COUNT * 8 && 0 == segment % 8);
        KASSERT(ring <= 3);
        KASSERT(limit <= 0xFFFFF);

        int index = segment / 8;
        gdt[index].ge_limitlo = (uint16_t)limit;
        gdt[index].ge_baselo = (uint16_t)base;
        gdt[index].ge_basemid = (uint8_t)(base >> 16);
        gdt[index].ge_basehi = (uint8_t)(base >> 24);
        gdt[index].ge_flags = 0b11000000 | (uint8_t)(limit >> 16);

        gdt[index].ge_access = 0b10000000;
        gdt[index].ge_access |= (ring << 5);
        gdt[index].ge_access |= 0b10000;
        if (exec)
                gdt[index].ge_access |= 0b1000;
        if (dir)
                gdt[index].ge_access |= 0b100;
        if (rw)
                gdt[index].ge_access |= 0b10;
}

void gdt_clear(uint32_t segment)
{
        KASSERT(segment < GDT_COUNT * 8 && 0 == segment % 8);
        memset(&gdt[segment / 8], 0, sizeof(gdt[segment / 8]));
}

size_t gdt_tss_info(const void *arg, char *buf, size_t osize)
{
        size_t size = osize;

        KASSERT(NULL == arg);

        iprintf(&buf, &size, "TSS:\n");
        iprintf(&buf, &size, "kstack: %#.8x\n", tss.ts_esp0);

        return size;
}
