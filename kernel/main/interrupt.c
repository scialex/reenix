#include "types.h"

#include "util/debug.h"
#include "util/string.h"

#include "main/io.h"
#include "main/apic.h"
#include "main/interrupt.h"
#include "main/gdt.h"

#define MAX_INTERRUPTS          256

#define INTR_SPURIOUS      0xef

/* Convenient definitions for intr_desc.attr */

#define IDT_DESC_TRAP           0x01
#define IDT_DESC_BIT16          0x06
#define IDT_DESC_BIT32          0x0E
#define IDT_DESC_RING0          0x00
#define IDT_DESC_RING1          0x40
#define IDT_DESC_RING2          0x20
#define IDT_DESC_RING3          0x60
#define IDT_DESC_PRESENT        0x80

#define INTR(isr) (__intr_handler ## isr)
#define INTR_ERRCODE(isr)                               \
        extern intr_handler_t __intr_handler ## isr;    \
        __asm__ (                                       \
                ".global __intr_handler" #isr "\n"      \
                "__intr_handler" #isr ":\n\t"           \
                "push $" #isr "\n\t"                    \
                "pusha\n\t"                             \
                "push %ds\n\t"                          \
                "push %es\n\t"                          \
                "movl %ss, %edx\n\t"                    \
                "movl %edx, %ds\n\t"                    \
                "movl %edx, %es\n\t"                    \
                "call __intr_handler\n\t"               \
                "pop %es\n\t"                           \
                "pop %ds\n\t"                           \
                "popa\n\t"                              \
                "add $8, %esp\n\t"                      \
                "iret\n"                                \
        );

#define INTR_NOERRCODE(isr)                             \
        extern intr_handler_t __intr_handler ## isr;    \
        __asm__ (                                       \
                ".global __intr_handler" #isr "\n"      \
                "__intr_handler" #isr ":\n\t"           \
                "push $0\n\t"                           \
                "push $" #isr "\n\t"                    \
                "pusha\n\t"                             \
                "push %ds\n\t"                          \
                "push %es\n\t"                          \
                "movl %ss, %edx\n\t"                    \
                "movl %edx, %ds\n\t"                    \
                "movl %edx, %es\n\t"                    \
                "call __intr_handler\n\t"               \
                "pop %es\n\t"                           \
                "pop %ds\n\t"                           \
                "popa\n\t"                              \
                "add $8, %esp\n\t"                      \
                "iret\n"                                \
        );

