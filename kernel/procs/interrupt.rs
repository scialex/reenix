// TODO Copyright Header

//! All things interrupts go here.

use core::prelude::*;
use core::intrinsics;
use startup::gdt;
use super::apic;

/// A struct containing the register state when a interrupt function is called. Note that modifying
/// this structure in an ISR will change the state of the registers once the function returns to
/// the source of the interrupt.
#[allow(missing_copy_implementations)]
#[repr(C)]
#[derive(Clone, Show)]
pub struct Registers {
    pub es   : u32, pub ds  : u32, pub gs  : u32,                                     /* Pushed manually */
    pub edi  : u32, pub esi : u32, pub ebp : u32, pub esp : u32,                      /* pushed by pusha */
    pub ebx  : u32, pub edx : u32, pub ecx : u32, pub eax : u32,                      /* pushed by pusha */
    pub intr : u32, pub err : u32,                                                    /* Interrupt number and error code */
    pub eip  : u32, pub cs  : u32, pub eflags : u32, pub useresp : u32, pub ss : u32, /* pushed by the processor automatically */
}

/// The total number of interrupts we can use.
pub const MAX_INTERRUPTS : u16 = 256;

/// All Interrupts enabled.
pub const LOW  : u8 = 0;
/// All Interrupts disabled
pub const HIGH : u8 = 0xFF;

pub const DIVIDE_BY_ZERO : u8 = 0x00;
pub const INVALID_OPCODE : u8 = 0x06;
pub const GPF            : u8 = 0x0d;
pub const PAGE_FAULT     : u8 = 0x0e;
pub const SYSCALL        : u8 = 0x2e;
pub const PIT            : u8 = 0xf1;
pub const APICTIMER      : u8 = 0xf0;
pub const KEYBOARD       : u8 = 0xe0;
pub const DISK_PRIMARY   : u8 = 0xd0;
pub const DISK_SECONDARY : u8 = 0xd1;
pub const SPURIOUS       : u8 = 0xef;

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

pub struct IPLWatchdog { oldipl: u8, }
pub fn temporary_ipl(ipl: u8) -> IPLWatchdog {
    let oldipl = get_ipl();
    set_ipl(ipl);
    dbg!(debug::INTR, "Setting ipl to {}", ipl);
    IPLWatchdog { oldipl: oldipl }
}

impl IPLWatchdog {
    pub fn set_ipl(&mut self, ipl: u8) { set_ipl(ipl); }
    pub fn reset_ipl(&mut self) { set_ipl(self.oldipl); }
}
impl Drop for IPLWatchdog { fn drop(&mut self) { dbg!(debug::INTR, "reseting ipl to {}", self.oldipl); self.reset_ipl(); } }

#[unsafe_no_drop_flag]
#[repr(C, packed)]
#[derive(Copy)]
struct InterruptDescription {
    baselo   : u16,
    selector : u16,
    zero     : u8,
    attr     : u8,
    basehi   : u16,
}

#[repr(C, packed)]
struct InterruptInfo {
    size : u16,
    base : *const InterruptDescription,
}

const TRAP    : u8 = 0x01;
#[allow(dead_code)]
const BIT16   : u8 = 0x06;
const BIT32   : u8 = 0x0E;
const RING0   : u8 = 0x00;
#[allow(dead_code)]
const RING1   : u8 = 0x40;
#[allow(dead_code)]
const RING2   : u8 = 0x20;
const RING3   : u8 = 0x60;
const PRESENT : u8 = 0x80;

/// A rust Interrupt Service Routine (ISR). It is called with a pointer to a copy of the registers
/// as they appeared before sending this interrupt. Note that mutateing the registers _WILL_ mutate
/// the registers on return of the function.
pub type InterruptHandler = extern "Rust" fn(&mut Registers);

#[allow(unused_unsafe)]
#[no_stack_check]
pub extern "Rust" fn unhandled_intr(r: &mut Registers) {
    kpanic!("Unhandled interrupt 0x{:X}.\nRegisters were {:?}\nProcess was {:?}\nThread was {:?}",
           r.intr, r, current_proc!(), current_thread!());
}

static mut IDT : InterruptState<'static> = InterruptState {
    table    : [InterruptDescription { baselo : 0, selector: 0, zero: 0, attr: 0, basehi: 0 }; MAX_INTERRUPTS as usize],
    handlers : [unhandled_intr; MAX_INTERRUPTS as usize],
    mappings : [None; MAX_INTERRUPTS as usize],
    data     : InterruptInfo { size: 0, base : 0 as *const InterruptDescription }
};

