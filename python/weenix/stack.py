import gdb
import weenix

class Stack:

    def __init__(self, gdbframe=None):
        if (gdbframe == None):
            gdbframe = gdb.newest_frame()
        self._frames = []
        while (None != gdbframe):
            self._frames.append(weenix.stack.Frame(gdbframe))
            gdbframe = gdbframe.older()

    def contains(self, fname):
        for frame in self._frames:
            if (frame._func == fname):
                return True
        return False

    def __str__(self):
        res = ""
        for i, frame in enumerate(self._frames):
            res += "#{0:<2} {1}\n".format(i, str(frame))
        return res

class Frame:

    def __init__(self, gdbframe):
        self._func = "???" if gdbframe.name() == None else gdbframe.name()
        self._pc = gdbframe.find_sal().pc
        self._symtab = gdbframe.find_sal()
        self._args = {}
        if (gdbframe.function() != None):
            gdbframe.select()
            argstr = gdb.execute("info args", to_string=True)
            for line in argstr.split("\n"):
                parts = line.split("=", 1)
                if (len(parts) == 2):
                    self._args[parts[0].strip()] = parts[1].strip()

    def args(self):
        return self._args

    def __str__(self, line=0):
        if (self._symtab.symtab == None):
            res = "0x{1:>08x} ?? ()".format(line, self._pc)
        else:
            hasargs = False
            res = "0x{1:>08x} {2} (".format(line, self._pc, self._func)
            for arg in self._args.iterkeys():
                hasargs = True
                res += arg + "=" + self._args[arg] + ", "
            if (hasargs):
                res = res[:-2]
            res += ") in {0}:{1}".format(self._symtab.symtab.filename, self._symtab.line)
        return res