INTR_NOERRCODE(0)
INTR_NOERRCODE(1)
INTR_NOERRCODE(2)
INTR_NOERRCODE(3)
INTR_NOERRCODE(4)
INTR_NOERRCODE(5)
INTR_NOERRCODE(6)
INTR_NOERRCODE(7)
INTR_ERRCODE(8)
INTR_NOERRCODE(9)
INTR_ERRCODE(10)
INTR_ERRCODE(11)
INTR_ERRCODE(12)
INTR_ERRCODE(13)
INTR_ERRCODE(14)
INTR_NOERRCODE(15)
INTR_NOERRCODE(16)
INTR_ERRCODE(17)
INTR_NOERRCODE(18)
INTR_NOERRCODE(19)
INTR_NOERRCODE(20)
INTR_NOERRCODE(21)
INTR_NOERRCODE(22)
INTR_NOERRCODE(23)
INTR_NOERRCODE(24)
INTR_NOERRCODE(25)
INTR_NOERRCODE(26)
INTR_NOERRCODE(27)
INTR_NOERRCODE(28)
INTR_NOERRCODE(29)
INTR_NOERRCODE(30)
INTR_NOERRCODE(31)
INTR_NOERRCODE(32)
INTR_NOERRCODE(33)
INTR_NOERRCODE(34)
INTR_NOERRCODE(35)
INTR_NOERRCODE(36)
INTR_NOERRCODE(37)
INTR_NOERRCODE(38)
INTR_NOERRCODE(39)
INTR_NOERRCODE(40)
INTR_NOERRCODE(41)
INTR_NOERRCODE(42)
INTR_NOERRCODE(43)
INTR_NOERRCODE(44)
INTR_NOERRCODE(45)
INTR_NOERRCODE(46)
INTR_NOERRCODE(47)
INTR_NOERRCODE(48)
INTR_NOERRCODE(49)
INTR_NOERRCODE(50)
INTR_NOERRCODE(51)
INTR_NOERRCODE(52)
INTR_NOERRCODE(53)
INTR_NOERRCODE(54)
INTR_NOERRCODE(55)
INTR_NOERRCODE(56)
INTR_NOERRCODE(57)
INTR_NOERRCODE(58)
INTR_NOERRCODE(59)
INTR_NOERRCODE(60)
INTR_NOERRCODE(61)
INTR_NOERRCODE(62)
INTR_NOERRCODE(63)
INTR_NOERRCODE(64)
INTR_NOERRCODE(65)
INTR_NOERRCODE(66)
INTR_NOERRCODE(67)
INTR_NOERRCODE(68)
INTR_NOERRCODE(69)
INTR_NOERRCODE(70)
INTR_NOERRCODE(71)
INTR_NOERRCODE(72)
INTR_NOERRCODE(73)
INTR_NOERRCODE(74)
INTR_NOERRCODE(75)
INTR_NOERRCODE(76)
INTR_NOERRCODE(77)
INTR_NOERRCODE(78)
INTR_NOERRCODE(79)
INTR_NOERRCODE(80)
INTR_NOERRCODE(81)
INTR_NOERRCODE(82)
INTR_NOERRCODE(83)
INTR_NOERRCODE(84)
INTR_NOERRCODE(85)
INTR_NOERRCODE(86)
INTR_NOERRCODE(87)
INTR_NOERRCODE(88)
INTR_NOERRCODE(89)
INTR_NOERRCODE(90)
INTR_NOERRCODE(91)
INTR_NOERRCODE(92)
INTR_NOERRCODE(93)
INTR_NOERRCODE(94)
INTR_NOERRCODE(95)
INTR_NOERRCODE(96)
INTR_NOERRCODE(97)
INTR_NOERRCODE(98)
INTR_NOERRCODE(99)
INTR_NOERRCODE(100)
INTR_NOERRCODE(101)
INTR_NOERRCODE(102)
INTR_NOERRCODE(103)
INTR_NOERRCODE(104)
INTR_NOERRCODE(105)
INTR_NOERRCODE(106)
INTR_NOERRCODE(107)
INTR_NOERRCODE(108)
INTR_NOERRCODE(109)
INTR_NOERRCODE(110)
INTR_NOERRCODE(111)
INTR_NOERRCODE(112)
INTR_NOERRCODE(113)
INTR_NOERRCODE(114)
INTR_NOERRCODE(115)
INTR_NOERRCODE(116)
INTR_NOERRCODE(117)
INTR_NOERRCODE(118)
INTR_NOERRCODE(119)
INTR_NOERRCODE(120)
INTR_NOERRCODE(121)
INTR_NOERRCODE(122)
INTR_NOERRCODE(123)
INTR_NOERRCODE(124)
INTR_NOERRCODE(125)
INTR_NOERRCODE(126)
INTR_NOERRCODE(127)
INTR_NOERRCODE(128)
INTR_NOERRCODE(129)
INTR_NOERRCODE(130)
INTR_NOERRCODE(131)
INTR_NOERRCODE(132)
INTR_NOERRCODE(133)
INTR_NOERRCODE(134)
INTR_NOERRCODE(135)
INTR_NOERRCODE(136)
INTR_NOERRCODE(137)
INTR_NOERRCODE(138)
INTR_NOERRCODE(139)
INTR_NOERRCODE(140)
INTR_NOERRCODE(141)
INTR_NOERRCODE(142)
INTR_NOERRCODE(143)
INTR_NOERRCODE(144)
INTR_NOERRCODE(145)
INTR_NOERRCODE(146)
INTR_NOERRCODE(147)
INTR_NOERRCODE(148)
INTR_NOERRCODE(149)
INTR_NOERRCODE(150)
INTR_NOERRCODE(151)
INTR_NOERRCODE(152)
INTR_NOERRCODE(153)
INTR_NOERRCODE(154)
INTR_NOERRCODE(155)
INTR_NOERRCODE(156)
INTR_NOERRCODE(157)
INTR_NOERRCODE(158)
INTR_NOERRCODE(159)
INTR_NOERRCODE(160)
INTR_NOERRCODE(161)
INTR_NOERRCODE(162)
INTR_NOERRCODE(163)
INTR_NOERRCODE(164)
INTR_NOERRCODE(165)
INTR_NOERRCODE(166)
INTR_NOERRCODE(167)
INTR_NOERRCODE(168)
INTR_NOERRCODE(169)
INTR_NOERRCODE(170)
INTR_NOERRCODE(171)
INTR_NOERRCODE(172)
INTR_NOERRCODE(173)
INTR_NOERRCODE(174)
INTR_NOERRCODE(175)
INTR_NOERRCODE(176)
INTR_NOERRCODE(177)
INTR_NOERRCODE(178)
INTR_NOERRCODE(179)
INTR_NOERRCODE(180)
INTR_NOERRCODE(181)
INTR_NOERRCODE(182)
INTR_NOERRCODE(183)
INTR_NOERRCODE(184)
INTR_NOERRCODE(185)
INTR_NOERRCODE(186)
INTR_NOERRCODE(187)
INTR_NOERRCODE(188)
INTR_NOERRCODE(189)
INTR_NOERRCODE(190)
INTR_NOERRCODE(191)
INTR_NOERRCODE(192)
INTR_NOERRCODE(193)
INTR_NOERRCODE(194)
INTR_NOERRCODE(195)
INTR_NOERRCODE(196)
INTR_NOERRCODE(197)
INTR_NOERRCODE(198)
INTR_NOERRCODE(199)
INTR_NOERRCODE(200)
INTR_NOERRCODE(201)
INTR_NOERRCODE(202)
INTR_NOERRCODE(203)
INTR_NOERRCODE(204)
INTR_NOERRCODE(205)
INTR_NOERRCODE(206)
INTR_NOERRCODE(207)
INTR_NOERRCODE(208)
INTR_NOERRCODE(209)
INTR_NOERRCODE(210)
INTR_NOERRCODE(211)
INTR_NOERRCODE(212)
INTR_NOERRCODE(213)
INTR_NOERRCODE(214)
INTR_NOERRCODE(215)
INTR_NOERRCODE(216)
INTR_NOERRCODE(217)
INTR_NOERRCODE(218)
INTR_NOERRCODE(219)
INTR_NOERRCODE(220)
INTR_NOERRCODE(221)
INTR_NOERRCODE(222)
INTR_NOERRCODE(223)
INTR_NOERRCODE(224)
INTR_NOERRCODE(225)
INTR_NOERRCODE(226)
INTR_NOERRCODE(227)
INTR_NOERRCODE(228)
INTR_NOERRCODE(229)
INTR_NOERRCODE(230)
INTR_NOERRCODE(231)
INTR_NOERRCODE(232)
INTR_NOERRCODE(233)
INTR_NOERRCODE(234)
INTR_NOERRCODE(235)
INTR_NOERRCODE(236)
INTR_NOERRCODE(237)
INTR_NOERRCODE(238)
INTR_NOERRCODE(239)
INTR_NOERRCODE(240)
INTR_NOERRCODE(241)
INTR_NOERRCODE(242)
INTR_NOERRCODE(243)
INTR_NOERRCODE(244)
INTR_NOERRCODE(245)
INTR_NOERRCODE(246)
INTR_NOERRCODE(247)
INTR_NOERRCODE(248)
INTR_NOERRCODE(249)
INTR_NOERRCODE(250)
INTR_NOERRCODE(251)
INTR_NOERRCODE(252)
INTR_NOERRCODE(253)
INTR_NOERRCODE(254)
INTR_NOERRCODE(255)

