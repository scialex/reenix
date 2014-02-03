#include "errno.h"
#include "globals.h"

#include "mm/mman.h"
#include "mm/mm.h"
#include "mm/tlb.h"
#include "mm/pagetable.h"
#include "mm/kmalloc.h"

#include "vm/vmmap.h"

#include "api/elf.h"
#include "api/binfmt.h"

#include "util/init.h"
#include "util/debug.h"
#include "util/string.h"

#include "fs/file.h"
#include "fs/fcntl.h"
#include "fs/lseek.h"
#include "fs/vfs_syscall.h"

static int _elf32_platform_check(const Elf32_Ehdr *header)
{
        return (EM_386 == header->e_machine)
               && (ELFCLASS32 == header->e_ident[EI_CLASS])
               && (ELFDATA2LSB == header->e_ident[EI_DATA]);
}

/* Helper function for the ELF loader. Maps the specified segment
 * of the program header from the given file in to the given address
 * space with the given memory offset (in pages). On success returns 0, otherwise
 * returns a negative error code for the ELF loader to return.
 * Note that since any error returned by this function should
 * cause the ELF loader to give up, it is acceptable for the
 * address space to be modified after returning an error.
 * Note that memoff can be negative */
static int _elf32_map_segment(vmmap_t *map, vnode_t *file, int32_t memoff, const Elf32_Phdr *segment)
{
        uintptr_t addr;
        if (memoff < 0) {
                KASSERT(ADDR_TO_PN(segment->p_vaddr) > (uint32_t) -memoff);
                addr = (uintptr_t)segment->p_vaddr - (uintptr_t)PN_TO_ADDR(-memoff);
        } else {
                addr = (uintptr_t)segment->p_vaddr + (uintptr_t)PN_TO_ADDR(memoff);
        }
        uint32_t off = segment->p_offset;
        uint32_t memsz = segment->p_memsz;
        uint32_t filesz = segment->p_filesz;

        dbg(DBG_ELF, "Mapping program segment: type %#x, offset %#08x,"
            " vaddr %#08x, filesz %#x, memsz %#x, flags %#x, align %#x\n",
            segment->p_type, segment->p_offset, segment->p_vaddr,
            segment->p_filesz, segment->p_memsz, segment->p_flags,
            segment->p_align);

        /* check for bad data in the segment header */
        if (PAGE_SIZE != segment->p_align) {
                dbg(DBG_ELF, "ERROR: segment does not have correct alignment\n");
                return -ENOEXEC;
        } else if (filesz > memsz) {
                dbg(DBG_ELF, "ERROR: segment file size is greater than memory size\n");
                return -ENOEXEC;
        } else if (PAGE_OFFSET(addr) != PAGE_OFFSET(off)) {
                dbg(DBG_ELF, "ERROR: segment address and offset are not aligned correctly\n");
                return -ENOEXEC;
        }

        int perms = 0;
        if (PF_R & segment->p_flags) {
                perms |= PROT_READ;
        }
        if (PF_W & segment->p_flags) {
                perms |= PROT_WRITE;
        }
        if (PF_X & segment->p_flags) {
                perms |= PROT_EXEC;
        }

        if (0 < filesz) {
                /* something needs to be mapped from the file */
                /* start from the starting address and include enough pages to
                 * map all filesz bytes of the file */
                uint32_t lopage = ADDR_TO_PN(addr);
                uint32_t npages = ADDR_TO_PN(addr + filesz - 1) - lopage + 1;
                off_t fileoff = (off_t)PAGE_ALIGN_DOWN(off);

                int ret;
                if (!vmmap_is_range_empty(map, lopage, npages)) {
                        dbg(DBG_ELF, "ERROR: ELF file contains overlapping segments\n");
                        return -ENOEXEC;
                } else if (0 > (ret = vmmap_map(map, file, lopage, npages, perms,
                                                MAP_PRIVATE | MAP_FIXED, fileoff,
                                                0, NULL))) {
                        return ret;
                }
        }

        if (memsz > filesz) {
                /* there is left over memory in the segment which must
                 * be initialized to 0 (anonymously mapped) */
                uint32_t lopage = ADDR_TO_PN(addr + filesz);
                uint32_t npages = ADDR_TO_PN(PAGE_ALIGN_UP(addr + memsz)) - lopage;

                int ret;
                if (npages > 1 && !vmmap_is_range_empty(map, lopage + 1, npages - 1)) {
                        dbg(DBG_ELF, "ERROR: ELF file contains overlapping segments\n");
                        return -ENOEXEC;
                } else if (0 > (ret = vmmap_map(map, NULL, lopage, npages, perms,
                                                MAP_PRIVATE | MAP_FIXED, 0, 0, NULL))) {
                        return ret;
                } else if (!PAGE_ALIGNED(addr + filesz) && filesz > 0) {
                        /* In this case, we have accidentally zeroed too much of memory, as
                         * we zeroed all memory in the page containing addr + filesz.
                         * However, the remaining part of the data is not a full page, so we
                         * should not just map in another page (as there could be garbage
                         * after addr+filesz). For instance, consider the data-bss boundary
                         * (c.f. Intel x86 ELF supplement pp. 82).
                         * To fix this, we need to read in the contents of the file manually
                         * and put them at that user space addr in the anon map we just
                         * added. */
                        void *buf;
                        if (NULL == (buf = page_alloc()))
                                return -ENOMEM;
                        if (!(0 > (ret = file->vn_ops->read(file, (off_t) PAGE_ALIGN_DOWN(off + filesz),
                                                            buf, PAGE_OFFSET(addr + filesz))))) {
                                ret = vmmap_write(map, PAGE_ALIGN_DOWN(addr + filesz),
                                                  buf, PAGE_OFFSET(addr + filesz));
                        }
                        page_free(buf);
                        return ret;
                }
        }
        return 0;
}

