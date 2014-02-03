/*
 *  File: ldnames.c
 *  Date: 30 March 1998
 *  Acct: David Powell (dep)
 *  Desc:
 */

#include "sys/types.h"

#include "stdlib.h"
#include "string.h"

#include "ldnames.h"
#include "ldalloc.h"

typedef struct modent modent_t;
struct modent {
        const char      *name;
        modent_t        *next;
};

static modent_t *names = NULL;


/* This function adds a name to the global name list.  This is intended
 * for keeping track of what libraries have already been loaded so that
 * circular/multiple dependencies don't result in a collosal mess.
 *
 * _ldaddname has the caveat that names passed must stick around; this
 * works fine for names located in the dynstr section and are
 * referenced in the dynamic section */

void _ldaddname(const char *name)
{
        modent_t        *newent;

        newent = (modent_t *)_ldalloc(sizeof(*newent));
        newent->name = name;
        newent->next = names;
        names = newent;
}


/* This function checks to see if the specified name has already been
 * added to the name list (via _ldaddname).  If so, 1 is returned.
 * Otherwise, 0 is returned. */

int _ldchkname(const char *name)
{
        /* Just does a linear search - nothing fancy here */

        modent_t        *curent;

        curent = names;
        while (curent) {
                if (strcmp(curent->name, name))
                        curent = curent->next;
                else
                        return 1;
        }

        return 0;
}