typedef struct intr_desc {
        uint16_t baselo;
        uint16_t selector;
        uint8_t zero;
        uint8_t attr;
        uint16_t basehi;
} __attribute__((packed)) intr_desc_t;

typedef struct intr_info {
        uint16_t size;
        uint32_t base;
} __attribute__((packed)) intr_info_t;

static intr_desc_t intr_table[MAX_INTERRUPTS];
static intr_handler_t intr_handlers[MAX_INTERRUPTS];
static int32_t intr_mappings[MAX_INTERRUPTS];

intr_info_t intr_data = {
        .size = sizeof(intr_info_t),
        .base = (uint32_t) intr_table
};

/* This variable is updated when an interrupt occurs to
 * point to the saved registers of the interrupted context.
 * When it is non-NULL the processor is in an interrupt
 * context, otherwise it is in a non-interrupt process.
 * This variable is maintained for easy reference by
 * debuggers. */
static regs_t *_intr_regs = NULL;

intr_handler_t intr_register(uint8_t intr, intr_handler_t handler)
{
        intr_handler_t old = intr_handlers[intr];
        intr_handlers[intr] = handler;
        return old;
}

int32_t intr_map(uint16_t irq, uint8_t intr)
{
        KASSERT(INTR_SPURIOUS != intr);

        int32_t oldirq = intr_mappings[intr];
        intr_mappings[intr] = irq;
        apic_setredir(irq, intr);
        return oldirq;
}

