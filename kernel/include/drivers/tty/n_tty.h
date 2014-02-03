#pragma once

#include "drivers/tty/ldisc.h"
#include "proc/kmutex.h"

/*
 * The default line discipline.
 */

typedef struct n_tty n_tty_t;

/**
 * Allocate and initialize an n_tty line discipline, which is not yet
 * attached to a tty.
 *
 * @return a newly allocated n_tty line discipline
 */
tty_ldisc_t *n_tty_create();

/**
 * Destory an n_tty line discipline.
 *
 * @param ntty the n_tty line discipline to destroy
 */
void n_tty_destroy(tty_ldisc_t *ntty);
