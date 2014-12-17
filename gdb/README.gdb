Supported Platforms
========================

You can debug Weenix with gdb by passing the '-d gdb' argument to
the Weenix script when running a supported platform. Note that gdb
is not aware of Weenix threads and processes, so you cannot use
the "thread" command (or similar commands) to change the thread
being debugged, you must allow Weenix to run until the thread you
want to debug becomes the current thread.

Extensions
========================

Weenix provides several custom gdb commands for tasks such as
listing processes and viewing page tables. These commands are all
subcommands of the "kernel" command. To see a full list of all
kernel commands run "kernel" without specifying any subcommand. To
see usage information for a command run "help kernel <command>". A
few useful commands are:

kernel dbg:
Prints or changes the current debug mode (changes what is printed
to the log).

kernel page:
Prints information on allocated pages.

kernel slab:
Prints information on allocated slabs.

kernel proc:
Prints information about Weenix processes.

These commands are programmed using fairly recent features of
gdb's python extension API. Therefore, you will need to use a
recent version of gdb to take advantage of these features
(currently development snapshot 7.3.20110620 or higher).

Creating New Extensions
========================

Extensions are written as python modules in the Weenix module with
they most pertain too (for example the "kernel proc" command is in
kernel/proc/proc.py). During the build the "kernel/gdb-commands"
target searches all Weenix modules for files with a .py extension
and adds them to a list of files to run when gdb is started. To
add your own extension write a python module with a .py extension
in any kernel module's directory.

The first stop for anyone aiming to write a python gdb extension
should be gdb manual's section on developing python
extensions. However, Weenix has a set of common extension modules
in the python/ directory (which is automatically included in gdb's
python path) whose purposes range from light wrappers around gdb's
API to very Weenix specific API's. When possible you should always
use the Weenix specific wrappers around gdb's APIs. The following
are descriptions of common uses of Weenix's modules.

== Custom Commands ==

The most likely thing you will want to do is add new commands
(e.g. 'break', 'backtrace', 'info breakpoints', etc.) to gdb. The
gdb API provides the gdb.Command class for this, however Weenix
has a weenix.Command class which you should use instead. The
weenix class automatically prefixes your command with the word
"kernel". So you will end up with commands such as 'kernel stack'
and 'kernel proc'. This allows users to type the word kernel and
tab complete to see a list of available Weenix specific commands
or run the 'kernel' command by itself to get the same
list. Otherwise weenix.Command is identical to gdb.Command.

== Stack Traces ==

gdb provices some functions and classes for accessing stack
frames, however these functions can only access the current
stack. If you want to take a snapshot of the stack for use later
you should use the weenix.stack.Frame class. This class unwinds
the stack and its str representation prints the stack identically
to the stack printed by the 'backtrace' command.

== Hooks ==

One of the coolest things gdb python extensions allow you to do is
set invisible breakpoints that call back into python code when
they are hit. This allows you to instrument virtually every part
of Weenix. In order to standardize the way this is done we provide
the 'util/gdb.h' header file which has the GDB_DEFINE_HOOK() and
GDB_CALL_HOOK() macros. The first defines a function with a
mangled name and no body which takes the name and type of the
arguments you want to pass back to python. The second is placed at
the point in the code where you want the hook to be activated and
is passed the values to give to the python callback. For example,
the following code might be used to instrument a memory allocation
algorithm:

GDB_DEFINE_HOOK(alloc,void *addr, int size)
GDB_DEFINE_HOOK(free,void *addr)

void *alloc_func(int s)
{
		void *res = /* do allocation based on size*/;
		GDB_CALL_HOOK(alloc, res, s);
		return res;
}

void free(void *a)
{
		GDB_CALL_HOOK(free, a);
		/* deallocate */
}

Note that it is okay for the hook to have the same name as the
function it is in because its name is mangled. On the python side
there is a callback function which is called when the hook is
encountered, it is passed a dictionary which contains mappings
from the hook argument names ('addr' and 'size', not 'a' and 's')
to their values.

It is possible for multiple breakpoints to be set on the same
hook, so always make sure to use any existing hooks in the code if
you can instead of adding your own (for example the 'shutdown'
hook will probably be used by many extensions).

Some hooks are hit very often, and therefore instrumenting them
can be very compuationally expensive. Therefore it is a good idea
to allow the user to turn those hooks on only when they are
interested in them. To do this use parameters (see next section).

== Parameters ==

gdb provides the gdb.Parameter class to add new parameters;
however, just as with commands, Weenix has its own
weenix.Parameter class which prefixes all parameters with 'kernel'
(e.g. 'set kernel memcheck'). Also as with commands, users can run
'show kernel' to see a list of Weenix specific parameters. A
common type of parameter is one that is set once, before Weenix
boot, and should never be changed again while Weenix runs. Weenix
provides a special class for this called weenix.Flag. These
parameters are all booleans, and cannot be changed once the 'boot'
hook has run. Also, when the 'boot' hook is hit a callback in the
flag class is called to allow it to do any necessary
initialization since the value of the parameter is finalized at
that point.
