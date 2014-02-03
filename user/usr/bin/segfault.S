/* 
 * An extremely simple test binary. It relies on your
 * process text and stack being mapped in (and hence vmmap_map
 * code working) but otherwise has even fewer dependencies than
 * hello. It should almost immediately segfault.
 */

.globl main
main:
	xor %eax, %eax;
	mov (%eax), %eax;
	ret;
