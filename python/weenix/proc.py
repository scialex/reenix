import gdb
import weenix
import weenix.list

_proc_states = {
	1 : "RUNNING",
	2 : "EXITED"
}

class Proc:

	def __init__(self, val):
		self._val = val

	def name(self):
		return self._val["p_comm"].string()

	def pid(self):
		return int(self._val["p_pid"])

	def state(self):
		return _proc_states[int(self._val["p_state"])]

	def status(self):
		return int(self._val["p_status"])

	def parent(self):
		proc = self._val["p_pproc"]
		if (proc == 0):
			return None
		else:
			return Proc(proc.dereference())

	def children(self):
		for child in weenix.list.load(self._val["p_children"], "struct proc", "p_child_link"):
			yield Proc(child.item())

	def str_short(self):
		res = "{0:>5} ({1}) {2}".format(self.pid(), self.name(), self.state())
		if (self.state() == "EXITED"):
			res = "{0} ({1})".format(res, self.status())
		if (self == curproc()):
			res = "\033[1m{0}\033[22m".format(res)
		return res

	def __eq__(self, other):
		if (not isinstance(other, Proc)):
			return False
		else:
			return self.pid() == other.pid()

	def __ne__(self, other):
		return not self.__eq__(other)

	def __str__(self):
		res = "PID: {0} ({1})\n".format(self.pid(), self.name())
		if (self == curproc()):
			res = "\033[1m{0}\033[22m".format(res)
		if (self.state() == "EXITED"):
			res += "{0} ({1})\n".format(self.state(), self.status())
		else:
			res += "{0}\n".format(self.state())
		if (self.parent() != None):
			res += "Parent:\n"
			res += "{0}\n".format(self.parent().str_short())
		if (len(list(self.children())) > 0):
			res += "Children:\n"
			for child in self.children():
				res += "{0}\n".format(child.str_short())
		return res

def iter():
	for link in weenix.list.load("proc_list", "struct proc", "p_list_link"):
		yield Proc(link.item())

def lookup(pid):
	return Proc(weenix.eval_func("proc_lookup", pid).dereference())

def curproc():
	return Proc(gdb.parse_and_eval("curproc"))

def str_proc_tree(proc=None, indent=""):
	if (proc == None):
		proc = lookup(0)
	
	res = "{0}| {1}\n".format(indent, proc.str_short())

	for child in proc.children():
		res += str_proc_tree(child, indent+"  ")
	return res
