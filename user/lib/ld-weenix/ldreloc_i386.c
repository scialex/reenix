/*
 *  File: ldreloc_i386.c
 *  Date: Oct 22 2002
 *  Acct: Rob Manchester (rmanches)
 *  Desc: x86 elf runtime linker
 */

#include "sys/types.h"
#include "stdlib.h"
#include "stdio.h"

#include "ldtypes.h"
#include "ldresolve.h"
#include "ldutil.h"

extern void _ld_bind(module_t *);

/*
 * We locate argc, argv, envp and the auxv_t array, then we have to
 * relocate ourselfs before we can use global data.  Since we link
 * ourselfs with -Bsymbolic we will only have relocations of type
 * R_386_RELATIVE which are pretty easy to resolve.  Once we finish with
 * that we call _ldstart which will take care of loading any shared libs
 * that the exec depends on (the kernel loaded the exe, though, we still
 * have to link it).  _ldstart returns the address of the exe's start
 * function.
 *
 */

typedef struct args_frame_t {
        int             argc;
        char            **argv;
        char            **env;
} args_frame_t;

ldinit_t _ldloadrtld(int argc, char **argv, char **envp, Elf32_auxv_t *auxv)
{
        Elf32_Ehdr      *hdr;
        Elf32_Phdr      *phdr;
        Elf32_Dyn       *dyn;
        Elf32_Rel       *rel;
        uint32_t        base = 0;
        uint32_t        d_reloff = 0, d_relcount = 0;
        uint32_t i;

        /* Find our own base address in the auxv array */
        for (i = 0; auxv[i].a_type != AT_NULL; i++) {
                if (auxv[i].a_type == AT_BASE) {
                        base = auxv[i].a_un.a_val;
                        break;
                }
        }

        /* Make sure we are ourselves (the kernel didn't goof) */
        hdr = (Elf32_Ehdr *) base;
        if (hdr->e_ident[EI_MAG0] != ELFMAG0 ||
            hdr->e_ident[EI_MAG1] != ELFMAG1 ||
            hdr->e_ident[EI_MAG2] != ELFMAG2 ||
            hdr->e_ident[EI_MAG3] != ELFMAG3) {
                exit(1);
        }

        /* Find our program header */
        phdr = (Elf32_Phdr *)(hdr->e_phoff + base);

        /* Find our dynamic segment */
        while (phdr->p_type != PT_DYNAMIC)
                phdr++;


        /* Find relocation-related entries of the dynamic segment */
        dyn = (Elf32_Dyn *)(phdr->p_vaddr + base);
        for (; dyn->d_tag != DT_NULL; dyn++) {
                if (DT_REL == dyn->d_tag) {
                        d_reloff = dyn->d_un.d_ptr;
                } else if (DT_RELCOUNT == dyn->d_tag) {
                        d_relcount = dyn->d_un.d_val;
                }
        }


        /* Relocate ourselves */
        rel = (Elf32_Rel *)(d_reloff + base);

        for (i = 0; i < d_relcount; i++) {
                uint32_t type = ELF32_R_TYPE(rel[i].r_info);
                if (type == R_386_RELATIVE) {
                        uint32_t *vaddr  = (uint32_t *)(rel[i].r_offset + base);
                        uint32_t  addend = *vaddr;
                        if (*vaddr != addend + base) {
                                *vaddr = addend + base;
                        }
                } else {
                        exit(0);
                }
        }

        /* We're all set up. Now relocate the executable */
        return _ldstart(envp, auxv);
}



void _ldrelocobj(module_t *module)
{
        int             i, sym, type;
        const char     *name;
        ldsym_t         symbol;
        Elf32_Addr      offset, *addr;
        Elf32_Addr      base = module->base;

        for (i = 0; i < module->nreloc; i++) {
                sym = ELF32_R_SYM(module->reloc[i].r_info);
                type = ELF32_R_TYPE(module->reloc[i].r_info);
                name = module->dynstr + module->dynsym[sym].st_name;
                offset = module->reloc[i].r_offset;
                addr = (Elf32_Addr *)(offset + base);
                size_t size;

                switch (type) {
                                /* Position-independent code should ONLY contain the next 4 types */
                        case R_386_RELATIVE:
                                *addr += base;
                                break;
                        case R_386_COPY:
                                symbol = _ldresolve(module, name, -1, &size, 1);
                                /* memcpy: symbol to addr */
                                char *dest = (char *) addr;
                                char *src = (char *) symbol;
                                while (size--)
                                        *dest++ = *src++;
                                break;

                                /* TODO this never actually gets called; it's overwritten by the
                                 * calls to _ldrelocplt and _ldbindnow */
                        case R_386_JMP_SLOT:
                                symbol = _ldresolve(module, name, -1, &size, 0);
                                if (symbol == 0) {
                                        /* HUH? */
                                        return;
                                }
                                /* Only eager binding for now: TODO */
                                *(uint32_t *) addr = (uint32_t) symbol;
                                break;
                        case R_386_GLOB_DAT:
                                symbol = _ldresolve(module, name, -1, 0, 0);
                                if (symbol == 0) {
                                        /* HUH? */
                                        return;
                                }
                                *(uint32_t *) addr = (uint32_t) symbol;
                                break;
                                /* For non-PIC (requires modifying text) */
                        case R_386_32:
                                symbol = _ldresolve(module, name, -1, 0, 0);
                                if (symbol == 0) {
                                        /* HUH? */
                                        return;
                                }
                                *addr += (Elf32_Addr) symbol;
                                break;
                        case R_386_PC32:
                                symbol = _ldresolve(module, name, -1, 0, 0);
                                if (symbol == 0) {
                                        /* HUH? */
                                        return;
                                }
                                *addr += (Elf32_Addr) symbol - (Elf32_Addr) addr;
                                break;
                        default:
                                printf("Unknown relocation type %d\n", type);
                                exit(1);
                                break;
                }
        }
}

void _ldrelocplt(module_t *module)
{
        int                     i, max;
        const Elf32_Rel        *rel = module->pltreloc;

        max = module->npltreloc;
        for (i = 0; i < max; i++) {
                if (ELF32_R_TYPE(rel[i].r_info) != R_386_JMP_SLOT) {
                        printf("Unknown relocation type %d\n",
                               ELF32_R_TYPE(rel[i].r_info));
                        exit(1);
                }
                *(Elf32_Addr *)(module->base + rel[i].r_offset) += module->base;
        }
}

void
_ldpltgot_init(module_t *module)
{
        Elf32_Addr *pltbase = module->pltgot;
        pltbase[1] = (Elf32_Addr) module;
        pltbase[2] = (Elf32_Addr) &_ld_bind;
}

void
_ldbindnow(module_t *mod)
{
        int                     i, max, sym, type;
        const char             *name;
        ldsym_t                 symbol;
        const Elf32_Rel        *rel = mod->pltreloc;
        Elf32_Addr *addr;

        max = mod->npltreloc;
        for (i = 0; i < max; i++) {
                sym = ELF32_R_SYM(rel[i].r_info);
                type = ELF32_R_TYPE(rel[i].r_info);
                name = mod->dynstr + mod->dynsym[sym].st_name;
                addr = (Elf32_Addr *)(mod->base + rel[i].r_offset);

                if (sym && type == R_386_JMP_SLOT) {
                        symbol = _ldresolve(mod, name, -1, 0, 0);
                        *addr = (Elf32_Addr)symbol;
                }
        }
}