pub struct InterruptState<'a> {
    table    : [InterruptDescription; MAX_INTERRUPTS as usize],
    handlers : [InterruptHandler; MAX_INTERRUPTS as usize],
    mappings : [Option<u16>; MAX_INTERRUPTS as usize],
    data     : InterruptInfo,
}

/// This just makes a handler which kpanics with a custom message.
macro_rules! make_panic_handler{
    ($int:ident) => ({
        #[allow(unused_unsafe)]
        #[no_stack_check]
        extern "Rust" fn die(r: &mut Registers) {
            kpanic!(concat!("Recieved a ", stringify!($int), " interrupt (0x{:X}). Aborting"), r.intr);
        }
        register($int, die);
    })
}

/// Set the given entry in the IDT.
unsafe fn set_entry(isr: u8, addr: u32, seg: u16, flags: u8) {
    IDT.table[isr as usize] = InterruptDescription {
        baselo   : (addr & 0xffff) as u16,
        basehi   : ((addr >> 16) & 0xFFFF) as u16,
        zero     : 0,
        attr     : flags,
        selector : seg,
    };
}

/// This is the function that is actually initially entered by the interrupt handler. It should
/// never be called directly. It is public only so that the compiler will not remove this for being
/// dead code.
#[no_mangle]
#[no_stack_check]
#[inline(never)]
#[allow(dead_code)]
pub unsafe extern "C" fn _rust_intr_handler(r: &mut Registers) {
    // TODO I might need to setup the %es stuff as early as here.
    let h = IDT.handlers[r.intr as usize];
    h(r);
    if IDT.mappings[r.intr as usize].is_some() {
        apic::set_eoi();
    }
}

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
        let old = IDT.handlers[intr as usize];
        IDT.handlers[intr as usize] = handler;
        let handled = transmute::<InterruptHandler, *const u8>(old) != transmute::<InterruptHandler, *const u8>(unhandled_intr);
        return if handled { Some(old) } else { None };
    }
}

/// Redirects the given irq to the given interrupt.
pub fn map(irq: u16, intr: u8) -> Option<u16> {
    assert!(SPURIOUS != intr, "Should not redirect spurious interrupt");
    unsafe {
        let old = IDT.mappings[intr as usize];
        IDT.mappings[intr as usize] = Some(irq);
        apic::set_redirect(irq as u32, intr);
        return old;
    }
}

