#pragma once

#include "priv.h"

#include "test/kshell/kshell.h"

typedef struct kshell_command {
        char              kc_name[KSH_CMD_NAME_LEN + 1];
        kshell_cmd_func_t kc_cmd_func;
        char              kc_desc[KSH_DESC_LEN + 1];

        list_link_t       kc_commands_link;
} kshell_command_t;

kshell_command_t *kshell_command_create(const char *name,
                                        kshell_cmd_func_t cmd_func,
                                        const char *desc);

void kshell_command_destroy(kshell_command_t *cmd);
