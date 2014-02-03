#pragma once

#include "sys/types.h"
#include "limits.h"

#ifndef NULL
#define NULL 0
#endif

#ifndef EXIT_SUCCESS
#define EXIT_SUCCESS 0
#endif

#ifndef EXIT_FAILURE
#define EXIT_FAILURE 1
#endif

/* Exit */
void exit(int status);
int atexit(void (*func)(void));
void _Exit(int status); /* NYI */

/* string to num conversion */
int atoi(const char *val);
/* --- NYI --- */
long atol(const char *val);
float atof(const char *val);

#define atoi(val) ((int)strtol(val, NULL, 10))
#define atol(val)       strtol(val, NULL, 10)
#define atolf(val)      strtof(val, NULL)

long strtol(const char *nptr, char **endptr, int base);
long long strtoll(const char *nptr, char **endptr, int base);
double strtod(const char *nptr, char **endptr);
float strtof(const char *nptr, char **endptr);
long double strtold(const char *nptr, char **endptr);
/* --- END NYI --- */


/* Malloc library */
void *malloc(size_t size);
void free(void *ptr);
void *realloc(void *ptr, size_t size);
void *calloc(size_t nelem, size_t elsize);

#define RAND_MAX INT_MAX

int  rand(void);
void srand(unsigned int seed);
