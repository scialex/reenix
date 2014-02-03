/*
 *  File: ldtypes.h
 *  Date: 12 April 1998
 *  Acct: David Powell (dep)
 *  Desc:
 */

#ifndef _ldtypes_h_
#define _ldtypes_h_

#include "elf.h"

#define LD_ERR_EXIT     13

typedef Elf32_auxv_t auxv_t;    /* linux is funky */

typedef int(*ldfunc_t)();
typedef void *ldsym_t;
typedef void (*ldinit_t)(int argc, char **argv, char **environ, auxv_t *auxv);

typedef struct ldenv_t {
        int ld_bind_now;
        int ld_debug;
        const char *ld_preload;
        const char *ld_library_path;
} ldenv_t;

extern ldenv_t _ldenv;

typedef struct module_t module_t;
struct module_t {
        char            *name;          /* the filename                 */
        char            *runpath;       /* the run path to use          */

        unsigned long   base;           /* base address of module       */
        Elf32_Word      *hash;          /* the module's hash table      */
        Elf32_Sym       *dynsym;        /* the dynamic symbol table     */
        char            *dynstr;        /* the dynamic string table     */

        ldfunc_t        init;           /* module initialization fcn.   */
        ldfunc_t        fini;           /* module shutdown fcn.         */

        Elf32_Rel       *pltreloc;      /* PLT relocations              */
        Elf32_Rel       *reloc;         /* normal relocations           */
        /* ADDED: Non-IA-32 version */
#if 0
        Elf32_Rela      *pltreloc;      /* PLT relocations              */
        Elf32_Rela      *reloc;         /* normal relocations           */
#endif
        int             nreloc;         /* number of relocation entries */
        int             npltreloc;      /* number of relocation entries */

        module_t        *next;          /* the next module in the chain */
        module_t        *first;         /* the first module             */
        Elf32_Addr      *pltgot;        /* base of plt                  */
};

#endif /* _ldtypes.h_ */

