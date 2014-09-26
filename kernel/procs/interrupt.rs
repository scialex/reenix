// TODO Copyright Header

use core::prelude::*;
use core::intrinsics;
use startup::gdt;
use super::apic;

#[repr(C)]
#[deriving(Clone, Show)]
pub struct Registers {
    es   : u32, ds  : u32, gs : u32, /* Pushed manually */
    edi  : u32, esi : u32, ebp: u32, esp : u32, ebx : u32, ecx : u32, eax : u32, /* pushed by pusha */
    intr : u32, err : u32, /* Interrupt number and error code */
    eip  : u32, cs  : u32, eflags : u32, useresp : u32, ss : u32, /* pushed by the processor automatically */
}

pub static MAX_INTERRUPTS : u16 = 256;
pub static SPURIOUS       : u8  = 0xEF;

pub static LOW  : u8 = 0;
pub static HIGH : u8 = 0xff;

pub static DIVIDE_BY_ZERO : u8 = 0x00;
pub static INVALID_OPCODE : u8 = 0x06;
pub static GPF            : u8 = 0x0d;
pub static PAGE_FAULT     : u8 = 0x0e;
pub static SYSCALL        : u8 = 0x2e;
pub static PIT            : u8 = 0xf1;
pub static APICTIMER      : u8 = 0xf0;
pub static KEYBOARD       : u8 = 0xe0;
pub static DISK_PRIMARY   : u8 = 0xd0;
pub static DISK_SECONDARY : u8 = 0xd1;

/**
 * Enable interupts
 */
#[inline]
pub fn enable() {
    unsafe { asm!("sti" : : : : "volatile"); }
}

/**
 * Disable interupts
 */
#[inline]
pub fn disable() {
    unsafe { asm!("cli" : : : : "volatile"); }
}

/**
 * Atomically enables interrupts using the sti
 * instruction and puts the processor into a halted
 * state, this function returns once an interrupt
 * occurs.
 */
#[inline]
pub fn wait() {
    unsafe  {
        /* the sti instruction enables interrupts, however
         * interrupts are not checked for until the next
         * instruction is executed, this means that the following
         * code will not be succeptible to a bug where an
         * interrupt occurs between the sti and hlt commands
         * and does not wake us up from the hlt. */
        asm!("sti; hlt" : : : : "volatile");
    }
}

/**
 * Sets the interrupt priority level for hardware interrupts.
 * At initialization time devices should detect their individual
 * IPLs and save them for use with this function. IPL_LOW allows
 * all hardware interrupts. IPL_HIGH blocks all hardware interrupts
 */
#[inline]
pub fn get_ipl() -> u8 {
    unsafe { apic::get_ipl() }
}

/**
 * Retreives the current interrupt priority level.
 */
#[inline]
pub fn set_ipl(ipl: u8) {
    unsafe { apic::set_ipl(ipl); }
}

#[repr(C, packed)]
struct InterruptDescription {
    baselo   : u16,
    selector : u16,
    zero     : u8,
    attr     : interrupt_attr::Attr,
    basehi   : u16,
}

#[repr(C, packed)]
struct InterruptInfo {
    size : u16,
    base : *const InterruptDescription,
}

pub mod interrupt_attr {
    pub type Attr = u8;
    pub static TRAP    : Attr = 0x01;
    pub static BIT16   : Attr = 0x06;
    pub static BIT32   : Attr = 0x0E;
    pub static RING0   : Attr = 0x00;
    pub static RING1   : Attr = 0x40;
    pub static RING2   : Attr = 0x20;
    pub static RING3   : Attr = 0x60;
    pub static PRESENT : Attr = 0x80;
}

pub type InterruptHandler = unsafe extern fn(&Registers);

#[allow(unused_unsafe)]
#[no_split_stack]
unsafe extern fn unhandled_intr(r: &Registers) {
    panic!("Unhandled interrupt 0x{:X}", r.intr);
}

static mut IDT : InterruptState<'static> = InterruptState {
    table    : [InterruptDescription { baselo : 0, selector: 0, zero: 0, attr: 0, basehi: 0 }, ..MAX_INTERRUPTS as uint],
    handlers : [unhandled_intr, ..MAX_INTERRUPTS as uint],
    mappings : [None, ..MAX_INTERRUPTS as uint], 
    data     : InterruptInfo { size: 0, base : 0 as *const InterruptDescription }
};

