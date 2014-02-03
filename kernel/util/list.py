import gdb

import weenix
import weenix.list

class ListCommand(weenix.Command):
    """usage: list <list> [<type> <member>]
	<list>   the list_t to be printed
	<type>   the type of the values stored on the list
	<member> type's list link member used to make the list
	Prints all items on a list_t, if <type> and <member> are not given
	then the addresses of the list links are printed, otherwise the items
	are printed assuming that they have the given type."""

    def __init__(self):
        weenix.Command.__init__(self, "list",
                                gdb.COMMAND_DATA,
                                gdb.COMPLETE_SYMBOL)

    def invoke(self, arg, tty):
        args = gdb.string_to_argv(arg)
        if (len(args) == 1):
            for i, item in enumerate(weenix.list.load(args[0])):
                gdb.write("{0:>3}: {1:8}\n".format(i, item.link_addr()))
        elif (len(args) == 3):
            for i, item in enumerate(weenix.list.load(args[0], args[1], args[2])):
                gdb.write("{0:>3}: {1}\n".format(i, item.item()))
        else:
            gdb.write("{0}\n".format(self.__doc__))
            raise gdb.GdbError("invalid arguments")            

ListCommand()
