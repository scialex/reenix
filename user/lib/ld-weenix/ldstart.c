/*
 *  File: ldstart.c
 *  Date: 14 March 1998
 *  Acct: David Powell (dep)
 *  Desc: A run time linker
 *
 *
 *  Acct: Rob Manchester (rmanches)
 *  Desc: Added x86 Elf Compatability
 */

#include "sys/types.h"
#include "stdlib.h"
#include "string.h"
#include "stdio.h"
#include "unistd.h"
#include "fcntl.h"
#include "sys/mman.h"

#include "elf.h"
#include "weenix/syscall.h"

#include "ldutil.h"
#include "ldtypes.h"
#include "ldresolve.h"
#include "ldnames.h"
#include "ldalloc.h"

#ifndef DEFAULT_RUNPATH
#define DEFAULT_RUNPATH "/lib:/usr/lib"
#endif

extern int _ldbindnow(module_t *curmod);

static const char *default_runpath =
        DEFAULT_RUNPATH;

static const char *err_cantfind =
        "ld.so.1: panic - unable to find library \"%s\"\n";
static const char *err_noentry =
        "ld.so.1: panic - no entry point\n";
static const char *err_mapping =
        "ld.so.1: panic - failure to map section of length 0x%x at 0x%x\n";
static const char *err_zeromap =
        "ld.so.1: panic - failure to map /dev/zero\n";

static module_t *_ldfirst;
static module_t **_ldlast;

static int      pagesize;
static char     **env;
ldenv_t _ldenv;


#define trunc_page(x) ((x) & ~(pagesize - 1))
#define round_page(x) (((x) + pagesize - 1) & ~(pagesize - 1))


static const char *_ldgetenv(const char *var)
{
        char **e = env;
        while (*e) {
                const char *p = *e;
                while (*p == *var)
                        p++, var++;
                if (*p == '=' && *var == 0) {
                        return p++;
                }
                e++;
        }
        return 0;
}

static void _ldenv_init(char **environ)
{
        env = environ;
        if (_ldgetenv("LD_BIND_NOW")) {
                _ldenv.ld_bind_now = 1;
        }
        if (_ldgetenv("LD_DEBUG")) {
                _ldenv.ld_debug = 1;
        }
        _ldenv.ld_preload = _ldgetenv("LD_PRELOAD");
        _ldenv.ld_library_path = _ldgetenv("LD_LIBRARY_PATH");
}

