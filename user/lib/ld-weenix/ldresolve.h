/*
 *  File: ldresolve.h
 *  Date: 12 April 1998
 *  Acct: David Powell (dep)
 *  Desc: Various symbol resolution functions
 */

#ifndef _ldresolve_h_
#define _ldresolve_h_

#ifdef  __cplusplus
extern "C" {
#endif

#include "ldtypes.h"

        int _ldlookup(module_t *module, const char *name);
        ldsym_t _ldsymbol(module_t *module, const char *name, int binding, int type,
                          Elf32_Word *size);
        ldsym_t _ldresolve(module_t *module, const char *name, int type,
                           Elf32_Word *size, int copy);
        ldsym_t _ldexresolve(module_t *module, const char *name, int type,
                             Elf32_Word *size);

#ifdef  __cplusplus
}
#endif

#endif /* _ldresolve_h_ */