static __attribute__((used)) void __intr_handler(regs_t regs)
{
        intr_handler_t handler = intr_handlers[regs.r_intr];
        _intr_regs = &regs;
        if (NULL != handler) {
                handler(&regs);
        } else {
                panic("Unhandled interrupt 0x%x\n", regs.r_intr);
        }

        if (0 <= intr_mappings[regs.r_intr]) {
                apic_eoi();
        }

        _intr_regs = NULL;
}

static void __intr_divide_by_zero_handler(regs_t *regs)
{
        panic("\nDivide by zero error at eip=0x%08x\n", regs->r_eip);
}

static void __intr_gpf_handler(regs_t *regs)
{
        panic("\nGeneral Protection Fault:\nError: 0x%.8x\n", regs->r_err);
}

static void __intr_timer_handler(regs_t *regs)
{
        panic("\nTimer Interrupt:\nError: 0x%.8x\n", regs->r_err);
}

static void __intr_inval_opcode_handler(regs_t *regs)
{
        panic("\nInvalid opcode error at eip=0x%08x\n", regs->r_eip);
}

static void __intr_spurious(regs_t *regs)
{
        dbg(DBG_CORE, ("ignoring spurious interrupt\n"));
}

static void __intr_set_entry(uint8_t isr, uint32_t addr, int seg, int flags)
{
        intr_table[isr].baselo = (uint16_t)((addr) & 0xffff);
        intr_table[isr].basehi = (uint16_t)(((addr) >> 16) & 0xffff);
        intr_table[isr].zero = 0;
        intr_table[isr].attr = flags;
        intr_table[isr].selector = seg;
}

