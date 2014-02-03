#include "types.h"

struct proc;
struct vmarea;

int do_munmap(void *addr, size_t len);
int do_mmap(void *addr, size_t len, int prot, int flags, int fd, off_t off, void **ret);
