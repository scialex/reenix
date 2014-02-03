define pagetable
	if $argc > 0
		set $proc = proc_lookup($arg0)
		if $proc != NULL
			printf "Process %i (%s):\n", $proc->p_pid, $proc->p_comm
			set $pagedir = $proc->p_pagedir
		else
			printf "No process with PID %i exists\n", $arg0
			set $pagedir = NULL
		end
	else
		printf "Current mappings:\n"
		set $pagedir = current_pagedir
	end

	if $pagedir != NULL
		kinfo pt_mapping_info current_pagedir
	end
end
document pagetable
Without arguments displays current page table mappings in the form
"[vstart, vend) => [pstart, pend)". Takes an optional integer argument
to specify the PID of a process whose page table mappings should be
printed instead.
end