static module_t *_ldlinkobj(module_t *info, char *baseaddr, Elf32_Dyn *dyn)
{
        Elf32_Dyn       *curdyn;
        module_t        **curmod;
        char            *name;

        /* uint32_t     d_needed = 0; */
        uint32_t        d_rpath = 0;
        uint32_t        d_relocsz = 0;
        uint32_t        d_relocent = 0;

        uint32_t        d_plttype = 0;
        uint32_t        d_pltsize = 0;

        /* create an info structure if we weren't passed one */
        if (!info) {
                info = (module_t *) _ldalloc(sizeof(*info));
                memset(info, 0, sizeof(*info));
                _ldfirst = info->first = info;
                curmod = &(info->next);
        } else {
                curmod = _ldlast;
        }

        info->base = (unsigned long) baseaddr;

        for (curdyn = dyn; curdyn->d_tag != DT_NULL; curdyn++) {
                switch (curdyn->d_tag) {
                        case DT_HASH:
                                info->hash = (void *)(info->base + curdyn->d_un.d_ptr);
                                break;
                        case DT_SYMTAB:
                                info->dynsym = (void *)(info->base + curdyn->d_un.d_ptr);
                                break;
                        case DT_STRTAB:
                                info->dynstr = (char *)(info->base + curdyn->d_un.d_ptr);
                                break;
                        case DT_JMPREL:
                                info->pltreloc = (void *)(info->base + curdyn->d_un.d_ptr);
                                break;
                        case DT_RELA:
                        case DT_REL:
                                info->reloc = (void *)(info->base + curdyn->d_un.d_ptr);
                                break;
                        case DT_INIT:
                                info->init = (ldfunc_t)(info->base + curdyn->d_un.d_ptr);
                                break;
                        case DT_FINI:
                                info->fini = (ldfunc_t)(info->base + curdyn->d_un.d_ptr);
                                break;
                        case DT_NEEDED:
                                /* d_needed =  curdyn->d_un.d_val; */
                                break;
                        case DT_RPATH:
                                d_rpath = curdyn->d_un.d_val;
                                break;
                        case DT_PLTGOT:
                                info->pltgot = (void *)(info->base + curdyn->d_un.d_ptr);
                                break;
                        case DT_PLTRELSZ:
                                d_pltsize = curdyn->d_un.d_val;
                                break;
                        case DT_PLTREL:
                                d_plttype = curdyn->d_un.d_val;
                                break;
                        case DT_BIND_NOW:
                                _ldenv.ld_bind_now = 1;
                                break;
                        case DT_RELENT:
                        case DT_RELAENT:
                                d_relocent = curdyn->d_un.d_val;
                                break;
                        case DT_RELSZ:
                        case DT_RELASZ:
                                d_relocsz = curdyn->d_un.d_val;
                                break;
                        default:
                                break;
                }
        }

        if (info->reloc) {
                info->nreloc = d_relocsz / d_relocent;
        }

        if (info->pltreloc) {
                /* bytes per element */
                int bpe = d_plttype == DT_REL ?
                          sizeof(Elf32_Rel) : sizeof(Elf32_Rela);
                info->npltreloc = d_pltsize / bpe;
        }

        /* Set up plt */
        if (info->pltgot) {
                _ldpltgot_init(info);
        }

        /* create modules for dependencies */
        for (curdyn = dyn; curdyn->d_tag != DT_NULL; curdyn++) {
                if (curdyn->d_tag == DT_NEEDED) {
                        name = info->dynstr + curdyn->d_un.d_val;
                        if (_ldchkname(name))
                                break;
                        _ldaddname(name);
                        *curmod = (module_t *)_ldalloc(sizeof(module_t));
                        (**curmod).name = name;
                        _ldaddname((**curmod).name);
                        if (d_rpath) {
                                (**curmod).runpath =
                                        info->dynstr + d_rpath;
                        } else {
                                (**curmod).runpath = NULL;
                        }
                        (**curmod).next = NULL;
                        (**curmod).first = _ldfirst;
                        curmod = &((**curmod).next);
                }
        }
        _ldlast = curmod;

        return info;
}


/* Given a filename and a colon-delimited path, this function attempts
 * to open the named file using each element of the path as a prefix
 * for the file.  The result of the first successful open is returned,
 * otherwise -1 is returned */

int _ldtryopen(const char *filename, const char *path)
{
        char            buffer[2048];   /* shouldn't be overflown */
        const char      *pos, *oldpos;
        int             len, flen;
        int             fd;

        if (!path || !*path)
                return -1;

        flen = strlen(filename) + 1;

        oldpos = pos = path;

        /* ADDED: try w/ no prefix first */
        strncpy(buffer, filename, flen);
        fd = open(buffer, O_RDONLY, 0);
        if (fd >= 0) {
                return fd;
        }
        /* END ADDED */

        while (*pos) {

                for (; (*pos) && (*pos != ':'); pos++)
                        /* LINTED */
                        ;
                len = pos - oldpos;
                strncpy(buffer, oldpos, len + 1);
                buffer[len] = '/';
                strncpy(buffer + len + 1, filename, flen);

                fd = open(buffer, O_RDONLY, 0);
                if (fd >= 0) {
                        return fd;
                }

                oldpos = ++pos;
        }

        return -1;
}


/* This function maps the specified section of a shared library.  It
 * also handles the special cases involving the bss and other
 * anonymously mapped areas.  */

/*
 *   ----------------------- top
 *   |                     |          (+phdr->p_memsz)
 *   |  anonymous mapping  |
 *   |        (bss)        |
 *   |                     |
 *   ----------------------- mid2
 *   ----------------------- ztop
 *   |   zeroed out file   |
 *   ----------------------- zbegin   (+phdr->p_filesz)
 *   |                     | mid1
 *   |     mapped file     |
 *   |                     |
 *   |                     |
 *   ----------------------- bottom   (+0)
 */

