#pragma once

/* Kernel and user header (via symlink) */

/* Page protection flags.
*/
#define PROT_NONE       0x0     /* No access. */
#define PROT_READ       0x1     /* Pages can be read. */
#define PROT_WRITE      0x2     /* Pages can be written. */
#define PROT_EXEC       0x4     /* Pages can be executed. */

/* Return value for mmap() on failure.
*/
#define MAP_FAILED      ((void*)-1)

/* Mapping type - shared or private.
*/
#define MAP_SHARED      1
#define MAP_PRIVATE     2
#define MAP_TYPE        3     /* mask for above types */

/* Mapping flags.
*/
#define MAP_FIXED       4
#define MAP_ANON        8
