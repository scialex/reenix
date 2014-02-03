#pragma once

#include "types.h"

struct proc;
struct argstr;
struct argvec;

int copy_from_user(void *kaddr, const void *uaddr, size_t nbytes);
int copy_to_user(void *uaddr, const void *kaddr, size_t nbytes);

char *user_strdup(struct argstr *ustr);
char **user_vecdup(struct argvec *uvec);

int range_perm(struct proc *p, const void *vaddr, size_t len, int perm);
int addr_perm(struct proc *p, const void *vaddr, int perm);
