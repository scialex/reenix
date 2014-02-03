#include "stdio.h"

static int stdstreams[3] = { 0, 1, 2 };

FILE *stdin = &stdstreams[0];
FILE *stdout = &stdstreams[1];
FILE *stderr = &stdstreams[2];
