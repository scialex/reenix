#include "util/debug.h"
#include "util/string.h"
#include "util/printf.h"

#include "main/io.h"
#include "main/interrupt.h"

#include "kernel.h"

/* Below is a truly terrible poll-driven serial driver that we use for debugging
 * purposes - it outputs to COM1, but
 * this can be easily changed. It does not use interrupts, and cannot read input
 * */
/* This port is COM1 */
#define PORT 0x3f8
/* Corresponding interrupt vector */
#define PORT_INTR 0x0d

/* Special port available on Bochs emulator only. Any characters
 * written to this port are printed to the Bochs log. We use this
 * instead of a serial driver when available because it is faster */
#define PORT_BOCHS 0xe9

uint64_t dbg_modes;

typedef struct dbg_mode {
        const char *d_name;
        uint64_t d_mode;
        const char *d_color;
} dbg_mode_t;

static int dbg_supports_bochs_e9_hack;

void dbg_init()
{

        /* Test host machine compatibility with with bochs e9 hack. */
        dbg_supports_bochs_e9_hack = (PORT_BOCHS == inb(PORT_BOCHS));

        if (!dbg_supports_bochs_e9_hack) {
                outb(PORT + 3, 0x80);   /* Enable DLAB (set baud rate divisor) */
                outb(PORT + 0, 0x03);   /* Set divisor to 3 (lo byte) 38400 baud */
                outb(PORT + 1, 0x00);   /*                  (hi byte) */
                outb(PORT + 3, 0x03);   /* 8 bits, no parity, one stop bit */
                outb(PORT + 2, 0xC7);   /* Enable FIFO, clear them, with 14-byte threshold */
        }

        dbg_modes = DBG_DEFAULT;
        dbg_add_modes(QUOTE(__DBG__));
}

static dbg_mode_t dbg_tab[] = {
        DBG_TAB
};

const char *dbg_color(uint64_t d_mode)
{
        dbg_mode_t *mode;
        for (mode = dbg_tab; mode->d_mode != 0UL; mode++) {
                if (mode->d_mode & d_mode)
                        return mode->d_color;
        }
        /* If we get here, something went seriously wrong */
        panic("Unknown debug mode 0x%llx\n", d_mode);
        return NULL;
}

static void dbg_puts(char *c)
{
        if (dbg_supports_bochs_e9_hack) {
                while (*c != '\0') {
                        outb(PORT_BOCHS, *c++);
                }
        } else {
                while (*c != '\0') {
                        /* Wait until the port is free */
                        while (!(inb(PORT + 5) & 0x20));
                        outb(PORT, *c++);
                }
        }
}

#define BUFFER_SIZE 1024
void dbg_print(char *fmt, ...)
{
        va_list args;
        char buf[BUFFER_SIZE];
        int count;

        va_start(args, fmt);
        count = vsnprintf(buf, BUFFER_SIZE, fmt, args);
        va_end(args);

        if (count >= BUFFER_SIZE) {
                dbg_puts("WARNING: The following message has been "
                         " truncated due to buffer size limitations.\n");
        }
        dbg_puts(buf);
}

void dbg_printinfo(dbg_infofunc_t func, const void *data)
{
        char buf[BUFFER_SIZE];
        func(data, buf, BUFFER_SIZE);
        dbg_puts(buf);
}

/**
 * searches for <code>name</code> in the list of known
 * debugging modes specified above and, if it
 * finds <code>name</code>, adds the corresponding
 * debugging mode to a list
 */
void dbg_add_mode(const char *name)
{
        int cancel;
        dbg_mode_t *mode;

        if (*name == '-') {
                cancel = 1;
                name++;
        } else {
                cancel = 0;
        }

        for (mode = dbg_tab; mode->d_name != NULL; mode++)
                if (strcmp(name, mode->d_name) == 0)
                        break;
        if (mode->d_name == NULL) {
                dbg_print("Warning: Unknown debug option: \"%s\"\n", name);
                return;
        }

        if (cancel) {
                dbg_modes &= ~mode->d_mode;
        } else {
                dbg_modes |= mode->d_mode;
        }

}

/**
 * Cycles through each comma-delimited debugging option and
 * adds it to the debugging modes by calling dbg_add_mode
 */
void dbg_add_modes(const char *modes)
{
        char env[256];
        char *name;

        strncpy(env, modes, sizeof(env));
        /* Maybe it would be good if we did this without strtok, but I'm too lazy */
        for (name = strtok(env, ","); name; name = strtok(NULL, ","))
                dbg_add_mode(name);
}

size_t dbg_modes_info(const void *data, char *buf, size_t size)
{
        KASSERT(NULL == data);
        KASSERT(0 < size);

        size_t osize = size;

        dbg_mode_t *mode;
        for (mode = dbg_tab; mode->d_name != NULL; ++mode) {
                if (dbg_modes &mode->d_mode && mode->d_mode != DBG_ALL) {
                        int len;
                        if ((len = snprintf(buf, size, "%s,", mode->d_name)) >= (int)size) {
                                break;
                        } else {
                                buf += len;
                                size -= len;
                        }
                }
        }

        if (size == osize) {
                buf[0] = '\0';
                return 0;
        } else {
                /* remove trailing comma */
                buf[-1] = '\0';
                return osize - size + 1;
        }
}

/* This is meant as a good point to automatically set a breakpoint which will stop
 * just after a panic has occured and printed its message. */
static void dbg_panic_halt()
{
        __asm__ volatile("cli; hlt");
}

#define PANIC_BUFSIZE 2048
void dbg_panic(const char *file, int line, const char *func, const char *fmt, ...)
{
        char buf[PANIC_BUFSIZE];
        va_list args;
        va_start(args, fmt);

        dbg_print("panic in %s:%u %s(): ", file, line, func);
        vsnprintf(buf, PANIC_BUFSIZE, fmt, args);
        dbg_print("%s", buf);
        dbg_print("\nKernel Halting.\n\n");

        va_end(args);

        dbg_panic_halt();
}
