#include "test/kshell/kshell.h"

#include "config.h"

#include "command.h"
#include "commands.h"
#include "priv.h"
#include "tokenizer.h"

#ifndef __VFS__
#include "drivers/bytedev.h"
#include "drivers/tty/tty.h"
#endif

#include "mm/kmalloc.h"

#ifdef __VFS__
#include "fs/fcntl.h"
#include "fs/open.h"
#include "fs/vfs_syscall.h"
#endif

#include "test/kshell/io.h"

#include "util/init.h"
#include "util/debug.h"
#include "util/printf.h"
#include "util/string.h"

static __attribute__((unused)) void kshell_init()
{
        list_init(&kshell_commands_list);

        kshell_add_command("help", kshell_help,
                           "prints a list of available commands");
        kshell_add_command("echo", kshell_echo, "display a line of text");
#ifdef __VFS__
        kshell_add_command("cat", kshell_cat,
                           "concatenate files and print on the standard output");
        kshell_add_command("ls", kshell_ls, "list directory contents");
        kshell_add_command("cd", kshell_cd, "change the working directory");
        kshell_add_command("rm", kshell_rm, "remove files");
        kshell_add_command("link", kshell_link,
                           "call the link function to create a link to a file");
        kshell_add_command("rmdir", kshell_rmdir,
                           "remove empty directories");
        kshell_add_command("mkdir", kshell_mkdir, "make directories");
        kshell_add_command("stat", kshell_stat, "display file status");
#endif

        kshell_add_command("exit", kshell_exit, "exits the shell");
}
init_func(kshell_init);

void kshell_add_command(const char *name, kshell_cmd_func_t cmd_func,
                        const char *desc)
{
        kshell_command_t *cmd;

        cmd = kshell_command_create(name, cmd_func, desc);
        KASSERT(NULL != cmd);
        list_insert_tail(&kshell_commands_list, &cmd->kc_commands_link);

        dprintf("Added %s command\n", name);
}

kshell_t *kshell_create(uint8_t ttyid)
{
        kshell_t *ksh;

        ksh = (kshell_t *)kmalloc(sizeof(kshell_t));
        if (NULL == ksh) {
                dprintf("Not enough memory to create kshell\n");
                return NULL;
        }

#ifdef __VFS__
        int fd;
        char tty_path[MAXPATHLEN];

        snprintf(tty_path, MAXPATHLEN, "/dev/tty%u", ttyid);
        if ((fd = do_open(tty_path, O_RDWR)) < 0) {
                dprintf("Couldn't open %s\n", tty_path);
                kfree(ksh);
                return NULL;
        }
        ksh->ksh_out_fd = ksh->ksh_in_fd = ksh->ksh_fd = fd;
#else
        bytedev_t *bd;
        bd = bytedev_lookup(MKDEVID(TTY_MAJOR, ttyid));
        if (NULL == bd) {
                dprintf("Couldn't find TTY with ID %u\n", ttyid);
                kfree(ksh);
                return NULL;
        }
        ksh->ksh_bd = bd;
#endif

        dprintf("kshell successfully created on TTY %u\n", ttyid);
        return ksh;
}

void kshell_destroy(kshell_t *ksh)
{
        KASSERT(NULL != ksh);
        kprintf(ksh, "Bye!\n");
#ifdef __VFS__
        if (do_close(ksh->ksh_fd) < 0) {
                panic("Error closing TTY file descriptor\n");
        }
        dprintf("kshell with file descriptor %d destroyed\n", ksh->ksh_fd);
#else
        dprintf("kshell on byte device %u destroyed\n", ksh->ksh_bd->cd_id);
#endif
        kfree(ksh);
}

/**
 * Removes the token from the input line it came from, replacing it
 * with spaces.
 *
 * @param ksh the kshell
 * @param token the token to scrub
 */
static void kshell_scrub_token(kshell_t *ksh, kshell_token_t *token)
{
        KASSERT(NULL != ksh);
        KASSERT(NULL != token);
        KASSERT(NULL != token->kt_text);

        memset(token->kt_text, ' ', token->kt_textlen);
}

/**
 * Finds the redirection operators ('<' and '>') in the input line,
 * stores the name of the file to redirect stdout in in redirect_out
 * and the name of the file to redirect stdin in redirect_in, and
 * removes any trace of the redirection from the input line.
 *
 * @param ksh the kshell
 * @param line the input line
 * @param redirect_in buffer to store the name of the file to redirect
 * stdin from. Buffer size assumed to be at least MAXPATHLEN
 * @param redirect_out buffer to store the name of the file to stdout
 * to. Buffer size assumed to be at least MAXPATHLEN
 * @param append out parameter containing true if the file stdout is
 * being redirected to should be appeneded to
 * @return 0 on success and <0 on error
 */
