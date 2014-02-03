#include <errno.h>
#include <fcntl.h>
#include <stdlib.h>
#include <stdio.h>
#include <string.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <unistd.h>

#define LINE_LEN 16

int main(int argc, char **argv) {
  int readfd = 0;
  if (argc == 2) {
    readfd = open(argv[1], O_RDONLY, 0666);
    if (readfd < 0) {
      fprintf(stderr, "open: %s\n", strerror(errno));
      return 1;
    }
  } else if (argc > 2) {
    printf("usage: hd [file]\n");
    return 1;
  }

  char lastbuf[LINE_LEN];
  char curbuf[LINE_LEN];
  int off = 0;
  int lastrep = 0;
  int bytes;

  int i;
  while ((bytes = read(readfd, curbuf, LINE_LEN)) > 0) {
    if (off > 0 && !memcmp(lastbuf, curbuf, LINE_LEN)) {
      if (!lastrep) {
        printf("*\n");
        lastrep = 1;
      }
      off += bytes;
      continue;
    }
    lastrep = 0;
    printf("%08x  ", off);
    off += bytes;
    /* print bytes */
    for (i = 0; i < LINE_LEN; ++i) {
      if (i < bytes) {
        printf("%02x ", (unsigned char)curbuf[i]);
      } else {
        printf("   ");
      }
      if (i == 7) {
        printf(" ");
      }
    }
    /* show printable characters */
    printf("|");
    for (i = 0; i < bytes; ++i) {
      char c = curbuf[i];
      if (c < 32 || c > 126) {
        printf(".");
      } else {
        printf("%c",c);
      }
    }
    printf("|\n");
    memcpy(lastbuf, curbuf, LINE_LEN);
  }
  printf("%08x\n", off);

  if (readfd > 0) {
    close(readfd);
  }
  return 0;
}