macro_rules! make_noerr_handlers {
    ($(($num:expr, $seg:expr, $flag:expr)),*) => ({
        #[cfg(target_arch = "x86")]
        #[no_stack_check] #[inline(never)] #[no_mangle]
        unsafe extern "C" fn intr_entry() {
            // NOTE This is a huge hack to make sure that this code is not removed for being
            // unreachable
            dbg!(debug::CORE, "Calling into interrupt entry function to ensure it is not removed");
            asm!("jmp 2f":::: "volatile");
            asm!(concat!("
            .global _rust_intr_handler_global
            _rust_intr_handler_global:
            pusha
            push %gs
            push %ds
            push %es", /* Push each of the segment registers manually. */ "
            push %esp", /* Push the pointer to be used as the argument to the _rust_intr_handler function */ "
            movl %ss, %edx
            movl %edx, %ds
            movl %edx, %es
            movl $$0, %edx
            mov $$0x40, %dx
            mov %dx, %gs", /* Set up the segment registers appropriately */"
            call _rust_intr_handler", /* Call the rust interrupt handler function */ "
            pop %esp", /* Pop the argument off the stack */ "
            pop %es", /* Pop the segment regsters back off the stack */"
            pop %ds
            pop %gs
            popa", /* Pop the other registers off the stack */ "
            add $$8, %esp", /* Remove the interrupt number and error code from the stack */ "
            iret
            ") : : : : "volatile");
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
            asm!("
                 2:
                 nop
                 "::::"volatile");

            return;
        }
        intr_entry();

        $({
            let x: u32;
            asm!(concat!("leal _rust_intr_handler_",stringify!($num),", $0") : "=r"(x) : : : "volatile");
            set_entry($num, x, $seg, $flag);
         })*
    })
}

macro_rules! make_err_handlers {
    ($(($num:expr, $seg:expr, $flag:expr)),*) => ({
        #[no_stack_check]
        #[inline(never)]
        #[no_mangle]
        unsafe extern "C" fn intr_entry_noerr() {
            // NOTE This is a huge hack to make sure that this code is not removed for being
            // unreachable without declaring it public.
            dbg!(debug::CORE, "Calling into interrupt err entry function to ensure it is not removed");
            asm!("jmp 2f":::: "volatile");
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

            asm!("2: nop"::::"volatile");
            return;
        }
        intr_entry_noerr();

        $({
            let x: u32;
            asm!(concat!("movl _rust_intr_handler_err_",stringify!($num),", $0") : "=r"(x) : : : "volatile");
            set_entry($num, x, $seg, $flag);
         })*
    })
}

#[inline(never)]
pub fn init_stage1() {
    unsafe {
        IDT.data = InterruptInfo {
                size: (intrinsics::size_of::<InterruptDescription>() * IDT.table.len()) as u16 - 1 ,
                base: IDT.table.as_ptr(),
            };
    }
    unsafe {
        make_noerr_handlers!(
            (0,   gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (1,   gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (2,   gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (3,   gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (4,   gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (5,   gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (6,   gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (7,   gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (9,   gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (15,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (16,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (18,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (19,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (20,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (21,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (22,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (23,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (24,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (25,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (26,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (27,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (28,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (29,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (30,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (31,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (32,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (33,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (34,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (35,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (36,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (37,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (38,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (39,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (40,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (41,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (42,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (43,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (44,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (45,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),

            // The Syscall entry interrupt has different flags
            (46,  gdt::KERNEL_TEXT, PRESENT | BIT32 | TRAP | RING3),

            (47,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (48,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (49,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (50,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (51,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (52,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (53,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (54,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (55,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (56,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (57,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (58,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (59,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (60,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (61,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (62,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (63,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (64,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (65,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (66,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (67,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (68,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (69,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (70,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (71,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (72,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (73,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (74,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (75,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (76,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (77,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (78,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (79,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (80,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (81,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (82,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (83,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (84,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (85,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (86,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (87,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (88,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (89,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (90,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (91,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (92,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (93,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (94,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (95,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (96,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (97,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (98,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (99,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (100, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (101, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (102, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (103, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (104, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (105, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (106, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (107, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (108, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (109, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (110, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (111, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (112, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (113, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (114, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (115, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (116, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (117, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (118, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (119, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (120, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (121, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (122, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (123, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (124, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (125, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (126, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (127, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (128, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (129, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (130, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (131, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (132, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (133, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (134, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (135, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (136, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (137, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (138, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (139, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (140, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (141, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (142, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (143, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (144, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (145, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (146, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (147, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (148, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (149, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (150, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (151, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (152, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (153, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (154, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (155, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (156, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (157, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (158, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (159, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (160, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (161, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (162, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (163, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (164, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (165, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (166, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (167, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (168, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (169, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (170, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (171, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (172, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (173, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (174, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (175, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (176, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (177, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (178, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (179, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (180, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (181, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (182, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (183, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (184, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (185, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (186, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (187, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (188, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (189, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (190, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (191, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (192, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (193, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (194, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (195, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (196, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (197, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (198, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (199, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (200, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (201, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (202, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (203, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (204, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (205, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (206, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (207, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (208, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (209, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (210, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (211, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (212, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (213, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (214, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (215, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (216, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (217, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (218, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (219, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (220, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (221, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (222, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (223, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (224, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (225, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (226, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (227, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (228, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (229, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (230, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (231, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (232, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (233, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (234, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (235, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (236, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (237, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (238, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (239, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (240, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (241, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (242, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (243, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (244, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (245, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (246, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (247, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (248, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (249, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (250, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (251, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (252, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (253, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (254, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (255, gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0));
        make_err_handlers!(
            (8,   gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (10,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (11,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (12,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (13,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (14,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0),
            (17,  gdt::KERNEL_TEXT, PRESENT | BIT32 | RING0));
    }
    let ptr : *const InterruptInfo = unsafe { &IDT.data as *const InterruptInfo };
    unsafe { asm!("lidt ($0)" : : "r"(ptr)); }
    unsafe { apic::set_spurious_interrupt(SPURIOUS); }
    register(SPURIOUS, spurious_intr);
    make_panic_handler!(DIVIDE_BY_ZERO);
    make_panic_handler!(GPF);
    make_panic_handler!(INVALID_OPCODE);
}

pub fn init_stage2() {}

extern "Rust" fn spurious_intr(regs: &mut Registers) {
    dbg!(debug::INTR, "Ignoring spurious interrupt. Registers were {:?}. Process was {:?}. Thread was {:?}",
         regs, current_proc!(), current_thread!());
}

