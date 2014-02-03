#include "drivers/tty/virtterm.h"

#include "drivers/tty/driver.h"
#include "drivers/tty/keyboard.h"
#include "drivers/tty/screen.h"
#include "drivers/tty/tty.h"

#include "main/interrupt.h"

#include "mm/kmalloc.h"

#include "util/string.h"
#include "util/debug.h"

/* The number of virtual terminals */
#define NTERMS __NTERMS__

/* Total size of the scroll buffer */
#define SCROLL_BUFSIZE (5 * DISPLAY_SIZE)

#define driver_to_vt(driver) \
        CONTAINER_OF(driver, virtterm_t, vt_driver)

static void vt_provide_char(tty_driver_t *ttyd, char c);
static tty_driver_callback_t vt_register_callback_handler(
        tty_driver_t *ttyd,
        tty_driver_callback_t callback,
        void *arg);
static tty_driver_callback_t vt_unregister_callback_handler(
        tty_driver_t *driver);
static void *vt_block_io(tty_driver_t *ttyd);
static void  vt_unblock_io(tty_driver_t *ttyd, void *data);

static tty_driver_ops_t vt_driver_ops = {
        .provide_char                = vt_provide_char,
        .register_callback_handler   = vt_register_callback_handler,
        .unregister_callback_handler = vt_unregister_callback_handler,
        .block_io                    = vt_block_io,
        .unblock_io                  = vt_unblock_io
};

typedef struct virtterm {
        /*
         * head, tail, and top are row aligned
         */

        /* Current buffer head (circular buffer) */
        int vt_head;

        /* Current buffer tail (circular buffer) */
        int vt_tail;

        /* Top of screen (in current scroll buffer) */
        int vt_top;

        /* Cursor position (in buffer, not on screen) */
        int vt_cursor;

        /* scroll buffer */
        char vt_buf[SCROLL_BUFSIZE];

        /* "Temporary buffer" */
        char vt_tempbuf[DISPLAY_WIDTH *DISPLAY_HEIGHT];

        tty_driver_t vt_driver;
} virtterm_t;

static virtterm_t vt_terms[NTERMS];
static virtterm_t *vt_curterm;

/**
 * Called when a key is pressed. Sends the key press to the current
 * terminal if there is one.
 *
 * @param c the character pressed
 */
static void vt_keyboard_handler(char c);

/**
 * Puts a given character to a given virtual terminal, moving the
 * cursor accordingly and correctly handling special characters, such
 * as newline and backspace.
 *
 * Note that vt_handle_char puts the character into the virtual
 * terminals buffer, _NOT_ onto the screen.
 *
 * @return true if we put a new displayable character on the screen
 */
static int vt_handle_char(virtterm_t *vt, char c);

/**
 * Redraws the entire screen.
 */
static void vt_redraw();

/**
 * Redraws only the cursor.
 */
static void vt_cursor_redraw();

/* to and from are pointers into our circular buffer - this returns the
 * distance (including wraparound) between them */
#define circ_dist(to, from) \
        (((to) - (from) + SCROLL_BUFSIZE) % SCROLL_BUFSIZE)

/* Given a position (int) in the buffer, returns an int corresponding
 * to the first char in the next row of the buffer (using
 * DISPLAY_WIDTH) */
#define next_row(i) \
        ((((i) / DISPLAY_WIDTH + 1) * DISPLAY_WIDTH) % SCROLL_BUFSIZE)

/* Moves the given "pointer" into the buffer (really an int) by the
 * appropriate amount - works both forwards and backwards. Only works
 * on "moving" distances smaller than a SCROLL_BUFSIZE */
#define buf_add(ptr,amt) \
        (ptr = (((ptr) + (amt) + SCROLL_BUFSIZE) % SCROLL_BUFSIZE))

/* Surprisingly helpful */
#define buf_inc(ptr) buf_add(ptr,1)
#define buf_dec(ptr) buf_add(ptr,-1)


void
vt_init()
{
        /* Initialize NTERMS virtual terminals */
        int i;
        for (i = 0; i < NTERMS; i++) {
                memset(vt_terms[i].vt_buf, 0, SCROLL_BUFSIZE);
                vt_terms[i].vt_top = 0;
                vt_terms[i].vt_cursor = 0;
                vt_terms[i].vt_head = 0;

                /*
                 * Dealing with a silly memset bug - the problem is
                 * that if tail and head are ever the same, the first
                 * character we add will get overwritten when we
                 * memset after changing the tail.
                 */
                vt_terms[i].vt_tail = DISPLAY_WIDTH;

                vt_terms[i].vt_driver.ttd_ops = &vt_driver_ops;
                vt_terms[i].vt_driver.ttd_callback = NULL;
                vt_terms[i].vt_driver.ttd_callback_arg = NULL;
        }

        /* Initialize the current virtual terminal */
        vt_curterm = vt_terms;

        /* Wipe the screen to start (Who knows?) */
        vt_redraw();

        /* Register handler with keyboard */
        keyboard_register_handler(vt_keyboard_handler);
}

