#ifndef __DYNAMIC__

.globl __libc_static_entry

__libc_static_entry:
	addl $4, %esp; /* Make sure when we overwrite dummy return address
	                  with the correct one, so args will be in the right
	                  place when we call main */
	call main;
	push %eax; /* Argument to exit is return value from main */
	call exit;

#endif