pub struct InterruptState<'a> {
    table    : [InterruptDescription, ..MAX_INTERRUPTS as uint],
    handlers : [InterruptHandler, ..MAX_INTERRUPTS as uint],
    mappings : [Option<u16>, ..MAX_INTERRUPTS as uint],
    data     : InterruptInfo,
}

macro_rules! make_panic_handler(
    ($int:ident) => ({
        #[allow(unused_unsafe)]
        #[no_split_stack]
        unsafe extern fn die(r: &Registers) {
            panic!(concat!("Recieved a ", stringify!($int), " interrupt (0x{:X}). Aborting"), r.intr);
        }
        register($int, die);
    })
)

unsafe fn set_entry(isr: u8, addr: u32, seg: u16, flags: u8) {
    IDT.table[isr as uint].basehi = (addr & 0xffff) as u16;
    IDT.table[isr as uint].baselo = ((addr >> 16) & 0xffff) as u16;
    IDT.table[isr as uint].zero   = 0;
    IDT.table[isr as uint].attr   = flags;
    IDT.table[isr as uint].selector = seg;
}

/// This is the function that is actually initially entered by the interrupt handler.
#[no_mangle]
#[no_split_stack]
#[inline(never)]
unsafe extern "C" fn _rust_intr_handler(r: Registers) {
    // TODO I might need to setup the %es stuff as early as here.
    let h = IDT.handlers[r.intr as uint];
    h(&r);
    if IDT.mappings[r.intr as uint].is_none() {
        apic::set_eoi();
    }
}

