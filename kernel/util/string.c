#include "ctype.h"
#include "errno.h"

int memcmp(const void *cs, const void *ct, size_t count)
{
        int ret;
        /* Compare bytes at %esi and %edi up to %ecx bytes OR until
         * the bytes are not equal */
        /* If not equal, set zf = 0 and stop */
        /* Find out zf and sf and use them to return 0,1, or -1 */
        __asm__ volatile(
                "xor %%eax, %%eax\n\t"  /* Zero output */
                "cld\n\t"               /* Make sure direction is forwards */
                "repe\n\t"
                "cmpsb\n\t"
                "setnz %%al\n\t"        /* If it is not zero, put 1 in low part */
                "sets %%ah"             /* If sign set (means 2nd arg larger),
                                         * put 1 in high part */
                : "=a"(ret)
                : "S"(cs), "D"(ct), "c"(count)
                : "cc"                  /* Overwrite flags */
        );
        return ((ret & 1) ? ((ret >> 8) ? -1 : 1) : 0);
}

void *memcpy(void *dest, const void *src, size_t count)
{
        /* Move %ecx bytes from %esi to %edi */
        __asm__ volatile(
                "cld\n\t" /* Make sure direction is forwards */
                "rep\n\t"
                "movsb"
                : /* No output */
                : "S"(src), "D"(dest), "c"(count)
                : "cc" /* We overwrite condition codes - i.e., flags */
        );
        return dest;
}

void *memset(void *s, int c, size_t count)
{
        /* Fill %ecx bytes at %edi with %eax (actually %al) */
        __asm__ volatile(
                "cld\n\t" /* Make sure direction is forwards */
                "rep\n\t"
                "stosb"
                : /* No output */
                : "a"(c), "D"(s), "c"(count)
                : "cc" /* Overwrite flags */
        );
        return s;
}

int strncmp(const char *cs, const char *ct, size_t count)
{
        register signed char __res = 0;

        while (count) {
                if ((__res = *cs - *ct++) != 0 || !*cs++)
                        break;
                count--;
        }

        return __res;
}

int strcmp(const char *cs, const char *ct)
{
        register signed char __res;

        while (1) {
                if ((__res = *cs - *ct++) != 0 || !*cs++)
                        break;
        }

        return __res;
}

char *strcpy(char *dest, const char *src)
{
        char *tmp = dest;

        while ((*dest++ = *src++) != '\0')
                /* nothing */;
        return tmp;
}

char *strncpy(char *dest, const char *src, size_t count)
{
        char *tmp = dest;

        while (count-- && (*dest++ = *src++) != '\0')
                /* nothing */;

        return tmp;
}

size_t strnlen(const char *s, size_t count)
{
        const char *sc;

        for (sc = s; count-- && *sc != '\0'; ++sc)
                /* nothing */;
        return sc - s;
}


char *strcat(char *dest, const char *src)
{
        char *tmp = dest;

        while (*dest)
                dest++;

        while ((*dest++ = *src++) != '\0');

        return tmp;
}

size_t strlen(const char *s)
{
        const char *sc;

        for (sc = s; *sc != '\0'; ++sc)
                /* nothing */;
        return sc - s;
}

char *strchr(const char *s, int c)
{
        for (; *s != (char) c; ++s)
                if (*s == '\0')
                        return NULL;
        return (char *)s;
}

char *strrchr(const char *s, int c)
{
        char *r = NULL;
        for (; *s; ++s)
                if (*s == (char)c)
                        r = (char *)s;
        return r;
}

char *strstr(const char *s1, const char *s2)
{
        int l1, l2;

        l2 = strlen(s2);
        if (!l2)
                return (char *) s1;
        l1 = strlen(s1);
        while (l1 >= l2) {
                l1--;
                if (!memcmp(s1, s2, l2))
                        return (char *) s1;
                s1++;
        }
        return NULL;
}

/*
 * The following three functions were ripped out of OpenSolaris. Legally, they
 * might have to be in a separate file. Leaving it here out of laziness.
 * Got this from /onnv-gate/usr/src/common/uti/string.c.
 */

char *
strpbrk(const char *string, const char *brkset)
{
        const char *p;

        do {
                for (p = brkset; *p != '\0' && *p != *string; ++p)
                        ;
                if (*p != '\0')
                        return (char *)string;
        } while (*string++);

        return NULL;
}

size_t
strspn(const char *string, const char *charset)
{
        const char *p, *q;

        for (q = string; *q != '\0'; ++q) {
                for (p = charset; *p != '\0' && *p != *q; ++p)
                        ;
                if (*p == '\0')
                        break;
        }

        return q - string;
}

char *
strtok(char *string, const char *sepset)
{
        char            *p, *q, *r;
        static char     *savept;

        /*
         * Set `p' to our current location in the string.
         */
        p = (string == NULL) ? savept : string;
        if (p == NULL)
                return NULL;

        /*
         * Skip leading separators; bail if no tokens remain.
         */
        q = p + strspn(p, sepset);
        if (*q == '\0')
                return NULL;

        /*
         * Mark the end of the token and set `savept' for the next iteration.
         */
        if ((r = strpbrk(q, sepset)) == NULL)
                savept = NULL;
        else {
                *r = '\0';
                savept = ++r;
        }

        return q;
}

/* created with the help of:
 * perl -p -e 's/#define\s+(\w+)\s+\d+\s+\/\* ([^\t\*]+)\s*\*\/\s*$/case $1: return "$2";\n/' < /usr/include/sys/errno.h
 */