static int kshell_find_redirection(kshell_t *ksh, char *line,
                                   char *redirect_in,
                                   char *redirect_out,
                                   int *append)
{
        int retval;
        kshell_token_t token;

        while ((retval = kshell_next_token(ksh, line, &token)) > 0) {
                KASSERT(token.kt_type != KTT_EOL);
                line += retval;

                if (token.kt_type == KTT_WORD) continue;

                char *redirect;
                if (token.kt_type == KTT_REDIRECT_OUT) {
                        redirect = redirect_out;
                        *append = 0;
                } else if (token.kt_type == KTT_REDIRECT_OUT_APPEND) {
                        redirect = redirect_out;
                        *append = 1;
                } else if (token.kt_type == KTT_REDIRECT_IN) {
                        redirect = redirect_in;
                }
                kshell_scrub_token(ksh, &token);

                if ((retval = kshell_next_token(ksh, line, &token)) == 0) {
                        goto unexpected_token;
                }
                KASSERT(retval > 0);

                if (token.kt_type != KTT_WORD) goto unexpected_token;
                strncpy(redirect, token.kt_text, token.kt_textlen);
                redirect[token.kt_textlen] = '\0';
                kshell_scrub_token(ksh, &token);
        }
        return 0;

unexpected_token:
        kprintf(ksh, "kshell: Unexpected token '%s'\n",
                kshell_token_type_str(token.kt_type));
        return -1;
}

/**
 * Ignoring whitespace, finds the next argument from a string.
 *
 * @param ksh the kshell
 * @param line the string to find arguments in
 * @param arg out parameter which should point to the beginning of the
 * next argument if any were found
 * @param arglen the length of the argument if any were found
 * @return 0 if no argument was found, and the number of bytes read
 * otherwise
 */
static int kshell_find_next_arg(kshell_t *ksh, char *line,
                                char **arg, size_t *arglen)

{
        int retval;
        kshell_token_t token;

        if ((retval = kshell_next_token(ksh, line, &token)) == 0) {
                KASSERT(token.kt_type == KTT_EOL);
                return retval;
        }
        KASSERT(token.kt_type == KTT_WORD);
        *arg = token.kt_text;
        *arglen = token.kt_textlen;

        /*
         * This is a little hacky, but not awful.
         *
         * If we find a '\0', there are no more arguments
         * left. However, we still need to return a nonzero value to
         * alert the calling function about the argument we just
         * found. Since there are no more arguments, we aren't
         * overwriting anything by setting the next byte to '\0'. We
         * also know that we aren't writing into invalid memory
         * because in the struct definition for kshell_t, we declared
         * ksh_buf to have KSH_BUF_SIZE + 1 bytes.
         */
        if (line[retval] == '\0') {
                line[retval + 1] = '\0';
        } else {
                line[retval] = '\0';
        }
        return retval;
}

/**
 * Finds the arguments of the command just into a kshell. This should
 * be called directly after returning from a read.
 *
 * @param buf the buffer to extract arguments from
 * @param argv out parameter containing an array of null-terminated
 * strings, one for each argument
 * @param max_args the maximum number of arguments to find
 * @param argc out parameter containing the number of arguments found
 */
static void kshell_get_args(kshell_t *ksh, char *buf,
                            char **argv, int max_args,
                            int *argc)
{
        size_t arglen;

        KASSERT(NULL != buf);
        KASSERT(NULL != argv);
        KASSERT(max_args > 0);
        KASSERT(NULL != argc);

        *argc = 0;
        while (kshell_find_next_arg(ksh, buf, argv + *argc, &arglen) &&
               *argc < max_args) {
                buf = argv[*argc] + arglen + 1;
                ++(*argc);
        }
        if (*argc >= max_args) {
                dprintf("Too many arguments\n");
        }
}

kshell_command_t *kshell_lookup_command(const char *name, size_t namelen)
{
        kshell_command_t *cmd;
        if (namelen > KSH_CMD_NAME_LEN) {
                namelen = KSH_CMD_NAME_LEN;
        }

        list_iterate_begin(&kshell_commands_list, cmd, kshell_command_t,
                           kc_commands_link) {
                KASSERT(NULL != cmd);
                if ((strncmp(cmd->kc_name, name, namelen) == 0) &&
                    (namelen == strnlen(cmd->kc_name, namelen))) {
                        return cmd;
                }
        } list_iterate_end();

        return NULL;
}

