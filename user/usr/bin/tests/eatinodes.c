#include <errno.h>
#include <string.h>
#include <stdlib.h>

#include <dirent.h>
#include <unistd.h>
#include <sys/stat.h>
#include <weenix/syscall.h>
#include <fcntl.h>
#include <sys/mman.h>
#include <stdio.h>

static char root_dir[64];

static void eatinodes_start(void) {
  int err;

  root_dir[0] = '\0';
  do {
    sprintf(root_dir, "eatinodes-%d", rand());
    err = mkdir(root_dir, 0777);
  } while (err != 0);
  printf("Created test root directory: ./%s\n", root_dir);

  err = chdir(root_dir);
  if (err < 0) {
    printf("Could not cd into test directory\n");
    exit(1);
  }
}

static void eatinodes(void) {
  int i;
  int fd;
  int err = 0;
  char fname[24];

  for (i = 0; !err; ++i) {
    sprintf(fname, "test-%d", i);
    fd = open(fname, O_CREAT|O_TRUNC|O_WRONLY, 0666);
    if (fd < 0) {
      printf("Could not open file %d: %s\n", i, strerror(errno));
      break;
    }
    err = close(fd);
    if (err < 0) {
      printf("Could not close fd %d: %s\n", fd, strerror(errno));
      break;
    }
    printf("Created %d files\n",i);
  }
  int j;
  printf("Cleaning up...\n");
  for (j = 0; j < i; ++j) {
    sprintf(fname, "test-%d", j);
    err = unlink(fname);
    if (err < 0) {
      printf("Could not remove file %d: %s\n", j, strerror(errno));
    }
  }
}

static void eatinodes_end(void) {
  chdir("..");
  int err = rmdir(root_dir);
  if (err < 0) {
    printf("Could not remove test directory: %s\n", strerror(errno));
  }
}

int main(int argc, char **argv) {
  
  eatinodes_start();
  eatinodes();
  eatinodes_end();

  return 0;

}