/* Read in the given fd's ELF header into the location pointed to by the given
 * argument and does some basic checks that it is a valid ELF file, is an
 * executable, and is for the correct platform
 * interp is 1 if we are loading an interpreter, 0 otherwise
 * Returns 0 on success, -errno on failure. Returns the ELF header in the header
 * argument. */
static int _elf32_load_ehdr(int fd, Elf32_Ehdr *header, int interp)
{
        int err;
        memset(header, 0, sizeof(*header));

        /* Preliminary check that this is an ELF file */
        if (0 > (err = do_read(fd, header, sizeof(*header)))) {
                return err;
        } else if ((SELFMAG > err) || 0 != memcmp(&header->e_ident[0], ELFMAG, SELFMAG)) {
                dbg(DBG_ELF, "ELF load failed: no magic number present\n");
                return -ENOEXEC;
        } else if (header->e_ehsize > err) {
                dbg(DBG_ELF, "ELF load failed: bad file size\n");
                return -ENOEXEC;
        }
        /* Log information about the file */
        dbg(DBG_ELF, "loading ELF file\n");
        dbgq(DBG_ELF, "ELF Header Information:\n");
        dbgq(DBG_ELF, "Version: %d\n", (int)header->e_ident[EI_VERSION]);
        dbgq(DBG_ELF, "Class:   %d\n", (int)header->e_ident[EI_CLASS]);
        dbgq(DBG_ELF, "Data:    %d\n", (int)header->e_ident[EI_DATA]);
        dbgq(DBG_ELF, "Type:    %d\n", (int)header->e_type);
        dbgq(DBG_ELF, "Machine: %d\n", (int)header->e_machine);

        /* Check that the ELF file is executable and targets
         * the correct platform */
        if (ET_EXEC != header->e_type && !(ET_DYN == header->e_type && interp)) {
                dbg(DBG_ELF, "ELF load failed: not exectuable ELF\n");
                return -ENOEXEC;
        } else if (!_elf32_platform_check(header)) {
                dbg(DBG_ELF, "ELF load failed: incorrect platform\n");
                return -ENOEXEC;
        }
        return 0;
}

/* Loads the program header tables from from the ELF file specified by
 * the open file descriptor fd. header should point to the header information
 * for that ELF file. pht is a buffer of size size. It must be large enough
 * to hold the program header tables (whose size can be determined from
 * the ELF header).
 *
 * Returns 0 on success or -errno on error. */