#ifdef __VFS__
/**
 * If stdin or stdout has been redirected to a file, closes the file
 * and directs I/O back to stdin and stdout.
 *
 * @param the kshell
 */
void kshell_undirect(kshell_t *ksh)
{
        KASSERT(NULL != ksh);

        if (ksh->ksh_in_fd != ksh->ksh_fd) {
                if (do_close(ksh->ksh_in_fd) < 0) {
                        panic("kshell: Error closing file descriptor %d\n",
                              ksh->ksh_in_fd);
                }
                ksh->ksh_in_fd = ksh->ksh_fd;
        }
        if (ksh->ksh_out_fd != ksh->ksh_fd) {
                if (do_close(ksh->ksh_out_fd) < 0) {
                        panic("kshell: Error closing file descriptor %d\n",
                              ksh->ksh_out_fd);
                }
                ksh->ksh_out_fd = ksh->ksh_fd;
        }
}

/**
 * Redirects stdin and stdout.
 *
 * @param ksh the kshell
 * @param redirect_in the name of the file to redirect stdin from
 * @param redirect_out the name of the file to redirect stdout to
 * @param append if true, output will be appended
 * @return 0 on sucess and <0 on error. If returns with <0, no streams
 * will be redirected.
 */
int kshell_redirect(kshell_t *ksh, const char *redirect_in,
                    const char *redirect_out, int append)
{
        int fd;

        KASSERT(NULL != ksh);
        KASSERT(NULL != redirect_in);
        KASSERT(NULL != redirect_out);

        if (redirect_in[0] != '\0') {
                if ((fd = do_open(redirect_in, O_RDONLY | O_CREAT)) < 0) {
                        kprintf(ksh, "kshell: %s: Error opening file\n", redirect_in);
                        goto error;
                }
                ksh->ksh_in_fd = fd;
        }
        if (redirect_out[0] != '\0') {
                int flags = append ? O_WRONLY | O_CREAT | O_APPEND :
                            O_WRONLY | O_CREAT;
                if ((fd = do_open(redirect_out, flags)) < 0) {
                        kprintf(ksh, "kshell: %s: Error opening file\n", redirect_out);
                        goto error;
                }
                ksh->ksh_out_fd = fd;
        }
        return 0;

error:
        kshell_undirect(ksh);
        return fd;
}
#endif


int kshell_execute_next(kshell_t *ksh)
{
        static const char *kshell_prompt = "kshell$";

        int nbytes, retval;
        kshell_command_t *cmd;
        char *args[KSH_MAX_ARGS];
        int argc;
        char redirect_in[MAXPATHLEN];
        char redirect_out[MAXPATHLEN];
        int append;

        /*
         * Need that extra byte at the end. See comment in
         * kshell_find_next_arg.
         */
        char buf[KSH_BUF_SIZE + 1];

        KASSERT(NULL != ksh);

        kprintf(ksh, "%s ", kshell_prompt);

        if ((nbytes = kshell_read(ksh, buf, KSH_BUF_SIZE)) <= 0) {
                return nbytes;
        }
        if (nbytes == 1) return 1;
        if (buf[nbytes - 1] == '\n') {
                /* Overwrite the newline with a null terminator */
                buf[--nbytes] = '\0';
        } else {
                /* Add the null terminator to the end */
                buf[nbytes] = '\0';
        }

        /* Even though we can't redirect I/O to files before VFS, we
         * still want to scrub out any reference to redirection before
         * passing the line off to kshell_get_args */
        redirect_in[0] = redirect_out[0] = '\0';
        if (kshell_find_redirection(ksh, buf, redirect_in, redirect_out, &append) < 0)
                goto done;
#ifdef __VFS__
        if ((retval = kshell_redirect(ksh, redirect_in, redirect_out, append)) < 0) {
                dprintf("Error redirecting I/O\n");
                goto done;
        }
#endif

        kshell_get_args(ksh, buf, args, KSH_MAX_ARGS, &argc);
        if (argc == 0) goto done;

        dprintf("Attempting to execute command '%s'\n", args[0]);

        if (strncmp(args[0], "exit", strlen(args[0])) == 0) {
                nbytes = 0;
                goto done;
        }

        if ((cmd = kshell_lookup_command(args[0], strlen(args[0]))) == NULL) {
                kprintf(ksh, "kshell: %s not a valid command\n", args[0]);
        } else {
                if ((retval = cmd->kc_cmd_func(ksh, argc, args)) < 0) {
                        nbytes = retval;
                        goto done;
                }
        }
        goto done;

done:
#ifdef __VFS__
        kshell_undirect(ksh);
#endif
        return nbytes;
}
