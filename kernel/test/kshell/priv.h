#pragma once

#include "test/kshell/kshell.h"

#include "util/list.h"

#define dprintf(x, args...) dbg(DBG_TEST, x, ## args)

#define KSH_BUF_SIZE 1024 /* This really just needs to be as large as
* the line discipline buffer */
#define KSH_CMD_NAME_LEN 16
#define KSH_MAX_ARGS 128
#define KSH_DESC_LEN 64

struct bytedev;
struct kshell_command;

struct kshell {
        /* If we have a filesystem, we can write to the file
         * descriptor. Otherwise, we need to write to a byte device */
#ifdef __VFS__
        int ksh_fd;

        /* Used for redirection */
        int ksh_out_fd;
        int ksh_in_fd;
#else
        struct bytedev *ksh_bd;
#endif
};

list_t kshell_commands_list;

/**
 * Searches for a shell command with a specified name.
 *
 * @param name name of the command to search for
 * @param namelen length of name
 * @return the command, if it exists, or NULL
 */
struct kshell_command *kshell_lookup_command(const char *name, size_t namelen);
