#include "drivers/tty/screen.h"

#include "drivers/tty/tty.h"
#include "drivers/tty/virtterm.h"

#include "main/io.h"

#include "mm/pagetable.h"

#include "util/debug.h"
#include "util/string.h"


/* Location of directly mapped video memory (by default) */
/* Note that this is a short * as video memory is addressed in 2-byte chunks -
 * 1st byte is attributes, 2nd byte is char */
#define PHYS_VIDEORAM 0xb8000
/* Port addresses for the CRT controller */
#define CRT_CONTROL_ADDR 0x3d4
#define CRT_CONTROL_DATA 0x3d5

/* Addresses we can pass to the CRT_CONTROLL_ADDR port */
#define CURSOR_HIGH 0x0e
#define CURSOR_LOW 0x0f
/* Right now, we shouldn't need cursor high to change from zero */

/* Default attribs */
#define DEFAULT_ATTRIB 0x0F

/* This is basically a "logic-free" file - it interfaces directly with the
 * hardware, but higher-level terminal output logic will be dealt with elsewhere
 * */

static uint16_t *videoram;

/* Needs to get a virtual memory mapping for video memory */
void
screen_init()
{
        videoram = (uint16_t *) pt_phys_perm_map(PHYS_VIDEORAM, 1);
}

/* Copied from OSDev */
void
screen_move_cursor(uint8_t x, uint8_t y)
{
        /* Commented out until we have kasserts */
        /* KASSERT(cursor_col < DISPLAY_WIDTH && cursor_row < DISPLAY_HEIGHT); */
        uint16_t pos = y * DISPLAY_WIDTH + x;

        outb(CRT_CONTROL_ADDR, CURSOR_HIGH);
        outb(CRT_CONTROL_DATA, pos >> 8);

        /*  Output address being modified */
        outb(CRT_CONTROL_ADDR, CURSOR_LOW);
        /* New position of cursor */
        outb(CRT_CONTROL_DATA, pos & 0xff);
}

void
screen_putchar(char c, uint8_t x, uint8_t y)
{
        /* Update the character at the current cursor position, using the default
         * attributes */
        *(videoram + (y * DISPLAY_WIDTH + x)) = (DEFAULT_ATTRIB << 8) | c;
}

void
screen_putchar_attrib(char c, uint8_t x, uint8_t y, uint8_t attrib)
{
        /* Similarly, but with custom attributes */
        *(videoram + (y * DISPLAY_WIDTH + x)) = (attrib << 8) | c;
}

void
screen_putbuf(const char *buf)
{
        uint16_t *pos;
        for (pos = videoram; pos - videoram < DISPLAY_WIDTH * DISPLAY_HEIGHT; buf++, pos++)
                *pos = (DEFAULT_ATTRIB << 8) | *buf;
}

/* In theory, this one should be much faster, but it probably isn't */
void
screen_putbuf_attrib(const uint16_t *buf)
{
        memcpy(videoram, buf, DISPLAY_WIDTH * DISPLAY_HEIGHT * 2);
}

void
screen_clear()
{
        /* Attributes: 0x0F (white on black) with character 0x20 (space) */
        /* Note that using a null character instead of a space is OK, but we can't
         * use a memcpy b/c attribute 0x00 is black on black (which messes up the
         * attribute settings . . . ) */
        uint16_t blank = (DEFAULT_ATTRIB << 8) | 0x20;
        uint16_t *pos;
        for (pos = videoram; pos - videoram < DISPLAY_WIDTH * DISPLAY_HEIGHT; pos++)
                *pos = blank;
}
