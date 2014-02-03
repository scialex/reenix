define vmmap
	if $argc > 0
		set $proc = proc_lookup($arg0)
		if $proc != NULL
			printf "Process %i (%s):\n", $proc->p_pid, $proc->p_comm
			set $vmmap = $proc->p_vmmap
		else
			printf "No process with PID %i exists\n", $arg0
			set $vmmap = NULL
		end
	else
		printf "Current process %i (%s):\n", curproc->p_pid, curproc->p_comm
		set $vmmap = curproc->p_vmmap
	end

	if $vmmap != NULL
		kinfo vmmap_mapping_info $vmmap
	end
end
document pagetable
Without arguments displays current mappings. Takes an optional integer
argument to specify the PID of a process whose mappings should be
printed instead.
end
