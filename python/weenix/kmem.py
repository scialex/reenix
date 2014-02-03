import gdb
import weenix

import weenix.list

PAGE_SIZE = 4096

_uint32_type = gdb.lookup_type("uint32_t")
_uintptr_type = gdb.lookup_type("uintptr_t")
_slab_type = gdb.lookup_type("struct slab")
_allocator_type = gdb.lookup_type("struct slab_allocator")
_bufctl_type = gdb.lookup_type("struct slab_bufctl")
_void_type = gdb.lookup_type("void")

class Slab:

	def __init__(self, alloc, val):
		self._alloc = alloc
		if (val.type.code == gdb.TYPE_CODE_PTR):
			self._value = val.cast(_slab_type.pointer()).dereference()
		else:
			self._value = val.cast(_slab_type)

	def objs(self, typ=None):
		next = self._value["s_addr"]
		for i in xrange(self._alloc["sa_slab_nobjs"]):
			bufctl = (next.cast(_uintptr_type)
					  + self._alloc["sa_objsize"]).cast(_bufctl_type.pointer())
			if (bufctl.dereference()["u"]["sb_slab"] == self._value.address):
				# if redzones are in effect we need to skip them
				if (int(next.cast(_uint32_type.pointer()).dereference()) == 0xdeadbeef):
					value = (next.cast(_uint32_type.pointer()) + 1).cast(_void_type.pointer())
				else:
					value = next

				if (typ != None):
					yield value.cast(typ.pointer())
				else:
					yield value
					
			next = (next.cast(_uintptr_type)
					+ self._alloc["sa_objsize"]
					+ _bufctl_type.sizeof).cast(_void_type.pointer())

class SlabAllocator:

	def __init__(self, val):
		if (val.type.code == gdb.TYPE_CODE_PTR):
			self._value = val.cast(_allocator_type.pointer()).dereference()
		else:
			self._value = val.cast(_allocator_type)

	def name(self):
		return self._value["sa_name"].string()

	def size(self):
		return int(self._value["sa_objsize"])

	def slabs(self):
		next = self._value["sa_slabs"]
		while (next != 0):
			yield Slab(self._value, next.dereference())
			next = next.dereference()["s_next"]

	def objs(self, typ=None):
		for slab in self.slabs():
			for obj in slab.objs(typ):
				yield obj

	def __str__(self):
		res =  "name:      {0}\n".format(self.name())
		res += "slabcount: {0}\n".format(len(list(self.slabs())))
		res += "objsize:   {0}\n".format(self.size())
		res += "objcount:  {0}".format(len(list(self.objs())))
		return res

def allocators():
	next = gdb.parse_and_eval("slab_allocators")
	while (next != 0):
		yield SlabAllocator(next.dereference())
		next = next.dereference()["sa_next"]

def allocator(name):
	for alloc in allocators():
		if name == alloc.name():
			return alloc
	raise KeyError(name)

def pagesize():
	return PAGE_SIZE

def freepages():
	freepages = dict()
	for pagegroup in weenix.list.load("pagegroup_list", "struct pagegroup", "pg_link"):
		freelist = pagegroup.item()["pg_freelist"]
		for order in xrange(freelist.type.sizeof / freelist.type.target().sizeof):
			psize = (1 << order) * PAGE_SIZE
			count = len(weenix.list.load(freelist[order]))
			if (order in freepages):
				freepages[order] += count
			else:
				freepages[order] = count
	return freepages