void _ldmapsect(int fd, unsigned long baseaddr, Elf32_Phdr *phdr, int textrel)
{
        uintptr_t vmaddr = ((uintptr_t) phdr->p_vaddr) + baseaddr;
        uintptr_t offset = phdr->p_offset;
        uintptr_t memsz = phdr->p_memsz;
        uintptr_t filsz = phdr->p_filesz;

        uintptr_t map_addr = trunc_page(vmaddr);
        uintptr_t file_addr = trunc_page(offset);
        uintptr_t map_len;
        uintptr_t copy_len;
        int perms = 0;

        if (phdr->p_flags & PF_R)
                perms |= PROT_READ;
        if (phdr->p_flags & PF_W)
                perms |= PROT_WRITE;
        if (phdr->p_flags & PF_X)
                perms |= PROT_EXEC;

        /* Check if read-only sections will need relocation */
        if (textrel)
                perms |= PROT_WRITE;

        if (memsz > filsz) {
                map_len = trunc_page(offset + filsz) - file_addr;
        } else {
                map_len = round_page(offset + filsz) - file_addr;
        }

        if (map_len != 0) {
                if (mmap((char *) map_addr, map_len, perms,
                         ((perms & PROT_WRITE) ? MAP_PRIVATE : MAP_SHARED) | MAP_FIXED,
                         fd, file_addr) == MAP_FAILED) {
                        printf(err_mapping, map_len, map_addr);
                        exit(1);
                }
        }

        if (memsz == filsz) {
                return;
        }

        file_addr = trunc_page(offset + filsz);
        copy_len = (offset + filsz) - file_addr;
        map_addr = trunc_page(vmaddr + filsz);
        map_len = round_page(vmaddr + memsz) - map_addr;

        if (map_len != 0) {
                void *addr;
                int zfd = _ldzero();
                addr = mmap((char *)map_addr, map_len, perms,
                            MAP_PRIVATE | MAP_FIXED, zfd, 0);
                if (addr == MAP_FAILED) {
                        printf(err_zeromap);
                        exit(1);
                }
                close(zfd);

                if (copy_len != 0) {
                        lseek(fd, file_addr, SEEK_SET);
                        read(fd, addr, copy_len);
                }
        }

}

/* This function finds and maps the shared object associated with the
 * specified module.  When it is done, it calls _ldlinkobj to perform
 * additional operations pertaining to the object's dependencies as
 * well as the managment of the object at runtime */

void _ldloadobj(module_t *module)
{
        unsigned long   bottom, top, size;
        Elf32_Ehdr      *hdr;
        Elf32_Phdr      *phdr;
        Elf32_Dyn       *dyn = 0;
        char            *loc;
        int             fd, i;

        /* attempt to open library */
        fd = _ldtryopen(module->name, _ldenv.ld_library_path);
        if (fd == -1)
                fd = _ldtryopen(module->name, module->runpath);
        if (fd == -1)
                fd = _ldtryopen(module->name, default_runpath);
        if (fd == -1) {
                printf(err_cantfind, module->name);
                exit(1);
        }

        /* compute image size */
        hdr = (Elf32_Ehdr *)mmap(0, pagesize, PROT_READ | PROT_EXEC,
                                 MAP_SHARED, fd, 0);
        phdr = (Elf32_Phdr *)(hdr->e_phoff + (unsigned long)hdr);

        bottom = (unsigned long) - 1;
        top = 0;
        for (i = 0; i < hdr->e_phnum; i++) {
                if (phdr[i].p_type == PT_LOAD) {
                        if (phdr[i].p_vaddr < bottom)
                                bottom = phdr[i].p_vaddr;
                        if (phdr[i].p_vaddr + phdr[i].p_memsz > top)
                                top = phdr[i].p_vaddr + phdr[i].p_memsz;
                }
        }

        bottom = trunc_page(bottom);
        top = round_page(top);
        size = top - bottom;

        loc = (char *)mmap(NULL, size, PROT_NONE, MAP_SHARED, fd, 0);
        munmap(loc, size);

        /* Figure out whether or not things marked readonly need to
         * be writeable (find DT_TEXTREL). This is kind of a mess,
         * as we have to do this before we've mapped in the dynamic
         * section (need to read from file directly). */
        int dynoff;
        Elf32_Dyn curdyn;
        int textrel = 0;
        for (i = 0; i < hdr->e_phnum; i++) {
                if (phdr[i].p_type == PT_DYNAMIC) {
                        dynoff = phdr[i].p_offset;
                        break;
                }
        }
        lseek(fd, dynoff, SEEK_SET);
        do {
                if ((int)sizeof(curdyn) > read(fd, &curdyn, sizeof(curdyn)))
                        exit(1);

                if (curdyn.d_tag == DT_TEXTREL) {
                        textrel = 1;
                        break;
                }
        } while (curdyn.d_tag != DT_NULL);

        for (i = 0; i < hdr->e_phnum; i++) {
                if (phdr[i].p_type == PT_LOAD)
                        _ldmapsect(fd, (unsigned long)loc - bottom, phdr + i, textrel);
                else if (phdr[i].p_type == PT_DYNAMIC)
                        dyn = (Elf32_Dyn *)(loc + phdr[i].p_vaddr);
        }
        munmap(hdr, pagesize);
        close(fd);

        /* set up additional module information */
        _ldlinkobj(module, loc - bottom, dyn);
}

