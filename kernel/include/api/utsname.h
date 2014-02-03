#pragma once

#define _UTSNAME_LENGTH 128

struct utsname {
        char sysname[_UTSNAME_LENGTH];
        char nodename[_UTSNAME_LENGTH];
        char release[_UTSNAME_LENGTH];
        char version[_UTSNAME_LENGTH];
        char machine[_UTSNAME_LENGTH];
};

int uname(struct utsname *buf);