static int _elf32_load_phtable(int fd, Elf32_Ehdr *header, char *pht, size_t size)
{
        int err = 0;
        size_t phtsize = header->e_phentsize * header->e_phnum;
        KASSERT(phtsize <= size);
        if (0 > (err = do_lseek(fd, header->e_phoff, SEEK_SET))) {
                goto done;
        }
        if (0 > (err = do_read(fd, pht, phtsize))) {
                goto done;
        }
        KASSERT(err <= (int)phtsize);
        if (err < (int)phtsize) {
                err = -ENOEXEC;
                goto done;
        }

        err = 0;
done:
        return err;
}

/* Maps the PT_LOAD segments for an ELF file into the given address space.
 * vnode should be the open vnode of the ELF file.
 * map is the address space to map the ELF file into.
 * header is the ELF file's header.
 * pht is the full program header table.
 * memoff is the difference (in pages) between the desired base address and the
 * base address given in the ELF file (usually 0x8048094)
 *
 * Returns the number of segments loaded on success, -errno on failure. */
static int _elf32_map_progsegs(vnode_t *vnode, vmmap_t *map, Elf32_Ehdr *header, char *pht, int32_t memoff)
{
        int err = 0;

        uint32_t i = 0;
        int loadcount = 0;
        for (i = 0; i < header->e_phnum; ++i) {
                Elf32_Phdr *phtentry = (Elf32_Phdr *)(pht + (i * header->e_phentsize));
                if (PT_LOAD == phtentry->p_type) {
                        if (0 > (err = _elf32_map_segment(map, vnode, memoff, phtentry))) {
                                goto done;
                        } else {
                                ++loadcount;
                        }
                }
        }

        if (0 == loadcount) {
                dbg(DBG_ELF, "ERROR: ELF file contained no loadable sections\n");
                err = -ENOEXEC;
                goto done;
        }

        err = loadcount;
done:
        return err;
}

/* Locates the program header for the interpreter in the given list of program
 * headers through the phinterp out-argument. Returns 0 on success (even if there
 * is no interpreter) or -errno on error. If there is no interpreter section then
 * phinterp is set to NULL. If there is more than one interpreter then -EINVAL is
 * returned. */
static int _elf32_find_phinterp(Elf32_Ehdr *header, char *pht, Elf32_Phdr **phinterp)
{
        int err = 0;
        *phinterp = NULL;

        uint32_t i = 0;
        for (i = 0; i < header->e_phnum; ++i) {
                Elf32_Phdr *phtentry = (Elf32_Phdr *)(pht + (i * header->e_phentsize));
                if (PT_INTERP == phtentry->p_type) {
                        if (NULL == *phinterp) {
                                *phinterp = phtentry;
                        } else {
                                dbg(DBG_ELF, "ELF load failed: multiple interpreters\n");
                                err = -EINVAL;
                                goto done;
                        }
                }
        }

        err = 0;
done:
        return err;
}

/* Calculates the lower and upper virtual addresses that the given program
 * header table would load into if _elf32_load_progsegs were called. We traverse
 * all the program segments of type PT_LOAD and look at p_vaddr and p_memsz
 * Return the low and high vaddrs in the given arguments if they are non-NULL. */
static void _elf32_calc_progbounds(Elf32_Ehdr *header, char *pht, void **low, void **high)
{
        Elf32_Addr curlow = (Elf32_Addr) - 1;
        Elf32_Addr curhigh = 0;
        uint32_t i = 0;
        for (i = 0; i < header->e_phnum; ++i) {
                Elf32_Phdr *phtentry = (Elf32_Phdr *)(pht + (i * header->e_phentsize));
                if (PT_LOAD == phtentry->p_type) {
                        if (phtentry->p_vaddr < curlow)
                                curlow = phtentry->p_vaddr;
                        if (phtentry->p_vaddr + phtentry->p_memsz > curhigh)
                                curhigh = phtentry->p_vaddr + phtentry->p_memsz;
                }
        }
        if (NULL != low)
                *low = (void *) curlow;
        if (NULL != high)
                *high = (void *) curhigh;
}

