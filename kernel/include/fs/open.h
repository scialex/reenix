/*
 *  FILE: open.h
 *  AUTH: mcc
 *  DESC:
 *  DATE: Tue Apr  7 18:52:52 1998
 */

#pragma once

struct open_args;
struct proc;

int do_open(const char *filename, int flags);
int get_empty_fd(struct proc *p);
