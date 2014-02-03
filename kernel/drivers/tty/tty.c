#include "drivers/tty/tty.h"

#include "drivers/bytedev.h"

#include "drivers/tty/driver.h"
#include "drivers/tty/keyboard.h"
#include "drivers/tty/ldisc.h"
#include "drivers/tty/n_tty.h"
#include "drivers/tty/screen.h"
#include "drivers/tty/virtterm.h"

#include "mm/kmalloc.h"

#include "util/debug.h"

#define bd_to_tty(bd) \
        CONTAINER_OF(bd, tty_device_t, tty_cdev)

/**
 * The callback function called by the virtual terminal subsystem when
 * a key is pressed.
 *
 * @param arg a pointer to the tty attached to the virtual terminal
 * which received the key press=
 * @param c the character received
 */
static void tty_global_driver_callback(void *arg, char c);

/**
 * Reads a specified maximum number of bytes from a tty into a given
 * buffer.
 *
 * @param dev the tty to read from
 * @param offset this parameter is ignored in this function
 * @param buf the buffer to read into
 * @param count the maximum number of bytes to read
 * @return the number of bytes read into buf
 */
static int tty_read(bytedev_t *dev, int offset, void *buf, int count);

/**
 * Writes a specified maximum number of bytes from a given buffer into
 * a tty.
 *
 * @param dev the tty to write to
 * @param offset this parameter is ignored in this function
 * @param buf the buffer to read from
 * @param count the maximum number of bytes to write
 * @return the number of bytes written
 */
static int tty_write(bytedev_t *dev, int offset, const void *buf, int count);

/**
 * Echoes out to the tty driver.
 *
 * @param driver the tty driver to echo to
 * @param out the string to echo
 */
static void tty_echo(tty_driver_t *driver, const char *out);

static bytedev_ops_t tty_bytedev_ops = {
        tty_read,
        tty_write,
        NULL,
        NULL,
        NULL,
        NULL
};

void
tty_init()
{
        screen_init();
        vt_init();
        keyboard_init();

        /*
         * Create NTERMS tty's, all with the default line discipline
         * and a virtual terminal driver.
         */
        int nterms, i;

        nterms = vt_num_terminals();
        for (i = 0; i < nterms; ++i) {
                tty_driver_t *ttyd;
                tty_device_t *tty;
                tty_ldisc_t *ldisc;

                ttyd = vt_get_tty_driver(i);
                KASSERT(NULL != ttyd);
                KASSERT(NULL != ttyd->ttd_ops);
                KASSERT(NULL != ttyd->ttd_ops->register_callback_handler);

                tty = tty_create(ttyd, i);
                if (NULL == tty) {
                        panic("Not enough memory to allocate tty\n");
                }

                if (NULL != ttyd->ttd_ops->register_callback_handler(
                            ttyd, tty_global_driver_callback, (void *)tty)) {
                        panic("Callback already registered "
                              "to terminal %d\n", i);
                }

                ldisc = n_tty_create();
                if (NULL == ldisc) {
                        panic("Not enough memory to allocate "
                              "line discipline\n");
                }
                KASSERT(NULL != ldisc);
                KASSERT(NULL != ldisc->ld_ops);
                KASSERT(NULL != ldisc->ld_ops->attach);
                ldisc->ld_ops->attach(ldisc, tty);

                if (bytedev_register(&tty->tty_cdev) != 0) {
                        panic("Error registering tty as byte device\n");
                }
        }
}

/*
 * Initialize a tty device. The driver should not be NULL, but we don't
 * need a line descriptor yet. To initialize the byte device in the tty
 * struct, use the MKDEVID macro and the correct byte device function
 * table for a tty.
 */
tty_device_t *
tty_create(tty_driver_t *driver, int id)
{
        NOT_YET_IMPLEMENTED("DRIVERS: tty_create");
        return 0;
}

/*
 * This is the function called by the virtual terminal subsystem when
 * a key is pressed.
 *
 * The first thing you should do is to pass the character to the line
 * discipline. This way, the character will be buffered, so that when
 * the read system call is made, an entire line will be returned,
 * rather than just a single character.
 *
 * After passing the character off to the line discipline, the result
 * of receive_char() should be echoed to the screen using the
 * tty_echo() function.
 */
void
tty_global_driver_callback(void *arg, char c)
{
        NOT_YET_IMPLEMENTED("DRIVERS: tty_global_driver_callback");
}

/*
 * You should use the driver's provide_char function to output
 * each character of the string 'out'.
 */
void
tty_echo(tty_driver_t *driver, const char *out)
{
        NOT_YET_IMPLEMENTED("DRIVERS: tty_echo");
}

/*
 * In this function, you should block I/O, call the line discipline,
 * and unblock I/O. We do not want to receive interrupts while
 * modifying the input buffer.
 */
int
tty_read(bytedev_t *dev, int offset, void *buf, int count)
{
        NOT_YET_IMPLEMENTED("DRIVERS: tty_read");

        return 0;
}

/*
 * In this function, you should block I/O, process each
 * character with the line discipline and output the result to
 * the driver, and then unblock I/O.
 *
 * Important: You should return the number of bytes processed,
 * _NOT_ the number of bytes written out to the driver.
 */
int
tty_write(bytedev_t *dev, int offset, const void *buf, int count)
{
        NOT_YET_IMPLEMENTED("DRIVERS: tty_write");

        return 0;
}