/* Calculates the total size of all the arguments that need to be placed on the
 * user stack before execution can begin. See Intel i386 ELF supplement pp 54-59
 * Returns total size on success. Returns the number of non-NULL entries in
 * argv, envp, and auxv in argc, envc, and auxc arguments, respectively */
static size_t _elf32_calc_argsize(char *const argv[], char *const envp[], Elf32_auxv_t *auxv,
                                  size_t phtsize, int *argc, int *envc, int *auxc)
{
        size_t size = 0;
        int i;
        /* All strings in argv */
        for (i = 0; argv[i] != NULL; i++) {
                size += strlen(argv[i]) + 1; /* null terminator */
        }
        if (argc != NULL) {
                *argc = i;
        }
        /* argv itself (+ null terminator) */
        size += (i + 1) * sizeof(char *);

        /* All strings in envp */
        for (i = 0; envp[i] != NULL; i++) {
                size += strlen(envp[i]) + 1; /* null terminator */
        }
        if (envc != NULL) {
                *envc = i;
        }
        /* envp itself (+ null terminator) */
        size += (i + 1) * sizeof(char *);

        /* The only extra-space-consuming entry in auxv is AT_PHDR, as if we find
         * that entry we'll need to put the program header table on the stack */
        for (i = 0; auxv[i].a_type != AT_NULL; i++) {
                if (auxv[i].a_type == AT_PHDR) {
                        size += phtsize;
                }
        }
        if (auxc != NULL) {
                *auxc = i;
        }
        /* auxv itself (+ null terminator) */
        size += (i + 1) * sizeof(Elf32_auxv_t);

        /* argc */
        size += sizeof(int);
        /* argv, envp, and auxv pointers (as passed to main) */
        size += 3 * sizeof(void *);

        return size;
}

/* Copies the arguments that must be on the stack prior to execution onto the
 * user stack. This should never fail.
 * arglow:   low address on the user stack where we should start the copying
 * argsize:  total size of everything to go on the stack
 * buf:      a kernel buffer at least as big as argsize (for convenience)
 * argv, envp, auxv: various vectors of stuff (to go on the stack)
 * argc, envc, auxc: number of non-NULL entries in argv, envp, auxv,
 *                   respectively (to avoid recomputing them)
 * phtsize: the size of the program header table (to avoid recomputing)
 * c.f. Intel i386 ELF supplement pp 54-59
 */
