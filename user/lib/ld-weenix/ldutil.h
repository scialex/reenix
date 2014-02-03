/*
 *  File: ldutil.h
 *  Date: 15 March 1998
 *  Acct: David Powell (dep)
 *  Desc: Miscellanious utility functions
 */

#ifndef _ldutil_h_
#define _ldutil_h_
#include "ldtypes.h"
#ifdef  __cplusplus
extern "C" {
#endif

        void _ldverify(int test, const char *msg);
        int _ldzero();

        unsigned long _ldelfhash(const char *name);
        int _ldtryopen(const char *filename, const char *path);
        void _ldmapsect(int fd, unsigned long baseaddr, Elf32_Phdr *phdr, int textrel);
        void _ldloadobj(module_t *module);
        void _ldrelocobj(module_t *module);
        void _ldcleanup();
        ldinit_t _ldstart(char **environ, auxv_t *auxv);

        void _ldrelocplt(module_t *module);
        void _ldpltgot_init(module_t *module);

#ifdef  __cplusplus
}
#endif

#endif /* _ldutil_h_ */

