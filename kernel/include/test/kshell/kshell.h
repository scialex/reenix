#pragma once

#include "types.h"

typedef struct kshell kshell_t;
typedef int (*kshell_cmd_func_t)(kshell_t *, int argc, char **argv);

/**
 * Adds a command to the global command table for kernel shells.
 *
 * Note: When writing commands for the kernel shell, you _MUST_ use
 * the I/O functions from kshell_io.h instead of normal I/O
 * functions. See comment in kshell_io.h for more information.
 *
 * @param name the name of the command. Typing this name into the
 * shell will execute the command.
 * @param command the command to add to the shell
 * @param desc a description of the command. This is what will be
 * printed by the command 'help <command>'
 */
void kshell_add_command(const char *name, kshell_cmd_func_t command,
                        const char *desc);

/**
 * Allocates and initializes a kshell.
 *
 * @param bd the byte device the kshell will read from and write to
 * @return a kshell
 */
kshell_t *kshell_create(uint8_t ttyid);

/**
 * Destroys a kshell.
 *
 * @param ksh the kshell to destroy
 */
void kshell_destroy(kshell_t *ksh);

/**
 * Reads from the kshell's byte device and attempts to execute a
 * command.
 *
 * @param ksh the kshell to execute commands with
 * @return the number of bytes read
 */
int kshell_execute_next(kshell_t *ksh);