static void _elf32_load_args(vmmap_t *map, void *arglow, size_t argsize, char *buf,
                             char *const argv[], char *const envp[], Elf32_auxv_t *auxv,
                             int argc, int envc, int auxc, int phtsize)
{
        int i;

        /* Copy argc */
        *((int *) buf) = argc;

        /* Calculate where the strings / tables pointed to by the vectors start */
        size_t veclen = (argc + 1 + envc + 1) * sizeof(char *) + (auxc + 1) * sizeof(Elf32_auxv_t);

        char *vecstart = buf + sizeof(int) + 3 * sizeof(void *); /* Beginning of argv (in kernel buffer) */

        char *vvecstart = ((char *)arglow) + sizeof(int) + 3 * sizeof(void *); /* Beginning of argv (in user space) */

        char *strstart = vecstart + veclen; /* Beginning of first string pointed to by argv
                                                                                   (in kernel buffer) */

        /* Beginning of first string pointed to by argv (in user space) */
        char *vstrstart = vvecstart + veclen;

        /* Copy over pointer to argv */
        *(char **)(buf + 4) = vvecstart;
        /* Copy over pointer to envp */
        *(char **)(buf + 8) = vvecstart + (argc + 1) * sizeof(char *);
        /* Copy over pointer to auxv */
        *(char **)(buf + 12) = vvecstart + (argc + 1 + envc + 1) * sizeof(char *);

        /* Copy over argv along with every string in it */
        for (i = 0; i < argc; i++) {
                size_t len = strlen(argv[i]) + 1;
                strcpy(strstart, argv[i]);
                /* Remember that we need to use the virtual address of the string */
                *(char **) vecstart = vstrstart;
                strstart += len;
                vstrstart += len;
                vecstart += sizeof(char *);
        }
        /* null terminator of argv */
        *(char **) vecstart = NULL;
        vecstart += sizeof(char *);

        /* Copy over envp along with every string in it */
        for (i = 0; i < envc; i++) {
                size_t len = strlen(envp[i]) + 1;
                strcpy(strstart, envp[i]);
                /* Remember that we need to use the virtual address of the string */
                *(char **) vecstart = vstrstart;
                strstart += len;
                vstrstart += len;
                vecstart += sizeof(char *);
        }
        /* null terminator of envp */
        *(char **) vecstart = NULL;
        vecstart += sizeof(char *);

        /* Copy over auxv along with the program header (if we find it) */
        for (i = 0; i < auxc; i++) {
                /* Copy over the auxv entry */
                memcpy(vecstart, &auxv[i], sizeof(Elf32_auxv_t));
                /* Check if it points to the program header */
                if (auxv[i].a_type == AT_PHDR) {
                        /* Copy over the program header table */
                        memcpy(strstart, auxv[i].a_un.a_ptr, phtsize);
                        /* And modify the address */
                        ((Elf32_auxv_t *)vecstart)->a_un.a_ptr = vstrstart;
                }
                vecstart += sizeof(Elf32_auxv_t);
        }
        /* null terminator of auxv */
        ((Elf32_auxv_t *)vecstart)->a_type = NULL;

        /* Finally, we're done copying into the kernel buffer. Now just copy the
         * kernel buffer into user space */
        int ret = vmmap_write(map, arglow, buf, argsize);
        /* If this failed, we must have set up the address space wrong... */
        KASSERT(0 == ret);
}