char *strerror(int errnum)
{
        switch (errnum) {
                case EPERM: return "Not super-user";
                case ENOENT: return "No such file or directory";
                case ESRCH: return "No such process";
                case EINTR: return "interrupted system call";
                case EIO: return "I/O error";
                case ENXIO: return "No such device or address";
                case E2BIG: return "Arg list too long";
                case ENOEXEC: return "Exec format error";
                case EBADF: return "Bad file number";
                case ECHILD: return "No children";
                case EAGAIN: return "Resource temporarily unavailable";
                case ENOMEM: return "Not enough core";
                case EACCES: return "Permission denied";
                case EFAULT: return "Bad address";
                case ENOTBLK: return "Block device required";
                case EBUSY: return "Mount device busy";
                case EEXIST: return "File exists";
                case EXDEV: return "Cross-device link";
                case ENODEV: return "No such device";
                case ENOTDIR: return "Not a directory";
                case EISDIR: return "Is a directory";
                case EINVAL: return "Invalid argument";
                case ENFILE: return "File table overflow";
                case EMFILE: return "Too many open files";
                case ENOTTY: return "Inappropriate ioctl for device";
                case ETXTBSY: return "Text file busy";
                case EFBIG: return "File too large";
                case ENOSPC: return "No space left on device";
                case ESPIPE: return "Illegal seek";
                case EROFS: return "Read only file system";
                case EMLINK: return "Too many links";
                case EPIPE: return "Broken pipe";
                case EDOM: return "Math arg out of domain of func";
                case ERANGE: return "Math result not representable";
                case ENOMSG: return "No message of desired type";
                case EIDRM: return "Identifier removed";
                case ECHRNG: return "Channel number out of range";
                case EL2NSYNC: return "Level 2 not synchronized";
                case EL3HLT: return "Level 3 halted";
                case EL3RST: return "Level 3 reset";
                case ELNRNG: return "Link number out of range";
                case EUNATCH: return "Protocol driver not attached";
                case ENOCSI: return "No CSI structure available";
                case EL2HLT: return "Level 2 halted";
                case EDEADLK: return "Deadlock condition.";
                case ENOLCK: return "No record locks available.";
                case ECANCELED: return "Operation canceled";
                case ENOTSUP: return "Operation not supported";
                case EDQUOT: return "Disc quota exceeded";
                case EBADE: return "invalid exchange";
                case EBADR: return "invalid request descriptor";
                case EXFULL: return "exchange full";
                case ENOANO: return "no anode";
                case EBADRQC: return "invalid request code";
                case EBADSLT: return "invalid slot";
                case EBFONT: return "bad font file fmt";
                case ENOSTR: return "Device not a stream";
                case ENODATA: return "no data (for no delay io)";
                case ETIME: return "timer expired";
                case ENOSR: return "out of streams resources";
                case ENONET: return "Machine is not on the network";
                case ENOPKG: return "Package not installed";
                case EREMOTE: return "The object is remote";
                case ENOLINK: return "the link has been severed";
                case EADV: return "advertise error";
                case ESRMNT: return "srmount error";
                case ECOMM: return "Communication error on send";
                case EPROTO: return "Protocol error";
                case EMULTIHOP: return "multihop attempted";
                case EBADMSG: return "trying to read unreadable message";
                case ENAMETOOLONG: return "path name is too long";
                case EOVERFLOW: return "value too large to be stored in data type";
                case ENOTUNIQ: return "given log. name not unique";
                case EBADFD: return "f.d. invalid for this operation";
                case EREMCHG: return "Remote address changed";
                case ELIBACC: return "Can't access a needed shared lib.";
                case ELIBBAD: return "Accessing a corrupted shared lib.";
                case ELIBSCN: return ".lib section in a.out corrupted.";
                case ELIBMAX: return "Attempting to link in too many libs.";
                case ELIBEXEC: return "Attempting to exec a shared library.";
                case EILSEQ: return "Illegal byte sequence.";
                case ENOSYS: return "Unsupported file system operation";
                case ELOOP: return "Symbolic link loop";
                case ERESTART: return "Restartable system call";
                case ESTRPIPE: return "if pipe/FIFO, don't sleep in stream head";
                case ENOTEMPTY: return "directory not empty";
                case EUSERS: return "Too many users (for UFS)";
                case ENOTSOCK: return "Socket operation on non-socket";
                case EDESTADDRREQ: return "Destination address required";
                case EMSGSIZE: return "Message too long";
                case EPROTOTYPE: return "Protocol wrong type for socket";
                case ENOPROTOOPT: return "Protocol not available";
                case EPROTONOSUPPORT: return "Protocol not supported";
                case ESOCKTNOSUPPORT: return "Socket type not supported";
                case EPFNOSUPPORT: return "Protocol family not supported";
                case EAFNOSUPPORT: return "Address family not supported by protocol family";
                case EADDRINUSE: return "Address already in use";
                case EADDRNOTAVAIL: return "Can't assign requested address";
                case ENETDOWN: return "Network is down";
                case ENETUNREACH: return "Network is unreachable";
                case ENETRESET: return "Network dropped connection because of reset";
                case ECONNABORTED: return "Software caused connection abort";
                case ECONNRESET: return "Connection reset by peer";
                case ENOBUFS: return "No buffer space available";
                case EISCONN: return "Socket is already connected";
                case ENOTCONN: return "Socket is not connected";
                case ESHUTDOWN: return "Can't send after socket shutdown";
                case ETOOMANYREFS: return "Too many references: can't splice";
                case ETIMEDOUT: return "Connection timed out";
                case ECONNREFUSED: return "Connection refused";
                case EHOSTDOWN: return "Host is down";
                case EHOSTUNREACH: return "No route to host";
                case EALREADY: return "operation already in progress";
                case EINPROGRESS: return "operation now in progress";
                case ESTALE: return "Stale NFS file handle";
                default: return 0;
        }
}