int
vt_num_terminals()
{
        return NTERMS;
}


tty_driver_t *
vt_get_tty_driver(int id)
{
        if (id >= NTERMS) {
                return NULL;
        } else {
                return &vt_terms[id].vt_driver;
        }
}

void
vt_scroll(int lines, int scroll_up)
{
        /* We move vt_top and redraw the entire buffer */
        if (scroll_up) {
                /* Check against the head of the buffer */
                if (circ_dist(vt_curterm->vt_top,
                              vt_curterm->vt_head) <
                    lines * DISPLAY_WIDTH) {
                        vt_curterm->vt_top = vt_curterm->vt_head;
                } else {
                        buf_add(vt_curterm->vt_top,
                                -lines * DISPLAY_WIDTH);
                }
        } else {
                /* Check against tail of buffer */
                /* Note we add one as tail points AFTER the last
                 * line */
                if (circ_dist(vt_curterm->vt_tail,
                              vt_curterm->vt_top) <
                    (lines + 1) * DISPLAY_WIDTH) {
                        vt_curterm->vt_top = vt_curterm->vt_tail;
                        buf_add(vt_curterm->vt_top, -DISPLAY_WIDTH);
                } else {
                        buf_add(vt_curterm->vt_top, lines * DISPLAY_WIDTH);
                }
        }
        vt_redraw();
}

int
vt_switch(int term)
{
        if (term < 0 || term >= NTERMS)
                return -1;
        vt_curterm = &(vt_terms[term]);
        vt_redraw();
        return 0;
}

void
vt_print_shutdown()
{
        tty_driver_t *ttyd;
        static const char str[] = "                   "
                                  "It is now safe to turn off your computer";
        int i;

        if (vt_curterm == NULL) return;
        ttyd = &vt_curterm->vt_driver;
        KASSERT(NULL != ttyd->ttd_ops);
        KASSERT(NULL != ttyd->ttd_ops->provide_char);

        for (i = 0; i < 20; i++)
                ttyd->ttd_ops->provide_char(ttyd, '\n');
        for (i = 0; str[i] != '\0'; i++)
                ttyd->ttd_ops->provide_char(ttyd, str[i]);
        for (i = 0; i < 14; i++)
                ttyd->ttd_ops->provide_char(ttyd, '\n');
}

void
vt_provide_char(tty_driver_t *ttyd, char c)
{
        KASSERT(NULL != ttyd);

        virtterm_t *vt = driver_to_vt(ttyd);

        /* Store for optimizing */
        int old_cursor = vt->vt_cursor;
        int old_top = vt->vt_top;
        int can_write_char;

        /* If cursor is not on the screen, we move top */
        if (circ_dist(vt->vt_cursor, vt->vt_top) >= DISPLAY_SIZE) {
                /* Cursor should be on the last row in this case */

                vt->vt_top = next_row(vt->vt_cursor);
                buf_add(vt->vt_top, -DISPLAY_SIZE);
        }

        can_write_char = vt_handle_char(vt, c);

        /* Redraw if it's the current terminal */
        if (vt_curterm == vt) {
                /*
                 * Check if we can optimize (just put 1 char instead
                 * of redrawing screen)
                 */
                if (old_top == vt->vt_top) {
                        if (can_write_char) {
                                int rel_cursor = circ_dist(old_cursor, vt->vt_top);
                                screen_putchar(c, rel_cursor %
                                               DISPLAY_WIDTH,
                                               rel_cursor / DISPLAY_WIDTH);
                        }
                        vt_cursor_redraw();
                } else {
                        vt_redraw();
                }
        }
}


tty_driver_callback_t
vt_register_callback_handler(tty_driver_t *ttyd, tty_driver_callback_t callback, void *arg)
{
        tty_driver_callback_t previous_callback;

        KASSERT(NULL != ttyd);
        previous_callback = ttyd->ttd_callback;
        ttyd->ttd_callback = callback;
        ttyd->ttd_callback_arg = arg;
        return previous_callback;
}

tty_driver_callback_t
vt_unregister_callback_handler(tty_driver_t *ttyd)
{
        tty_driver_callback_t previous_callback;

        KASSERT(NULL != ttyd);
        previous_callback = ttyd->ttd_callback;
        ttyd->ttd_callback = NULL;
        return previous_callback;
}

void *
vt_block_io(tty_driver_t *ttyd)
{
        uint8_t oldipl;
        KASSERT(NULL != ttyd);

        oldipl = intr_getipl();
        intr_setipl(INTR_KEYBOARD);
        return (void *)(uintptr_t)oldipl;
}

void
vt_unblock_io(tty_driver_t *ttyd, void *data)
{
        uint8_t oldipl = (uint8_t)(uintptr_t)data;
        KASSERT(NULL != ttyd);

        KASSERT(intr_getipl() == INTR_KEYBOARD &&
                "Virtual terminal I/O not blocked");
        intr_setipl(oldipl);
}

void
vt_keyboard_handler(char c)
{
        if (vt_curterm && vt_curterm->vt_driver.ttd_callback) {
                vt_curterm->vt_driver.ttd_callback(
                        vt_curterm->vt_driver.ttd_callback_arg, c);
        }
}

