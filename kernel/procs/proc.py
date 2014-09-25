import gdb

import weenix
import weenix.list
import weenix.proc

class ProcCommand(weenix.Command):
	"""proc [<pids...>]
	Prints information about the listed pids. If no
	pids are listed the full process tree is printed."""

	def __init__(self):
		weenix.Command.__init__(self, "proc", gdb.COMMAND_DATA)

	def invoke(self, args, tty):
		if (len(args.strip()) == 0):
			print weenix.proc.str_proc_tree()
		else:
			for pid in args.split():
				if (pid == "curproc"):
					print weenix.proc.curproc()
				else:
					print weenix.proc.lookup(pid)

	def complete(self, line, word):
		l = map(lambda x: str(x.pid()), weenix.proc.iter())
		l.append("curproc")
		l = filter(lambda x: x.startswith(word), l)
		for used in line.split():
			l = filter(lambda x: x != used, l)
		l.sort()
		return l

ProcCommand()
