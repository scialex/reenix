#include "command.h"

#include "mm/kmalloc.h"

#include "util/debug.h"
#include "util/list.h"
#include "util/string.h"

kshell_command_t *kshell_command_create(const char *name,
                                        kshell_cmd_func_t cmd_func,
                                        const char *desc)
{
        kshell_command_t *cmd;
        size_t len;

        KASSERT(NULL != name);
        KASSERT(NULL != cmd_func);

        cmd = (kshell_command_t *)kmalloc(sizeof(kshell_command_t));
        if (NULL == cmd) {
                return NULL;
        }

        len = strnlen(name, KSH_CMD_NAME_LEN);
        strncpy(cmd->kc_name, name, len);
        cmd->kc_name[len] = '\0';

        cmd->kc_cmd_func = cmd_func;

        if (NULL != desc) {
                len = strnlen(desc, KSH_DESC_LEN);
                strncpy(cmd->kc_desc, desc, len);
                cmd->kc_desc[len] = '\0';
        } else {
                cmd->kc_desc[0] = '\0';
        }

        list_link_init(&cmd->kc_commands_link);

        return cmd;
}

void kshell_command_destroy(kshell_command_t *cmd)
{
        kfree(cmd);
}
