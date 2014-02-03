#pragma once

struct tty_ldisc;
struct tty_device;

typedef struct tty_ldisc_ops {
        /**
         * Attaches a line discipline to the given tty and allocates
         * any memory needed by the line discipline.
         *
         * @param ldisc the line discipline to attach
         * @param tty the tty to attach the line discipline to
         */
        void (*attach)(struct tty_ldisc *ldisc, struct tty_device *tty);

        /**
         * Detaches the line discipline from the tty it is currently
         * attached to and frees any memory allocated from attach.
         *
         * @param ldisc the line discipline
         * @param tty the tty to detach from
         */
        void (*detach)(struct tty_ldisc *ldisc, struct tty_device *tty);

        /**
         * Read bytes from the line discipline into the buffer.
         *
         * @param ldisc the line discipline
         * @param buf the buffer to read into
         * @param len the maximum number of bytes to read
         * @return the number of bytes read
         */
        int (*read)(struct tty_ldisc *ldisc, void *buf, int len);

        /**
         * Receive a character and return a string to be echoed to the
         * tty.
         *
         * @param ldisc the line discipline to receive the character
         * @param c the character received
         * @return a null terminated string to be echoed to the tty
         */
        const char *(*receive_char)(struct tty_ldisc *ldisc, char c);

        /**
         * Process a character and return a string to be echoed to the
         * tty.
         *
         * @param ldisc the line discipline
         * @param c the character to process
         * @return a null terminated string to be echoed to the tty
         */
        const char *(*process_char)(struct tty_ldisc *ldisc, char c);
} tty_ldisc_ops_t;

typedef struct tty_ldisc {
        tty_ldisc_ops_t   *ld_ops;
} tty_ldisc_t;
