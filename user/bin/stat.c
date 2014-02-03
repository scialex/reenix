#include <errno.h>
#include <fcntl.h>
#include <stdlib.h>
#include <stdio.h>
#include <string.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <unistd.h>

const char *modestr(int mode) {
  switch(mode) {
  case S_IFCHR:
    return "Character device";
  case S_IFBLK:
    return "Block device";
  case S_IFDIR:
    return "Directory";
  case S_IFREG:
    return "Regular file";
  case S_IFLNK:
    return "Symbolic link";
  default:
    return "Unknown";
  }
}

int main(int argc, char **argv) {
  if (argc != 2) {
    printf("usage: stat file\n");
    return 1;
  }

  struct stat ss;
  int rc = stat(argv[1], &ss);
  if (rc == -1) {
    printf("stat: %s\n", strerror(errno));
    return 1;
  }

  printf("      File: %s\n", argv[1]);
  printf("      Type: %s\n", modestr(ss.st_mode));
  printf("     Inode: %d\n", ss.st_ino);
  printf("Link count: %d\n", ss.st_nlink);
  printf("      Size: %d\n", ss.st_size);
  printf("    Blocks: %d\n", ss.st_blocks);
  return 0;
}
