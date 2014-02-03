define kstack
	if $argc == 0
		set $kthr = curthr
	else
		set $kthr = $arg0
	end

	set $save_eip = $eip
	set $save_ebp = $ebp
	set $save_esp = $esp

	if ($kthr == curthr) && (_intr_regs != NULL)
		set $eip = _intr_regs->r_eip
		set $ebp = _intr_regs->r_ebp
		set $esp = _intr_regs->r_esp
		info stack
	else if $kthr != curthr
		set $eip = $kthr->kt_ctx.c_eip
		set $ebp = $kthr->kt_ctx.c_ebp
		set $esp = $kthr->kt_ctx.c_esp
		info stack
	else
		info stack
	end

	set $eip = $save_eip
	set $ebp = $save_ebp
	set $esp = $save_esp
end
document kstack
usage: kthread [kthread_t*]
Takes a single, optional kthread_t as an argument.
If no argument is given curthr is used instead. This
command prints the current stack of the given thread.
This includes detecting if the given thread is has
been interrupted, and looking up the interrupted
stack, rather than the interrupt stack (useful for
viewing the stack trace which caused a page-fault).
end