macro_rules! make_noerr_handlers (
    ($(($num:expr, $seg:expr, $flag:expr)),*) => ({
        #[allow(unused)]
        #[no_split_stack]
        #[inline(never)]
        unsafe extern fn intr_entry() -> ! {
            asm!("
            .global _rust_intr_handlers_global
            _rust_intr_handler_global:
            pusha
            push %gs
            push %ds
            push %es
            movl %ss, %edx
            movl %edx, %ds
            movl %edx, %es
            call _rust_intr_handler
            pop %es
            pop %ds
            pop %gs
            popa
            add $$8, %esp
            iret
            " : : : : "volatile");
            $(
                asm!(concat!("
                .global _rust_intr_handler_", stringify!($num),"
                _rust_intr_handler_", stringify!($num),":
                push $$0
                push $$", stringify!($num), "
                jmp _rust_intr_handler_global
                ") : : : : "volatile");
            )*
            asm!("
                 .global _rust_intr_handlers_end
                 _rust_intr_handlers_end:
                 cli
                 hlt
                 "::::"volatile");

            unreachable!();
        }

        $({
            let x: u32;
            asm!(concat!("movl $$_rust_intr_handler_",stringify!($num),", $0") : "=r"(x) : : : "volatile");
            set_entry($num, x, $seg, $flag);
         })*
    })
)

macro_rules! make_err_handlers (
    ($(($num:expr, $seg:expr, $flag:expr)),*) => ({
        #[allow(unused)]
        #[no_split_stack]
        #[inline(never)]
        unsafe extern fn intr_entry_noerr() -> ! {
            $(
                asm!(concat!("
                .global _rust_intr_handler_err_", stringify!($num),"
                _rust_intr_handler_err_", stringify!($num),":
                push $$", stringify!($num), "
                jmp _rust_intr_handler_global
                ") : : : : "volatile");
            )*
            asm!("
                 .global _rust_intr_handlers_err_end
                 _rust_intr_handlers_err_end:
                 cli
                 hlt
                 "::::"volatile");

            unreachable!();
        }

        $({
            let x: u32;
            asm!(concat!("movl $$_rust_intr_handler_err_",stringify!($num),", $0") : "=r"(x) : : : "volatile");
            set_entry($num, x, $seg, $flag);
         })*
    })
)

pub fn init_stage1() {
    unsafe {
        IDT.data = InterruptInfo {
                size: intrinsics::size_of::<InterruptInfo>() as u16,
                base: &IDT.table[0],
            };
    }
    unsafe {
        make_noerr_handlers!(
            (1,   gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (2,   gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (3,   gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (4,   gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (5,   gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (6,   gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (7,   gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (9,   gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (15,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (16,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (18,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (19,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (20,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (21,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (22,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (23,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (24,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (25,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (26,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (27,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (28,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (29,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (30,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (31,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (32,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (33,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (34,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (35,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (36,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (37,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (38,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (39,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (40,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (41,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (42,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (43,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (44,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (45,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),

            // The Syscall entry interrupt has different flags
            (46,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::TRAP | interrupt_attr::RING3),

            (47,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (48,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (49,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (50,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (51,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (52,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (53,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (54,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (55,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (56,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (57,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (58,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (59,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (60,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (61,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (62,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (63,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (64,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (65,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (66,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (67,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (68,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (69,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (70,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (71,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (72,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (73,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (74,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (75,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (76,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (77,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (78,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (79,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (80,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (81,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (82,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (83,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (84,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (85,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (86,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (87,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (88,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (89,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (90,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (91,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (92,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (93,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (94,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (95,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (96,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (97,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (98,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (99,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (100, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (101, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (102, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (103, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (104, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (105, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (106, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (107, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (108, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (109, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (110, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (111, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (112, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (113, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (114, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (115, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (116, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (117, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (118, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (119, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (120, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (121, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (122, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (123, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (124, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (125, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (126, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (127, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (128, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (129, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (130, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (131, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (132, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (133, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (134, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (135, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (136, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (137, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (138, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (139, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (140, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (141, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (142, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (143, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (144, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (145, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (146, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (147, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (148, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (149, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (150, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (151, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (152, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (153, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (154, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (155, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (156, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (157, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (158, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (159, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (160, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (161, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (162, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (163, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (164, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (165, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (166, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (167, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (168, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (169, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (170, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (171, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (172, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (173, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (174, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (175, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (176, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (177, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (178, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (179, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (180, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (181, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (182, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (183, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (184, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (185, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (186, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (187, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (188, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (189, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (190, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (191, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (192, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (193, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (194, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (195, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (196, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (197, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (198, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (199, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (200, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (201, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (202, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (203, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (204, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (205, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (206, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (207, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (208, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (209, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (210, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (211, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (212, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (213, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (214, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (215, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (216, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (217, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (218, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (219, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (220, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (221, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (222, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (223, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (224, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (225, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (226, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (227, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (228, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (229, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (230, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (231, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (232, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (233, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (234, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (235, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (236, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (237, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (238, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (239, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (240, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (241, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (242, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (243, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (244, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (245, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (246, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (247, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (248, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (249, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (250, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (251, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (252, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (253, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (254, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (255, gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0));
        make_err_handlers!(
            (8,   gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (10,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (11,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (12,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (13,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (14,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0),
            (17,  gdt::KERNEL_TEXT, interrupt_attr::PRESENT | interrupt_attr::BIT32 | interrupt_attr::RING0));
    }
    unsafe { asm!("lidt ($0)" : : "r"(&IDT.data)); }
    dbg!(debug::MM, "pre apic");
    unsafe { apic::set_spurious_interrupt(SPURIOUS); }
    dbg!(debug::MM, "post apic");
    make_panic_handler!(SPURIOUS);
    dbg!(debug::MM, "post spurious");
    make_panic_handler!(DIVIDE_BY_ZERO);
    make_panic_handler!(GPF);
    make_panic_handler!(INVALID_OPCODE);
}

pub fn init_stage2() {}

/**
 * Registers an interrupt handler for the given interrupt handler.
 * If another handler had been previously registered for this interrupt
 * it is returned, otherwise this function returns `None`. It
 * is good practice to assert that this function returns `None` unless
 * it is known that this will not be the case.
 */
pub fn register(intr: u8, handler: InterruptHandler) -> Option<InterruptHandler> {
    use core::intrinsics::transmute;
    unsafe {
        let old = IDT.handlers[intr as uint];
        IDT.handlers[intr as uint] = handler;
        let handled = transmute::<InterruptHandler, *const u8>(old) != transmute::<InterruptHandler, *const u8>(unhandled_intr);
        return if handled { Some(old) } else { None };
    }
}

pub fn map(irq: u16, intr: u8) -> Option<u16> {
    assert!(SPURIOUS != intr, "Should not redirect spurious interrupt");
    unsafe {
        let old = IDT.mappings[intr as uint];
        IDT.mappings[intr as uint] = Some(irq);
        apic::set_redirect(irq as u32, intr);
        return old;
    }
}

