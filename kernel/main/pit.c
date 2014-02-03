#include "globals.h"

#include "main/io.h"
#include "main/interrupt.h"
#include "util/delay.h"
#include "main/apic.h"

#include "proc/sched.h"
#include "proc/kthread.h"

#define APIC_TIMER_IRQ 32 /* Map interrupt 32 */

/* IRQ */
/*#define PIT_IRQ 0*/

/* I/O ports */
/*#define PIT_DATA0 0x40
#define PIT_DATA1 0x41
#define PIT_DATA2 0x42
#define PIT_CMD   0x43

#define CLOCK_TICK_RATE 1193182
#undef HZ
#define HZ 1000

#define LATCH (CLOCK_TICK_RATE / HZ)*/
static unsigned int ms = 0;

void pit_handler(regs_t* regs) {
}

void pit_init(uint8_t intr) {
}
