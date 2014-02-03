import gdb

import string

import weenix
import weenix.kmem
import weenix.stack

class SlabAllocation:
    
    def __init__(self, addr, stack, allocator, initialization):
        self.addr = addr
        self.stack = stack
        self.allocator = allocator
        self.initialization = initialization

class PageAllocation:

    def __init__(self, addr, stack, npages, slabdata, initialization):
        self.addr = addr
        self.stack = stack
        self.npages = npages
        self.slabdata = slabdata
        self.initialization = initialization

class Memcheck:

    def __init__(self):
        self._slab_alloc_count = 0
        self._slab_free_count = 0
        self._slab_invalid_free = 0
        self._slab_allocated = {}
        self._page_alloc_count = 0
        self._page_free_count = 0
        self._page_invalid_free = 0
        self._page_allocated = {}
        self._initialized = False
        weenix.Hook("slab_obj_alloc", self._slab_alloc_callback)
        weenix.Hook("slab_obj_free", self._slab_free_callback)
        weenix.Hook("page_alloc", self._page_alloc_callback)
        weenix.Hook("page_free", self._page_free_callback)
        weenix.Hook("initialized", self._initialized_callback)
        weenix.Hook("shutdown", self._shutdown_callback)

    def _slab_alloc_callback(self, args):
        addr = args["addr"]
        stack = weenix.stack.Stack(gdb.newest_frame().older())
        allocator = weenix.kmem.SlabAllocator(gdb.Value(string.atol(args["allocator"], 16)).cast(gdb.lookup_type("void").pointer()))
        self._slab_allocated[addr] = SlabAllocation(addr, stack, allocator, not self._initialized)
        if (self._initialized):
            self._slab_alloc_count += 1
        return False

    def _slab_free_callback(self, args):
        if (not args["addr"] in self._slab_allocated):
            self._slab_invalid_free += 1
            print("Invalid free of address " + args["addr"] + ":")
            print(weenix.stack.Stack(gdb.newest_frame().older()))
        else:
            if (not self._slab_allocated[args["addr"]].initialization):
                self._slab_free_count += 1
            del(self._slab_allocated[args["addr"]])
        return False

    def _page_alloc_callback(self, args):
        addr = args["addr"]
        stack = weenix.stack.Stack(gdb.newest_frame().older())
        slabdata = stack.contains("_slab_allocator_grow")
        self._page_allocated[addr] = PageAllocation(addr, stack, string.atoi(args["npages"]), slabdata, not self._initialized)
        if (self._initialized and not slabdata):
            self._page_alloc_count += 1
        return False

    def _page_free_callback(self, args):
        if (not args["addr"] in self._page_allocated):
            self._page_invalid_free += 1
            print("Invalid free of address " + args["addr"] + ":")
            print(weenix.stack.Stack(gdb.newest_frame().older()))
        elif (self._page_allocated[args["addr"]].npages != string.atoi(args["npages"])):
            self._page_invalid_free += 1
            print("Address " + args["addr"] + " allocated for " + str(self._page_allocated[args["addr"]].npages) + " pages:")
            print(self._page_allocated[args["addr"]].stack)
            print("but freed with " + args["npages"] + " pages:")
            print(weenix.stack.Stack(gdb.newest_frame().older()))
            del(self._page_allocated[args["addr"]])
        else:
            if (not self._page_allocated[args["addr"]].initialization and not self._page_allocated[args["addr"]].slabdata):
                self._page_free_count += 1
            del(self._page_allocated[args["addr"]])
        return False

    def _initialized_callback(self, args):
        self._initialized = True
        return False

    def _shutdown_callback(self, args):
        size = 0
        for alloc in self._slab_allocated.itervalues():
            if (not alloc.initialization):
                size += alloc.allocator.size()
                print("Leaked {0} bytes from \"{1}\" at {2}:".format(alloc.allocator.size(), alloc.allocator.name(), alloc.addr))
                print(alloc.stack)
        npages = 0
        for page in self._page_allocated.itervalues():
            if (not page.initialization and not page.slabdata):
                npages += page.npages
                print("Leaked {0} pages at {1}:".format(page.npages, page.addr))
                print(page.stack)
        print("{0} slab allocs, {1} frees ({2} bytes leaked)".format(self._slab_alloc_count, self._slab_free_count, size))
        print("{0} page allocs, {1} frees ({2} pages leaked)".format(self._page_alloc_count, self._page_free_count, npages))
        print("{0} invalid slab frees".format(self._slab_invalid_free))
        print("{0} invalid page frees".format(self._page_invalid_free))
        return False

class MemcheckFlag(weenix.Flag):

    def __init__(self):
        weenix.Flag.__init__(self, "memcheck", gdb.COMMAND_DATA)

    def callback(self, value):
        if (value):
            Memcheck()

MemcheckFlag()