void intr_init()
{
        int i;
        intr_info_t *data = &intr_data;

        /* initialize intr_data */
        intr_data.size = sizeof(intr_desc_t) * MAX_INTERRUPTS - 1;
        intr_data.base = (uint32_t) intr_table;

        memset(intr_handlers, 0, sizeof(intr_handlers));
        for (i = 0; i < MAX_INTERRUPTS; ++i) {
                intr_mappings[i] = -1;
        }

        __intr_set_entry(0,   (uint32_t)&INTR(0),   GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(1,   (uint32_t)&INTR(1),   GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(2,   (uint32_t)&INTR(2),   GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(3,   (uint32_t)&INTR(3),   GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(4,   (uint32_t)&INTR(4),   GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(5,   (uint32_t)&INTR(5),   GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(6,   (uint32_t)&INTR(6),   GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(7,   (uint32_t)&INTR(7),   GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(8,   (uint32_t)&INTR(8),   GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(9,   (uint32_t)&INTR(9),   GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(10,  (uint32_t)&INTR(10),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(11,  (uint32_t)&INTR(11),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(12,  (uint32_t)&INTR(12),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(13,  (uint32_t)&INTR(13),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(14,  (uint32_t)&INTR(14),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(15,  (uint32_t)&INTR(15),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(16,  (uint32_t)&INTR(16),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(17,  (uint32_t)&INTR(17),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(18,  (uint32_t)&INTR(18),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(19,  (uint32_t)&INTR(19),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(20,  (uint32_t)&INTR(20),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(21,  (uint32_t)&INTR(21),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(22,  (uint32_t)&INTR(22),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(23,  (uint32_t)&INTR(23),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(24,  (uint32_t)&INTR(24),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(25,  (uint32_t)&INTR(25),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(26,  (uint32_t)&INTR(26),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(27,  (uint32_t)&INTR(27),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(28,  (uint32_t)&INTR(28),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(29,  (uint32_t)&INTR(29),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(30,  (uint32_t)&INTR(30),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(31,  (uint32_t)&INTR(31),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(32,  (uint32_t)&INTR(32),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(33,  (uint32_t)&INTR(33),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(34,  (uint32_t)&INTR(34),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(35,  (uint32_t)&INTR(35),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(36,  (uint32_t)&INTR(36),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(37,  (uint32_t)&INTR(37),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(38,  (uint32_t)&INTR(38),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(39,  (uint32_t)&INTR(39),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(40,  (uint32_t)&INTR(40),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(41,  (uint32_t)&INTR(41),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(42,  (uint32_t)&INTR(42),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(43,  (uint32_t)&INTR(43),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(44,  (uint32_t)&INTR(44),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(45,  (uint32_t)&INTR(45),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        /* BEWARE - this is the interrupt table entry for userland syscalls. It
         * differs from all the others. */
        __intr_set_entry(46,  (uint32_t)&INTR(46),  GDT_KERNEL_TEXT,
                         IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_TRAP | IDT_DESC_RING3);
        /* */
        __intr_set_entry(47,  (uint32_t)&INTR(47),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(48,  (uint32_t)&INTR(48),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(49,  (uint32_t)&INTR(49),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(50,  (uint32_t)&INTR(50),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(51,  (uint32_t)&INTR(51),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(52,  (uint32_t)&INTR(52),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(53,  (uint32_t)&INTR(53),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(54,  (uint32_t)&INTR(54),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(55,  (uint32_t)&INTR(55),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(56,  (uint32_t)&INTR(56),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(57,  (uint32_t)&INTR(57),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(58,  (uint32_t)&INTR(58),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(59,  (uint32_t)&INTR(59),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(60,  (uint32_t)&INTR(60),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(61,  (uint32_t)&INTR(61),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(62,  (uint32_t)&INTR(62),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(63,  (uint32_t)&INTR(63),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(64,  (uint32_t)&INTR(64),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(65,  (uint32_t)&INTR(65),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(66,  (uint32_t)&INTR(66),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(67,  (uint32_t)&INTR(67),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(68,  (uint32_t)&INTR(68),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(69,  (uint32_t)&INTR(69),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(60,  (uint32_t)&INTR(70),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(71,  (uint32_t)&INTR(71),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(72,  (uint32_t)&INTR(72),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(73,  (uint32_t)&INTR(73),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(74,  (uint32_t)&INTR(74),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(75,  (uint32_t)&INTR(75),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(76,  (uint32_t)&INTR(76),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(77,  (uint32_t)&INTR(77),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(78,  (uint32_t)&INTR(78),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(79,  (uint32_t)&INTR(79),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(70,  (uint32_t)&INTR(80),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(81,  (uint32_t)&INTR(81),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(82,  (uint32_t)&INTR(82),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(83,  (uint32_t)&INTR(83),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(84,  (uint32_t)&INTR(84),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(85,  (uint32_t)&INTR(85),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(86,  (uint32_t)&INTR(86),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(87,  (uint32_t)&INTR(87),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(88,  (uint32_t)&INTR(88),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(89,  (uint32_t)&INTR(89),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(90,  (uint32_t)&INTR(90),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(91,  (uint32_t)&INTR(91),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(92,  (uint32_t)&INTR(92),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(93,  (uint32_t)&INTR(93),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(94,  (uint32_t)&INTR(94),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(95,  (uint32_t)&INTR(95),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(96,  (uint32_t)&INTR(96),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(97,  (uint32_t)&INTR(97),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(98,  (uint32_t)&INTR(98),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(99,  (uint32_t)&INTR(99),  GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(100, (uint32_t)&INTR(100), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(101, (uint32_t)&INTR(101), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(102, (uint32_t)&INTR(102), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(103, (uint32_t)&INTR(103), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(104, (uint32_t)&INTR(104), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(105, (uint32_t)&INTR(105), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(106, (uint32_t)&INTR(106), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(107, (uint32_t)&INTR(107), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(108, (uint32_t)&INTR(108), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(109, (uint32_t)&INTR(109), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(110, (uint32_t)&INTR(110), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(111, (uint32_t)&INTR(111), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(112, (uint32_t)&INTR(112), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(113, (uint32_t)&INTR(113), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(114, (uint32_t)&INTR(114), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(115, (uint32_t)&INTR(115), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(116, (uint32_t)&INTR(116), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(117, (uint32_t)&INTR(117), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(118, (uint32_t)&INTR(118), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(119, (uint32_t)&INTR(119), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(120, (uint32_t)&INTR(120), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(121, (uint32_t)&INTR(121), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(122, (uint32_t)&INTR(122), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(123, (uint32_t)&INTR(123), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(124, (uint32_t)&INTR(124), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(125, (uint32_t)&INTR(125), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(126, (uint32_t)&INTR(126), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(127, (uint32_t)&INTR(127), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(128, (uint32_t)&INTR(128), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(129, (uint32_t)&INTR(129), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(130, (uint32_t)&INTR(130), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(131, (uint32_t)&INTR(131), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(132, (uint32_t)&INTR(132), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(133, (uint32_t)&INTR(133), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(134, (uint32_t)&INTR(134), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(135, (uint32_t)&INTR(135), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(136, (uint32_t)&INTR(136), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(137, (uint32_t)&INTR(137), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(138, (uint32_t)&INTR(138), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(139, (uint32_t)&INTR(139), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(140, (uint32_t)&INTR(140), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(141, (uint32_t)&INTR(141), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(142, (uint32_t)&INTR(142), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(143, (uint32_t)&INTR(143), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(144, (uint32_t)&INTR(144), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(145, (uint32_t)&INTR(145), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(146, (uint32_t)&INTR(146), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(147, (uint32_t)&INTR(147), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(148, (uint32_t)&INTR(148), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(149, (uint32_t)&INTR(149), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(150, (uint32_t)&INTR(150), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(151, (uint32_t)&INTR(151), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(152, (uint32_t)&INTR(152), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(153, (uint32_t)&INTR(153), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(154, (uint32_t)&INTR(154), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(155, (uint32_t)&INTR(155), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(156, (uint32_t)&INTR(156), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(157, (uint32_t)&INTR(157), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(158, (uint32_t)&INTR(158), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(159, (uint32_t)&INTR(159), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(160, (uint32_t)&INTR(160), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(161, (uint32_t)&INTR(161), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(162, (uint32_t)&INTR(162), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(163, (uint32_t)&INTR(163), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(164, (uint32_t)&INTR(164), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(165, (uint32_t)&INTR(165), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(166, (uint32_t)&INTR(166), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(167, (uint32_t)&INTR(167), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(168, (uint32_t)&INTR(168), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(169, (uint32_t)&INTR(169), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(160, (uint32_t)&INTR(170), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(171, (uint32_t)&INTR(171), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(172, (uint32_t)&INTR(172), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(173, (uint32_t)&INTR(173), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(174, (uint32_t)&INTR(174), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(175, (uint32_t)&INTR(175), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(176, (uint32_t)&INTR(176), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(177, (uint32_t)&INTR(177), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(178, (uint32_t)&INTR(178), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(179, (uint32_t)&INTR(179), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(170, (uint32_t)&INTR(180), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(181, (uint32_t)&INTR(181), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(182, (uint32_t)&INTR(182), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(183, (uint32_t)&INTR(183), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(184, (uint32_t)&INTR(184), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(185, (uint32_t)&INTR(185), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(186, (uint32_t)&INTR(186), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(187, (uint32_t)&INTR(187), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(188, (uint32_t)&INTR(188), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(189, (uint32_t)&INTR(189), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(190, (uint32_t)&INTR(190), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(191, (uint32_t)&INTR(191), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(192, (uint32_t)&INTR(192), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(193, (uint32_t)&INTR(193), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(194, (uint32_t)&INTR(194), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(195, (uint32_t)&INTR(195), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(196, (uint32_t)&INTR(196), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(197, (uint32_t)&INTR(197), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(198, (uint32_t)&INTR(198), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(199, (uint32_t)&INTR(199), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(200, (uint32_t)&INTR(200), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(201, (uint32_t)&INTR(201), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(202, (uint32_t)&INTR(202), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(203, (uint32_t)&INTR(203), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(204, (uint32_t)&INTR(204), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(205, (uint32_t)&INTR(205), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(206, (uint32_t)&INTR(206), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(207, (uint32_t)&INTR(207), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(208, (uint32_t)&INTR(208), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(209, (uint32_t)&INTR(209), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(210, (uint32_t)&INTR(210), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(211, (uint32_t)&INTR(211), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(212, (uint32_t)&INTR(212), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(213, (uint32_t)&INTR(213), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(214, (uint32_t)&INTR(214), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(215, (uint32_t)&INTR(215), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(216, (uint32_t)&INTR(216), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(217, (uint32_t)&INTR(217), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(218, (uint32_t)&INTR(218), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(219, (uint32_t)&INTR(219), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(220, (uint32_t)&INTR(220), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(221, (uint32_t)&INTR(221), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(222, (uint32_t)&INTR(222), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(223, (uint32_t)&INTR(223), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(224, (uint32_t)&INTR(224), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(225, (uint32_t)&INTR(225), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(226, (uint32_t)&INTR(226), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(227, (uint32_t)&INTR(227), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(228, (uint32_t)&INTR(228), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(229, (uint32_t)&INTR(229), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(230, (uint32_t)&INTR(230), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(231, (uint32_t)&INTR(231), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(232, (uint32_t)&INTR(232), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(233, (uint32_t)&INTR(233), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(234, (uint32_t)&INTR(234), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(235, (uint32_t)&INTR(235), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(236, (uint32_t)&INTR(236), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(237, (uint32_t)&INTR(237), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(238, (uint32_t)&INTR(238), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(239, (uint32_t)&INTR(239), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(240, (uint32_t)&INTR(240), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(241, (uint32_t)&INTR(241), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(242, (uint32_t)&INTR(242), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(243, (uint32_t)&INTR(243), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(244, (uint32_t)&INTR(244), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(245, (uint32_t)&INTR(245), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(246, (uint32_t)&INTR(246), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(247, (uint32_t)&INTR(247), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(248, (uint32_t)&INTR(248), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(249, (uint32_t)&INTR(249), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(250, (uint32_t)&INTR(250), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(251, (uint32_t)&INTR(251), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(252, (uint32_t)&INTR(252), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(253, (uint32_t)&INTR(253), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(254, (uint32_t)&INTR(254), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __intr_set_entry(255, (uint32_t)&INTR(255), GDT_KERNEL_TEXT, IDT_DESC_PRESENT | IDT_DESC_BIT32 | IDT_DESC_RING0);
        __asm__("lidt (%0)" :: "p"(data));

        apic_setspur(INTR_SPURIOUS);

        intr_register(INTR_SPURIOUS, __intr_spurious);
        intr_register(INTR_DIVIDE_BY_ZERO, __intr_divide_by_zero_handler);
        intr_register(INTR_GPF, __intr_gpf_handler);
        intr_register(INTR_INVALID_OPCODE, __intr_inval_opcode_handler);
}
