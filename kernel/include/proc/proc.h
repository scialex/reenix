#pragma once

#include "types.h"

#include "proc/kthread.h"

#include "mm/pagetable.h"

#include "vm/vmmap.h"

#include "config.h"

#define PROC_MAX_COUNT  65536
#define PROC_NAME_LEN   256

struct regs;

typedef struct proc {
        pid_t           p_pid;                 /* our pid */
        char            p_comm[PROC_NAME_LEN]; /* process name */

        list_t          p_threads;       /* the process's thread list */
        list_t          p_children;      /* the process's children list */
        struct proc    *p_pproc;         /* our parent process */

        int             p_status;        /* exit status */
        int             p_state;         /* running/sleeping/etc. */
        ktqueue_t       p_wait;          /* queue for wait(2) */

        pagedir_t      *p_pagedir;

        list_link_t     p_list_link;     /* link on the list of all processes */
        list_link_t     p_child_link;    /* link on proc list of children */

        /* VFS-related: */
        struct file    *p_files[NFILES]; /* open files */
        struct vnode   *p_cwd;           /* current working dir */

        /* VM */
        void           *p_brk;           /* process break; see brk(2) */
        void           *p_start_brk;     /* initial value of process break */
        struct vmmap   *p_vmmap;         /* list of areas mapped into
                                          * process' user address
                                          * space */
} proc_t;

/* Process states. */
#define PROC_RUNNING    1       /* has running threads */
#define PROC_DEAD       2       /* has already exited, hasn't been wait'ed */


/* Special PIDs for Kernel Deamons */
#define PID_IDLE     0
#define PID_INIT     1

void proc_init(void);

/**
 * This function allocates and initializes a new process.
 *
 * @param name the name to give the newly created process
 * @return the newly created process
 */
proc_t *proc_create(char *name);

/**
 * Finds the process with the specified PID.
 *
 * @param pid the PID of the process to find
 * @return a pointer to the process with PID pid, or NULL if there is
 * no such process
 */
proc_t *proc_lookup(int pid);

/**
 * Returns the list of running processes.
 *
 * @return the list of running processes
 */
list_t *proc_list(void);

/**
 * Stops another process from running again by cancelling all its
 * threads.
 *
 * @param p the process to kill
 * @param status the status the process should exit with
 */
void proc_kill(proc_t *p, int status);

/**
 * Kill every process except for the idle process.
 */
void proc_kill_all(void);

/**
 * Alerts the process that the currently executing thread has just
 * exited.
 *
 * @param retval the return value for the current thread
 */
void proc_thread_exited(void *retval);

/**
 * This function implements the _exit(2) system call.
 *
 * @param status the exit status of the process
 */
void do_exit(int status);

/**
 * This function implements the waitpid(2) system call.
 *
 * @param pid see waitpid man page, only -1 or positive numbers are supported
 * @param options see waitpid man page, only 0 is supported
 * @param status used to return the exit status of the child
 *
 * @return the pid of the child process which was cleaned up, or
 * -ECHILD if there are no children of this process
 */
pid_t do_waitpid(pid_t pid, int options, int *status);

/**
 * This function implements the fork(2) system call.
 *
 * @param regs the register state at the time of the system call
 */
int do_fork(struct regs *regs);

/**
 * Provides detailed debug information about a given process.
 *
 * @param arg a pointer to the process
 * @param buf buffer to write to
 * @param osize size of the buffer
 * @return the remaining size of the buffer
 */
size_t proc_info(const void *arg, char *buf, size_t osize);

/**
 * Provides debug information overview of all processes.
 *
 * @param arg must be NULL
 * @param buf buffer to write to
 * @param osize size of the buffer
 * @return the remaining size of the buffer
 */
size_t proc_list_info(const void *arg, char *buf, size_t osize);
