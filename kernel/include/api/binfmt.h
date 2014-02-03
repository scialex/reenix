#pragma once

#include "fs/vnode.h"

typedef int(*binfmt_load_func_t)(const char *filename, int fd,
                                 char *const *argv, char *const *envp, uint32_t *eip, uint32_t *esp);

int  binfmt_add(const char *id, binfmt_load_func_t loadfunc);

int binfmt_load(const char *filename, char *const *argv, char *const *envp, uint32_t *eip, uint32_t *esp);