static int _elf32_load(const char *filename, int fd, char *const argv[],
                       char *const envp[], uint32_t *eip, uint32_t *esp)
{
        int err = 0;
        Elf32_Ehdr header;
        Elf32_Ehdr interpheader;

        /* variables to clean up on failure */
        vmmap_t *map = NULL;
        file_t *file = NULL;
        char *pht = NULL;
        char *interpname = NULL;
        int interpfd = -1;
        file_t *interpfile = NULL;
        char *interppht = NULL;
        Elf32_auxv_t *auxv = NULL;
        char *argbuf = NULL;

        uintptr_t entry;

        file = fget(fd);
        KASSERT(NULL != file);

        /* Load and verify the ELF header */
        if (0 > (err = _elf32_load_ehdr(fd, &header, 0))) {
                goto done;
        }

        if (NULL == (map = vmmap_create())) {
                err = -ENOMEM;
                goto done;
        }

        size_t phtsize = header.e_phentsize * header.e_phnum;
        if (NULL == (pht = kmalloc(phtsize))) {
                err = -ENOMEM;
                goto done;
        }
        /* Read in the program header table */
        if (0 > (err = _elf32_load_phtable(fd, &header, pht, phtsize))) {
                goto done;
        }
        /* Load the segments in the program header table */
        if (0 > (err = _elf32_map_progsegs(file->f_vnode, map, &header, pht, 0))) {
                goto done;
        }

        Elf32_Phdr *phinterp = NULL;
        /* Check if program requires an interpreter */
        if (0 > (err = _elf32_find_phinterp(&header, pht, &phinterp))) {
                goto done;
        }

        /* Calculate program bounds for future reference */
        void *proglow;
        void *proghigh;
        _elf32_calc_progbounds(&header, pht, &proglow, &proghigh);

        entry = (uintptr_t) header.e_entry;

        /* if an interpreter was requested load it */
        if (NULL != phinterp) {
                /* read the file name of the interpreter from the binary */
                if (0 > (err = do_lseek(fd, phinterp->p_offset, SEEK_SET))) {
                        goto done;
                } else if (NULL == (interpname = kmalloc(phinterp->p_filesz))) {
                        err = -ENOMEM;
                        goto done;
                } else if (0 > (err = do_read(fd, interpname, phinterp->p_filesz))) {
                        goto done;
                }
                if (err != (int)phinterp->p_filesz) {
                        err = -ENOEXEC;
                        goto done;
                }

                /* open the interpreter */
                dbgq(DBG_ELF, "ELF Interpreter: %*s\n", phinterp->p_filesz, interpname);
                if (0 > (interpfd = do_open(interpname, O_RDONLY))) {
                        err = interpfd;
                        goto done;
                }
                kfree(interpname);
                interpname = NULL;

                interpfile = fget(interpfd);
                KASSERT(NULL != interpfile);

                /* Load and verify the interpreter ELF header */
                if (0 > (err = _elf32_load_ehdr(interpfd, &interpheader, 1))) {
                        goto done;
                }
                size_t interpphtsize = interpheader.e_phentsize * interpheader.e_phnum;
                if (NULL == (interppht = kmalloc(interpphtsize))) {
                        err = -ENOMEM;
                        goto done;
                }
                /* Read in the program header table */
                if (0 > (err = _elf32_load_phtable(interpfd, &interpheader, interppht, interpphtsize))) {
                        goto done;
                }

                /* Interpreter shouldn't itself need an interpreter */
                Elf32_Phdr *interpphinterp;
                if (0 > (err = _elf32_find_phinterp(&interpheader, interppht, &interpphinterp))) {
                        goto done;
                }
                if (NULL != interpphinterp) {
                        err = -EINVAL;
                        goto done;
                }

                /* Calculate the interpreter program size */
                void *interplow;
                void *interphigh;
                _elf32_calc_progbounds(&interpheader, interppht, &interplow, &interphigh);
                uint32_t interpnpages = ADDR_TO_PN(PAGE_ALIGN_UP(interphigh)) - ADDR_TO_PN(interplow);

                /* Find space for the interpreter */
                /* This is the first pn at which the interpreter will be mapped */
                uint32_t interppagebase = (uint32_t) vmmap_find_range(map, interpnpages, VMMAP_DIR_HILO);
                if ((uint32_t) - 1 == interppagebase) {
                        err = -ENOMEM;
                        goto done;
                }

                /* Base address at which the interpreter begins on that page */
                void *interpbase = (void *)((uintptr_t)PN_TO_ADDR(interppagebase) + PAGE_OFFSET(interplow));

                /* Offset from "expected base" in number of pages */
                int32_t interpoff = (int32_t) interppagebase - (int32_t) ADDR_TO_PN(interplow);

                entry = (uintptr_t) interpbase + ((uintptr_t) interpheader.e_entry - (uintptr_t) interplow);

                /* Load the interpreter program header and map in its segments */
                if (0 > (err = _elf32_map_progsegs(interpfile->f_vnode, map, &interpheader, interppht, interpoff))) {
                        goto done;
                }

                /* Build the ELF aux table */
                /* Need to hold AT_PHDR, AT_PHENT, AT_PHNUM, AT_ENTRY, AT_BASE,
                 * AT_PAGESZ, AT_NULL */
                if (NULL == (auxv = (Elf32_auxv_t *) kmalloc(7 * sizeof(Elf32_auxv_t)))) {
                        err = -ENOMEM;
                        goto done;
                }
                Elf32_auxv_t *auxvent = auxv;

                /* Add all the necessary entries */
                auxvent->a_type = AT_PHDR;
                auxvent->a_un.a_ptr = pht;
                auxvent++;

                auxvent->a_type = AT_PHENT;
                auxvent->a_un.a_val = header.e_phentsize;
                auxvent++;

                auxvent->a_type = AT_PHNUM;
                auxvent->a_un.a_val = header.e_phnum;
                auxvent++;

                auxvent->a_type = AT_ENTRY;
                auxvent->a_un.a_ptr = (void *) header.e_entry;
                auxvent++;

                auxvent->a_type = AT_BASE;
                auxvent->a_un.a_ptr = interpbase;
                auxvent++;

                auxvent->a_type = AT_PAGESZ;
                auxvent->a_un.a_val = PAGE_SIZE;
                auxvent++;

                auxvent->a_type = AT_NULL;

        } else {
                /* Just put AT_NULL (we don't really need this at all) */
                if (NULL == (auxv = (Elf32_auxv_t *) kmalloc(sizeof(Elf32_auxv_t)))) {
                        err = -ENOMEM;
                        goto done;
                }
                auxv->a_type = AT_NULL;
        }

        /* Allocate a stack. We put the stack immediately below the program text.
         * (in the Intel x86 ELF supplement pp 59 "example stack", that is where the
         * stack is located). I suppose we can add this "extra page for magic data" too */
        uint32_t stack_lopage = ADDR_TO_PN(proglow) - (DEFAULT_STACK_SIZE / PAGE_SIZE) - 1;
        err = vmmap_map(map, NULL, stack_lopage, (DEFAULT_STACK_SIZE / PAGE_SIZE) + 1,
                        PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_FIXED, 0, 0, NULL);
        KASSERT(0 == err);
        dbg(DBG_ELF, "Mapped stack at low addr 0x%p, size %#x\n",
            PN_TO_ADDR(stack_lopage), DEFAULT_STACK_SIZE + PAGE_SIZE);


        /* Copy out arguments onto the user stack */
        int argc, envc, auxc;
        size_t argsize = _elf32_calc_argsize(argv, envp, auxv, phtsize, &argc, &envc, &auxc);
        /* Make sure it fits on the stack */
        if (argsize >= DEFAULT_STACK_SIZE) {
                err = -E2BIG;
                goto done;
        }
        /* Copy arguments into kernel buffer */
        if (NULL == (argbuf = (char *) kmalloc(argsize))) {
                err = -ENOMEM;
                goto done;
        }
        /* Calculate where in user space we start putting the args. */
        void *arglow = (void *)((uintptr_t)(((char *) proglow) - argsize) & ~PTR_MASK);
        /* Copy everything into the user address space, modifying addresses in
         * argv, envp, and auxv to be user addresses as we go. */
        _elf32_load_args(map, arglow, argsize, argbuf, argv, envp, auxv, argc, envc, auxc, phtsize);

        dbg(DBG_ELF, "Past the point of no return. Swapping to map at 0x%p, setting brk to 0x%p\n", map, proghigh);
        /* the final threshold / What warm unspoken secrets will we learn? / Beyond
         * the point of no return ... */

        /* Give the process the new mappings. */
        vmmap_t *tempmap = curproc->p_vmmap;
        curproc->p_vmmap = map;
        map = tempmap; /* So the old maps are cleaned up */
        curproc->p_vmmap->vmm_proc = curproc;
        map->vmm_proc = NULL;

        /* Flush the process pagetables and TLB */
        pt_unmap_range(curproc->p_pagedir, USER_MEM_LOW, USER_MEM_HIGH);
        tlb_flush_all();

        /* Set the process break and starting break (immediately after the mapped-in
         * text/data/bss from the executable) */
        curproc->p_brk = proghigh;
        curproc->p_start_brk = proghigh;

        strncpy(curproc->p_comm, filename, PROC_NAME_LEN);

        /* Tell the caller the correct stack pointer and instruction
         * pointer to begin execution in user space */
        *eip = (uint32_t) entry;
        *esp = ((uint32_t) arglow) - 4; /* Space on the user stack for the (garbage) return address */
        /* Note that the return address will be fixed by the userland entry code,
         * whether in static or dynamic */

        /* And we're done */
        err = 0;

done:
        if (NULL != map) {
                vmmap_destroy(map);
        }
        if (NULL != file) {
                fput(file);
        }
        if (NULL != pht) {
                kfree(pht);
        }
        if (NULL != interpname) {
                kfree(interpname);
        }
        if (0 <= interpfd) {
                do_close(interpfd);
        }
        if (NULL != interpfile) {
                fput(interpfile);
        }
        if (NULL != interppht) {
                kfree(interppht);
        }
        if (NULL != auxv) {
                kfree(auxv);
        }
        if (NULL != argbuf) {
                kfree(argbuf);
        }
        return err;
}

static __attribute__((unused)) void elf32_init(void)
{
        binfmt_add("ELF32", _elf32_load);
}
init_func(elf32_init);
init_depends(binfmt_init);
