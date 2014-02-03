import gdb

import weenix
import weenix.kmem

class PageCommand(weenix.Command):

	def __init__(self):
		weenix.Command.__init__(self, "page",
					gdb.COMMAND_DATA,
					gdb.COMPLETE_NONE)

	def invoke(self, args, tty):
		total = 0
		print "pagesize: {0}".format(weenix.kmem.pagesize())
		
		names = list()
		blobs = list()
		pages = list()
		bytes = list()

		for order, count in weenix.kmem.freepages().iteritems():
			pcount = count * (1 << order)
			bcount = pcount * weenix.kmem.pagesize()
			names.append("freepages[{0}]:".format(order))
			blobs.append("{0} blob{1}".format(count, " " if (count == 1) else "s"))
			pages.append("{0} page{1}".format(pcount, " " if (pcount == 1) else "s"))
			bytes.append("{0} byte{1}".format(bcount, " " if (bcount == 1) else "s"))
			total += count * (1 << order)

		names.append("total:")
		blobs.append("")
		pages.append("{0} page{1}".format(total, " " if (total == 1) else "s"))
		bytes.append("{0} bytes".format(total * weenix.kmem.pagesize()))
		
		namewidth = max(map(lambda x: len(x), names))
		blobwidth = max(map(lambda x: len(x), blobs))
		pagewidth = max(map(lambda x: len(x), pages))
		bytewidth = max(map(lambda x: len(x), bytes))

		for name, blob, page, byte in zip(names, blobs, pages, bytes):
			print "{1:<{0}} {3:>{2}} {5:>{4}} {7:>{6}}".format(
				namewidth, name,
				blobwidth, blob,
				pagewidth, page,
				bytewidth, byte)

PageCommand()
