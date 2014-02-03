import gdb

import weenix
import weenix.kmem

class SlabCommand(weenix.Command):

	def __init__(self):
		weenix.Command.__init__(self, "slab", gdb.COMMAND_DATA)

	def _allocators(self):
		l = list()
		for alloc in weenix.kmem.allocators():
			l.append(alloc)
		return l

	def invoke(self, args, tty):
		names = list()
		slabs = list()
		sizes = list()
		counts = list()

		names.append("")
		slabs.append("slabs")
		sizes.append("objsize")
		counts.append("allocated")

		for alloc in weenix.kmem.allocators():
			names.append(alloc.name())
			slabs.append(str(len(list(alloc.slabs()))))
			sizes.append(str(alloc.size()))
			counts.append(str(len(list(alloc.objs()))))

		namewidth = max(map(lambda x: len(x), names))
		slabwidth = max(map(lambda x: len(x), slabs))
		sizewidth = max(map(lambda x: len(x), sizes))
		countwidth = max(map(lambda x: len(x), counts))

		for name, slab, size, count in zip(names, slabs, sizes, counts):
			print "{1:<{0}} {3:>{2}} {5:>{4}} {7:>{6}}".format(
				namewidth, name,
				slabwidth, slab,
				sizewidth, size,
				countwidth, count)

	def complete(self, line, word):
		l = map(lambda x: x.name(), self._allocators())
		l = filter(lambda x: x.startswith(word), l)
		for used in line.split():
			l = filter(lambda x: x != used, l)
		l.sort()
		return l

SlabCommand()
