#pragma once

#include "types.h"

struct regs;

int do_execve(const char *filename, char *const *argv, char *const *envp, struct regs *regs);

void kernel_execve(const char *filename, char *const *argv, char *const *envp);

void userland_entry(const struct regs *regs);
