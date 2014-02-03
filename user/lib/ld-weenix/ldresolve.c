/*
 *  File: ldresolve.c
 *  Date: 12 April 1998
 *  Acct: David Powell (dep)
 *  Desc: Various symbol resolution functions
 */

#include "string.h"

#include "ldresolve.h"
#include "ldutil.h"

#define H_nbucket       0
#define H_nchain        1
#define H_bucket        2

/* This function looks up the specified symbol in the specified
 * module.  If the symbol is present, it returns the symbol's index in
 * the dynamic symbol table, otherwise STN_UNDEF is returned. */

int _ldlookup(module_t *module, const char *name)
{
        unsigned long   hashval;
        unsigned long   y;

        hashval = _ldelfhash(name);
        hashval %= module->hash[H_nbucket];

        y = module->hash[H_bucket + hashval];

        while ((y != STN_UNDEF) &&
               strcmp(module->dynstr + module->dynsym[y].st_name, name)) {
                y = module->hash[H_bucket + module->hash[H_nbucket] + y];
        }

        return y;
}


/* This looks up the specified symbol in the given module, subject to
 * the provided binding and type restrictions (a value of -1 will
 * function as a wildcard for both the 'binding' and 'type'
 * parameters).  The symbol's size will be placed in the memory
 * location pointed to by 'size', if it is non-null.  0 is returned if
 * a symbol matching all the requirements is not found. */

ldsym_t _ldsymbol(module_t *module, const char *name, int binding, int type,
                  Elf32_Word *size)
{
        int     result;

        /* LINTED */
        if (((result = _ldlookup(module, name)) != STN_UNDEF) &&
            ((binding < 0) ||
             (ELF32_ST_BIND(module->dynsym[result].st_info) == binding)) &&
            ((type < 0) ||
             (ELF32_ST_TYPE(module->dynsym[result].st_info) == type)) &&
            (module->dynsym[result].st_shndx != SHN_UNDEF)) {
                if (size)
                        *size = module->dynsym[result].st_size;
                return (ldsym_t)((uintptr_t)module->base +
                                 (uintptr_t)module->dynsym[result].st_value);
        }

        return 0;
}


/* Given a module and a symbol name, this function attempts to find the
 * symbol through the process' link chain.  It first checks for its
 * presence as a global symbol, then as a weak symbol, and finally as a
 * local symbol in the specified module.  A type restriction can be
 * specified, and if 'size' is non-null, the memory location to which
 * it points will hold the size of the resolved symbol.  0 is returned
 * if the symbol cannot be found. */

ldsym_t _ldresolve(module_t *module, const char *name, int type,
                   Elf32_Word *size, int exclude)
{
        module_t        *curmod;
        ldsym_t         sym;

        curmod = module->first;

        while (curmod) {
                if (!exclude || curmod != module) {
                        if ((sym = _ldsymbol(curmod, name, STB_GLOBAL, type, size)))
                                return sym;
                }
                curmod = curmod->next;
        }

        curmod = module->first;
        while (curmod) {
                if ((sym = _ldsymbol(curmod, name, STB_WEAK, type, size)))
                        return sym;
                curmod = curmod->next;
        }

        return _ldsymbol(module, name, STB_LOCAL, type, size);
}

Elf32_Addr _rtresolve(module_t *mod, Elf32_Word reloff)
{
        Elf32_Rel      *rel = (void *)((Elf32_Addr)mod->pltreloc + reloff);
        int             sym = ELF32_R_SYM(rel->r_info);
        const char     *name = mod->dynstr + mod->dynsym[sym].st_name;
        ldsym_t         symbol = _ldresolve(mod, name, -1, 0, 0);
        *(Elf32_Addr *)(mod->base + rel->r_offset) = (Elf32_Addr)symbol;
        return (Elf32_Addr)symbol;
}