/* Redraws only the cursor */
void
vt_cursor_redraw()
{
        int rel_cursor = circ_dist(vt_curterm->vt_cursor, vt_curterm->vt_top);
        screen_move_cursor(rel_cursor % DISPLAY_WIDTH, rel_cursor / DISPLAY_WIDTH);
}


/* Redraws the screen based on the current virtual terminal */
void
vt_redraw()
{
        /* akerber: There is, quite tragically, a reason why this
         * doesn't ever use MIN and MAX - see the appropriate
         * header */

        /* Clear temporary buffer for use */
        memset(vt_curterm->vt_tempbuf, 0, DISPLAY_SIZE);
        /* Write in the entire buffer starting from top */
        if (vt_curterm->vt_top <= vt_curterm->vt_tail) { /* No wraparound */
                if (vt_curterm->vt_tail - vt_curterm->vt_top <=
                    DISPLAY_SIZE) {
                        memcpy(vt_curterm->vt_tempbuf,
                               vt_curterm->vt_buf + vt_curterm->vt_top,
                               vt_curterm->vt_tail - vt_curterm->vt_top);
                } else {
                        memcpy(vt_curterm->vt_tempbuf,
                               vt_curterm->vt_buf + vt_curterm->vt_top,
                               DISPLAY_SIZE);
                }
        } else { /* Wraparound */
                int first_part_size = SCROLL_BUFSIZE - vt_curterm->vt_top;
                /* First half */
                if (first_part_size >= DISPLAY_SIZE) {
                        memcpy(vt_curterm->vt_tempbuf,
                               vt_curterm->vt_buf + vt_curterm->vt_top,
                               DISPLAY_SIZE);
                } else {
                        memcpy(vt_curterm->vt_tempbuf,
                               vt_curterm->vt_buf + vt_curterm->vt_top,
                               first_part_size);
                        /* Second half (after wrapping) */
                        if (first_part_size + vt_curterm->vt_tail <=
                            DISPLAY_SIZE) {
                                memcpy(vt_curterm->vt_tempbuf +
                                       first_part_size,
                                       vt_curterm->vt_buf,
                                       vt_curterm->vt_tail);
                        } else {
                                memcpy(vt_curterm->vt_tempbuf +
                                       first_part_size,
                                       vt_curterm->vt_buf,
                                       DISPLAY_SIZE - first_part_size);
                        }
                }
        }
        screen_putbuf(vt_curterm->vt_tempbuf);

        /* Also want to reposition the cursor */
        vt_cursor_redraw();
}

/* Puts the given char into the given terminal's buffer, moving the
 * cursor accordingly for control chars Returns 1 if char can be
 * echoed (non-control char), 0 otherwise */
static int
vt_handle_char(virtterm_t *vt, char c)
{
        KASSERT(NULL != vt);

        /* Start where the cursor currently is located */
        int new_cursor = vt->vt_cursor;
        int ret = 0;
        switch (c) {
                case '\b': /* Move cursor back one space */
                        /* the user can't backspace past the beginning */
                        if (new_cursor != vt->vt_head)
                                buf_dec(new_cursor);
                        break;
                case '\r':
                        new_cursor = (new_cursor / DISPLAY_WIDTH) * DISPLAY_WIDTH;
                        break;

                        /* In the next two cases, the cursor advances, and we
                         * need to compare it with the end of the buffer to
                         * determine whether or not to advance the buffer
                         * tail */

                case '\n': /* To beginning of next line */
                        new_cursor = next_row(new_cursor);
                        goto handle_tail;
                default:
                        /* Actually put a char into the buffer */
                        vt->vt_buf[new_cursor] = c;
                        /* And increment */
                        buf_inc(new_cursor);
                        ret = 1;

handle_tail: /* Yuck */
                        if (circ_dist(new_cursor, vt->vt_cursor) >=
                            circ_dist(vt->vt_tail, vt->vt_cursor)) {
                                /* Current cursor pos past tail of the current
                                 * buffer, so advance tail */
                                int new_tail = next_row(new_cursor);
                                /* Check for head adjusting (if we write
                                 * enough that the scroll buffer fills up, we
                                 * sacrifice chars near the head) */
                                if (circ_dist(vt->vt_tail, vt->vt_head) >=
                                    circ_dist(vt->vt_tail, new_tail)) {
                                        vt->vt_head = next_row(new_tail);
                                }

                                /* Remember to clear space we may have acquired */
                                if (vt->vt_tail <= new_tail) {
                                        memset(vt->vt_buf + vt->vt_tail,
                                               0, new_tail - vt->vt_tail);
                                } else {
                                        memset(vt->vt_buf + vt->vt_tail,
                                               0, SCROLL_BUFSIZE - vt->vt_tail);
                                        memset(vt->vt_buf, 0, new_tail);
                                }
                                /* Finally, set the new tail */
                                vt->vt_tail = new_tail;
                        }
                        break;
        }
        vt->vt_cursor = new_cursor;
        return ret;
}
