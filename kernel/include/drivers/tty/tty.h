#pragma once

#include "drivers/bytedev.h"

#define TTY_MAJOR 2

struct tty_driver;
struct tty_ldisc;

typedef struct tty_device {
        struct tty_driver *tty_driver;
        struct tty_ldisc  *tty_ldisc;
        int                tty_id;
        bytedev_t          tty_cdev;
} tty_device_t;

/**
 * Initialize the tty subsystem.
 */
void tty_init(void);

/**
 * Creates a tty with the given driver and id.
 *
 * @param driver the tty driver to use
 * @param the id of the tty
 * @return a newly allocated tty or NULL on error
 */
tty_device_t *tty_create(struct tty_driver *driver, int id);
