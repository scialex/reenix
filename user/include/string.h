#pragma once

#include "stddef.h"
#include "errno.h"

/* ANSI C89 */
void    *memchr(const void *, int, size_t); /* NYI */
int      memcmp(const void *cs, const void *ct, size_t count);
void    *memcpy(void *dest, const void *src, size_t count);
void    *memmove(void *dest, const void *src, size_t count);
void    *memset(void *s, int c, size_t count);

char    *strcpy(char *dest, const char *src);
char    *strncpy(char *dest, const char *src, size_t count);

char    *strcat(char *dest, const char *src);
char    *strncat(char *, const char *, size_t); /* NYI */

int      strcmp(const char *cs, const char *ct);
int      strncmp(const char *cs, const char *ct, size_t count);

char    *strchr(const char *s, int c);
char    *strrchr(const char *s, int c);

size_t   strspn(const char *s, const char *accept);
size_t   strcspn(const char *, const char *); /* NYI */

char    *strpbrk(const char *string, const char *brkset);

char    *strstr(const char *s1, const char *s2);

size_t   strlen(const char *s);

char    *strerror(int errnum);

char    *strtok(char *s, const char *sepset);

/* Other */
size_t strnlen(const char *s, size_t count);
