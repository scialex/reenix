#include "stdio.h"

int debug(const char *str);

#define dbg(fmt, args...) \
        do { \
                char temp[2048]; \
                snprintf(temp, 2048, "%s:%d %s(): " fmt, __FILE__, __LINE__, __func__, ## args); \
                debug(temp); \
        } while(0);

