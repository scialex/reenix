import gdb

import weenix.stack

_weenix_command_prefix = "kernel"
_weenix_command_names = list()

class WeenixPrefixCommand(gdb.Command):

    def __init__(self):
        gdb.Command.__init__(self, _weenix_command_prefix, gdb.COMMAND_DATA, gdb.COMPLETE_COMMAND, True)

    def invoke(self, arg, tty):
        if (len(arg) != 0):
            print ("'{0}' is not a valid {1} command".format(arg, _weenix_command_prefix))
        print "valid {0} commands are:".format(_weenix_command_prefix)
        for command in _weenix_command_names:
            print "  {0}".format(command)
        print ("run 'help {0} <command>' for details on a particular command".format(_weenix_command_prefix))

WeenixPrefixCommand()

class Command(gdb.Command):

    def __init__(self, name, command_class, completer_class=None, prefix=False):
        if (len(name.split()) == 1):
            _weenix_command_names.append(name)
            _weenix_command_names.sort()
        name = "{0} {1}".format(_weenix_command_prefix, name)
        if (completer_class == None):
            gdb.Command.__init__(self, name, command_class, prefix=prefix)
        else:
            gdb.Command.__init__(self, name, command_class, completer_class, prefix=prefix)

_weenix_param_names = list()

class WeenixSetPrefixCommand(gdb.Command):

    def __init__(self):
        gdb.Command.__init__(self, "set " + _weenix_command_prefix, gdb.COMMAND_DATA, gdb.COMPLETE_COMMAND, True)

    def invoke(self, arg, tty):
        if (len(arg) != 0):
            print("'{0}' is not a valid {1} parameter".format(arg, _weenix_command_prefix))
        print("valid {0} parameters are:".format(_weenix_command_prefix))
        for param in _weenix_param_names:
            print("  {0}".format(param))
        print("run 'help {0} <param>' for details on a particular parameter".format(_weenix_command_prefix))

WeenixSetPrefixCommand()

class WeenixShowPrefixCommand(gdb.Command):

    def __init__(self):
        gdb.Command.__init__(self, "show " + _weenix_command_prefix, gdb.COMMAND_DATA, gdb.COMPLETE_COMMAND, True)

    def invoke(self, arg, tty):
        if (len(arg) != 0):
            print("'{0}' is not a valid {1} parameter".format(arg, _weenix_command_prefix))
        print("valid {0} parameters are:".format(_weenix_command_prefix))
        for param in _weenix_param_names:
            print("  {0}".format(param))
        print("run 'help {0} <param>' for details on a particular parameter".format(_weenix_command_prefix))

WeenixShowPrefixCommand()

class Parameter(gdb.Parameter):

    def __init__(self, name, command_class, parameter_class, enum=None):
        _weenix_param_names.append(name)
        _weenix_param_names.sort()
        name = "{0} {1}".format(_weenix_command_prefix, name)
        if (None == enum):
            gdb.Parameter.__init__(self, name, command_class, parameter_class)
        else:
            gdb.Parameter.__init__(self, name, command_class, parameter_class, enum)

class Flag(weenix.Parameter):

    def __init__(self, name, command_class, default=False):
        self._name = name
        self.value = default
        self._final = None
        weenix.Parameter.__init__(self, name, command_class, gdb.PARAM_BOOLEAN)
        weenix.Hook("boot", self.boot_callback)

    def boot_callback(self, args):
        self._final = self.value
        self.callback(self._final)

    def callback(self, value):
        None

    def get_set_string(self):
        if (None == self._final):
            return "{0} is {1}".format(self._name, "enabled" if self.value else "disabled")
        else:
            self.value = self._final
            return "{0} parameter cannot be changed once Weenix has booted".format(self._name)

    def get_show_string(self, value):
        return "{0} is {1}".format(self._name, value)

class WeenixError(gdb.GdbError):

    def __init__(self, msg):
        self.__msg = msg

    def __str__(self):
        return self.__msg

class Hook(gdb.Breakpoint):

    def __init__(self, name, callback):
        gdb.Breakpoint.__init__(self, "__py_hook_" + name, internal=True)
        self.callback = callback

    def stop(self):
        frame = weenix.stack.Frame(gdb.newest_frame())
        self.callback(frame.args())
        return False

class TypeError(WeenixError):

    def __init__(self, actual, expected, name=None):
        self.actual = actual
        self.expected = expected

        clarification = ""
        if (str(expected) != str(expected.strip_typedefs())):
            clarification = " ('{0}')".format(expected.strip_typedefs())

        if (name == None):
            name = "value"
        else:
            name = "'{0}'".format(name)

        WeenixError.__init__(
            self, "{0} has type '{1}', expected '{2}'{3}"
            .format(name, actual, expected, clarification))

def assert_type(value, expected, unqualified=True):
    if (type(value) == str):
        try:
            name = value
            actual = gdb.parse_and_eval(value).type
        except RuntimeError as err:
            raise WeenixError(str(err))
    elif (type(value) == gdb.Value):
        name = None
        actual = value.type
    elif (type(value) == gdb.Type):
        name = None
        actual = value
    else:
        raise RuntimeError("bad first argument: {0}".format(value))

    expect = expected
    if (unqualified):
        actual = actual.unqualified()
        expect = expect.unqualified()
        expected = expected.unqualified()
    actual = actual.strip_typedefs()
    expect = expect.strip_typedefs()
    if (not str(actual) == str(expect)):
        raise TypeError(actual, expected, name)

class EvalFunctionHelper(gdb.Function):

    def __init__(self):
        gdb.Function.__init__(self, self.name())
        self._count = 0
        self._vals = dict()

    def name(self):
        return "__wnx_val"

    def register(self, value):
        if (not isinstance(value, gdb.Value)):
            raise TypeError("expected gdb.Value")

        self._count += 1
        self._vals[self._count] = value
        return self._count

    def invoke(self, index):
        val = self._vals[int(index)]
        del self._vals[int(index)]
        return val

_weenix_eval_func_helper = EvalFunctionHelper()

def value_to_string(value):
    index = _weenix_eval_func_helper.register(value)
    return "${0}({1})".format(_weenix_eval_func_helper.name(), index)

def eval_func(name, *args):
    argstr = ""
    for i, arg in enumerate(args):
        if (isinstance(arg, gdb.Value)):
            arg = value_to_string(arg)
        argstr += "{0},".format(arg)
    argstr = argstr[:-1]
    return gdb.parse_and_eval("{0}({1})".format(name, argstr))
