#pragma once

#include "util/list.h"

struct kthread;
typedef struct ktqueue {
        list_t          tq_list;
        int             tq_size;
} ktqueue_t;

/**
 * Switches execution between kernel threads.
 */
void sched_switch(void);

/**
 * Marks the given thread as runnable, and adds it to the run queue.
 *
 * @param thr the thread to make runnable
 */
void sched_make_runnable(struct kthread *kt);

/**
 * Initializes a queue.
 *
 * @param q the queue
 */
void sched_queue_init(ktqueue_t *q);

/**
 * Returns true if the queue is empty.
 *
 * @param q the queue
 * @return true if the queue is empty
 */
int sched_queue_empty(ktqueue_t *q);

/**
 * Causes the current thread to enter into an uncancellable sleep on
 * the given queue.
 *
 * @param q the queue to sleep on
 */
void sched_sleep_on(ktqueue_t *q);

/**
 * Causes the current thread to enter into a cancellable sleep on the
 * given queue.
 *
 * @param q the queue to sleep on
 * @return -EINTR if the thread was cancelled and 0 otherwise
 */
int sched_cancellable_sleep_on(ktqueue_t *q);

/**
 * Wakes a single thread from sleep if there are any waiting on the
 * queue.
 *
 * @param q the q to wakeup a thread from
 * @return NULL if q is empty and a thread waiting on the q otherwise
 */
struct kthread *sched_wakeup_on(ktqueue_t *q);

/**
 * Wake up all threads running on the queue.
 *
 * @param q the queue to wake up threads from
 */
void sched_broadcast_on(ktqueue_t *q);

/**
 * Cancel the given thread from the queue it sleeps on.
 *
 * @param the thread to cancel sleep from
 */
void sched_cancel(struct kthread *kthr);