void _ldcleanup(int status)
{
        module_t        *curmod;

        /* Call .fini functions */  /* XXX: fix ordering */
        curmod = _ldfirst->next;
        while (curmod) {
                if (curmod->fini)
                        curmod->fini();
                curmod = curmod->next;
        }

        exit(status);
}


/* This is function is about as close to a 'mainline' as you will find
 * in the linker loader. We initiate the process of
 * evaluating and loading the dependencies of the executable.  Next we
 * relocate all the loaded modules, followed by calling the _init
 * function of all the dependencies.  Lastly, we return the entry point
 * to the calling function (the bootstrap code), which runs the now
 * linked process
 */

ldinit_t _ldstart(char **environ, auxv_t *auxv)
{

        unsigned long   abuf[10];
        module_t        *curmod;
        Elf32_Phdr      *phdr;
        uint32_t        i;

        /* Populate the auxv array */
        memset(abuf, 0, 10 * sizeof(unsigned long));
        for (i = 0; auxv[i].a_type != AT_NULL; i++) {
                if (auxv[i].a_type < 10) {
                        abuf[auxv[i].a_type] = auxv[i].a_un.a_val;
                }
        }

        pagesize = abuf[AT_PAGESZ];

        /* Set up memory pool */
        _ldainit(pagesize, 1);

        _ldenv_init(environ);
        /* Load the executable and all of it's dependencies */
        phdr = (Elf32_Phdr *)abuf[AT_PHDR];

        for (i = 0; i < abuf[AT_PHNUM]; i++) {
                if (phdr[i].p_type == PT_DYNAMIC) {
                        _ldlinkobj(NULL, (char *)0, (Elf32_Dyn *)phdr[i].p_vaddr);
                        break;
                }
        }

        curmod = _ldfirst->next;
        while (curmod) {
                _ldloadobj(curmod);
                curmod = curmod->next;
        }

        /* Perform all necessary relocations */
        /* We relocate the current module (executable) last, as it is the only one that will
         * contain R_386_COPY entries, and we need to make sure the things being
         * copied are correctly relocated (they are probably R_386_RELATIVE) prior
         * to copying */
        curmod = _ldfirst->next; /* Assume at least one module... */
        while (curmod) {
                _ldrelocobj(curmod);
                curmod = curmod->next;
        }
        _ldrelocobj(_ldfirst);

        curmod = _ldfirst;
        while (curmod) {
                _ldrelocplt(curmod);
                curmod = curmod->next;
        }

        if (_ldenv.ld_bind_now) {
                curmod = _ldfirst;
                while (curmod) {
                        _ldbindnow(curmod);
                        curmod = curmod->next;
                }
        }

        /* Call .init functions */  /* XXX: fix ordering */
        curmod = _ldfirst->next;
        while (curmod) {
                if (curmod->init) {
                        curmod->init();
                }
                curmod = curmod->next;
        }

        /* Jump to the linkee's entry point */
        _ldverify(!abuf[AT_ENTRY], err_noentry);
        return (ldinit_t) abuf[AT_ENTRY];
}
