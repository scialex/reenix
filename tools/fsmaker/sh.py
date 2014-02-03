import os
import sys
import cmd
import api

import math
import stat
import errno
import shlex
import Queue
import struct
import string
import optparse
import tempfile

import curses.ascii

class OptionParser(optparse.OptionParser):

    def __init__(self, **args):
        optparse.OptionParser.__init__(self, **args)

    def exit(self, code=0, msg=""):
        sys.stderr.write(msg)

class FsmakerShell(cmd.Cmd):

    def __init__(self, simdisk):
        self._simdisk = simdisk
        self._curdir = "/"
        self.prompt = "{0} > ".format(self._curdir)
        cmd.Cmd.__init__(self)

        self._parse_superblock = OptionParser(usage="usage: %prog", prog="superblock", description="prints a summary of the superblock's contents")

        self._parse_inode = OptionParser(usage="usage: %prog <nums...>", prog="inode", description="prints a summary of the specified inode's contents")
        self._parse_inode.add_option("-i", "--indirect", action="store_true", default=False,
                                     help="if the inode is a directory or data file print the indirect block contents")
        self._parse_inode.add_option("-c", "--contents", action="store_true", default=False,
                                     help="if the inode is a data file or directory this prints the contents of the file as part of the summary")
        self._parse_inode.add_option("-l", "--list", action="store_true", default=False,
                                     help="if the inode is a directory this prints a directory listing as part of the summary")

        self._parse_block = OptionParser(usage="usage: %prog", prog="block", description="prints the data from a given block")
        self._parse_ls = OptionParser(usage="usage: %prog <dirs...>", prog="ls", description="prints a directory listing")
        self._parse_cat = OptionParser(usage="usage: %prog <files...>", prog="cat", description="prints the contents of a file")
        self._parse_cd = OptionParser(usage="usage: %prog <dir>", prog="cd", description="changes the current working directory")

        self._parse_trunc = OptionParser(usage="usage: %prog <files...>", prog="truncate", description="changes the size of a file")
        self._parse_trunc.add_option("-s", "--size", action="store", type="int", default=0,
                                     help="the new file size (defaults to %default)")

        self._parse_rm = OptionParser(usage="usage: %prog <files...>", prog="rm", description="removes a file from the disk")
        self._parse_rm.add_option("-r", "--recursive", action="store_true", default=False,
                                     help="recrsively destroy all subdirectories and files of any directory arguments")
        self._parse_rmdir = OptionParser(usage="usage: %prog <dirs...>", prog="rmdir", description="removes an empty directory from the disk")

        self._parse_touch = OptionParser(usage="usage: %prog <dirs...>", prog="touch", description="creates a plain data file")
        self._parse_mkdir = OptionParser(usage="usage: %prog <dirs...>", prog="mkdir", description="creates an empty directory")

        self._parse_getfile = OptionParser(usage="usage: %prog <source> <dest>", prog="getfile", description="gets a file from the real disk and puts it on the simdisk")
        self._parse_putfile = OptionParser(usage="usage: %prog <source> <dest>", prog="putfile", description="puts a file from the simdisk onto the real disk")

        self._parse_format = OptionParser(usage="usage: %prog -i <inode count> [-s <size>|-b <blocks>]", prog="format", description="formats the simdisk to an empty file system")
        self._parse_format.add_option("-s", "--size", action="store", type="int", default=None,
                                      help="size for the new file system in bytes, must specify either this option or -b but not both")
        self._parse_format.add_option("-b", "--blocks", action="store", type="int", default=None,
                                      help="size for the new file system in blocks, must specify either this option or -s but not both")
        self._parse_format.add_option("-i", "--inodes", action="store", type="int", default=None,
                                      help="number of inodes to put on the disk, this must be specified and be compatible with the size of the disk (there must be enough space for the inodes)")
        self._parse_format.add_option("-d", "--directory", action="store", type="str", default=None,
                                      help="initializes the disk with the contents of the specified directory")

    def open(self, path, create=False):
        if (path.startswith("/")):
            return self._simdisk.open(path, create=create)
        else:
            return self._simdisk.open(self._curdir + "/" + path, create=create)

    def filepath_completion(self, text, line, begin, end, types=api.S5_TYPES):
        res = []
        filename = text

        bline = line[:begin]
        try:
            parts = shlex.split(bline)
            if (bline.endswith('"')):
                return []
            if (bline.endswith(" ") and not bline.endswith("\\ ")):
                dirpath = self._curdir
            else:
                dirpath = parts[-1]
        except ValueError:
            dirpath = bline.rsplit('"', 1)[-1]
            if (len(dirpath.strip()) == 0):
                dirpath = self._curdir

        try:
            dirinode = self.open(dirpath)
            if (None != dirinode):
                for dirent in dirinode.getdents():
                    if (dirent.name.startswith(filename) and dirent.name != "." and dirent.name != ".."):
                        t = self._simdisk.get_inode(dirent.inode).get_type()
                        if (t in types or t == api.S5_TYPE_DIR):
                            if (t == api.S5_TYPE_DIR):
                                res.append(dirent.name + "/")
                            else:
                                res.append(dirent.name)
                return res
            else:
                return []
        except api.S5fsException as e:
            return []

    def real_filepath_completion(self, text, line, begin, end):
        res = []
        filename = text

        bline = line[:begin]
        try:
            parts = shlex.split(bline)
            if (bline.endswith('"')):
                return []
            if (bline.endswith(" ") and not bline.endswith("\\ ")):
                dirpath = os.curdir
            else:
                dirpath = parts[-1]
        except ValueError:
            dirpath = bline.rsplit('"', 1)[-1]
            if (len(dirpath.strip()) == 0):
                dirpath = os.curdir

        try:
            for dirent in os.listdir(dirpath):
                if (dirent.startswith(filename)):
                    if (os.path.isdir(os.path.join(dirpath, dirent))):
                        res.append(dirent + "/")
                    else:
                        res.append(dirent)
            return res
        except api.S5fsException as e:
            return []

    def completion_argnum(self, text, line, begin, end):
        bline = line[:begin]
        try:
            parts = [x for x in shlex.split(bline) if not x.startswith("-")]
            if (bline.endswith(" ") and not bline.endswith("\\ ")):
                return len(parts)
            else:
                return len(parts) - 1
        except ValueError:
            parts = [x for x in shlex.split(bline.rsplit('"', 1)[0]) if not x.startswith("-")]
            return len(parts)

    def get_parentdir(self, path):
        path = os.path.normpath(path).rsplit('/', 1)
        if (len(path) == 1):
            res = (self.open(self._curdir), path[0])
        else:
            if (len(path[1]) == 0):
                res = (self.open(path[0]), '.')
            else:
                res = (self.open(path[0]), path[1])
        if (res[0] == None):
            raise api.S5fsException("no such directory: {0}".format(path[0]))
        return res

    def binary_print(self, data, prefix=""):
        binary = ""
        line = ""
        for i in xrange(int(math.floor(len(data) / 2))):
            unpacked = struct.unpack("BB", data[i * 2:(i + 1) * 2])
            binary += "{0:02x}{1:02x} ".format(*unpacked)
            for d in unpacked:
                if (curses.ascii.isprint(d)):
                    line += chr(d)
                else:
                    line += '.'
            if ((i + 1) % 10 == 0):
                print("{2}{0:<50} {1}".format(binary, line, prefix))
                binary = ""
                line = ""
        if (len(data) % 2 != 0):
            d = struct.unpack("B", data[-1])[0]
            binary += "{0:02x}".format(d)
            if (curses.ascii.isprint(d)):
                line += chr(d)
            else:
                line += '.'
        if (binary != ""):
            print("{2}{0:<50} {1}".format(binary, line, prefix))

    def dirents_print(self, dirents, prefix=""):
        direntlist = []
        maxlen = 0
        maxinode = 0
        for dirent in dirents:
            maxlen = max(maxlen, len(dirent.name))
            maxinode = max(maxinode, dirent.inode)
            direntlist.append(dirent)
        if (len(direntlist) == 0):
            print ("{0}<empty directory>".format(prefix))
        else:
            for dirent in direntlist:
                try:
                    inode = self._simdisk.get_inode(dirent.inode)
                    itype = inode.get_type()
                    itypestr = inode.get_type_str(short=True)
                    isize = inode.get_size()
                except api.S5fsException as e:
                    inode = None
                    errmsg = str(e)
                if (inode == None):
                    msg = "<{0}>".format(errmsg)
                else:
                    if (itype in set([ api.S5_TYPE_DATA, api.S5_TYPE_DIR ])):
                        msg = "{0} {1} bytes".format(itypestr, isize)
                    elif (itype in set([ api.S5_TYPE_BLK, api.S5_TYPE_CHR ])):
                        msg = "{0}".format(itypestr)
                    else:
                        msg = "{0} (INVALID, free inode)".format(itypestr)
                print("{4}{0:<{1}} {2:>{3}} {5}".format(dirent.name, maxlen, dirent.inode, len(str(maxinode)), prefix, msg))

    def help_help(self):
        print("prints help information about a command")

    def do_cd(self, args):
        try:
            (options, args) = self._parse_cd.parse_args(shlex.split(args))
        except ValueError as e:
            self._parse_cd.error(str(e))
            return

        if (len(args) != 1):
            self._parse_cd.error("command takes exactly one argument")
        else:
            f = self.open(args[0])
            if (None == f):
                self._parse_cd.error("no such file or directory: {0}".format(args[0]))
            elif (api.S5_TYPE_DIR != f.get_type()):
                self._parse_cd.error("not a directory: {0}".format(args[0]))
            else:
                self._curdir = os.path.normpath(self._curdir + ("" if self._curdir.endswith("/") else "/") + args[0])
                self.prompt = "{0} > ".format(self._curdir)

    def help_cd(self):
        self._parse_cd.print_help()

    def complete_cd(self, text, line, begidx, endidx):
        if (self.completion_argnum(text, line, begidx, endidx) == 1):
            return self.filepath_completion(text, line, begidx, endidx, types=set([ api.S5_TYPE_DIR ]))
        else:
            return []

    def do_superblock(self, args):
        try:
            (options, args) = self._parse_superblock.parse_args(shlex.split(args))
        except ValueError as e:
            self._parse_superblock.error(str(e))
            return

        if (len(args) != 0):
            self._parse_superblock.error("command does not take arguments")
        else:
            print(self._simdisk.get_super_block_summary())

    def help_superblock(self):
        self._parse_superblock.print_help()

    def complete_superblock(self, text, line, begidx, endidx):
        return []

    def do_inode(self, args):
        try:
            (options, args) = self._parse_inode.parse_args(shlex.split(args))
        except ValueError as e:
            self._parse_inode.error(str(e))
            return

        if (len(args) == 0):
            self._parse_inode.error("command requires at least one argument")
        else:
            for arg in args:
                try:
                    inode = self._simdisk.get_inode(string.atoi(arg))
                except ValueError as e:
                    inode = self.open(arg)
                except api.S5fsException as e:
                    inode = self.open(arg)

                if (None == inode):
                    self._parse_inode.error(arg + " is not a valid inode number or path")
                else:
                    try:
                        print(inode.get_summary())
                        if (options.indirect and inode.get_type() in set([ api.S5_TYPE_DATA, api.S5_TYPE_DIR ]) and inode.get_indirect_blockno() != 0):
                            try:
                                iblock = self._simdisk.get_block(inode.get_indirect_blockno())
                                for i in xrange(api.S5_BLOCK_SIZE / 4):
                                    num = struct.unpack("I", iblock.read(i * 4, 4))[0]
                                    sys.stdout.write(" {0:5}".format(num))
                                    if ((i + 1) % 8 == 0):
                                        sys.stdout.write("\n")
                                if ((i + 1) % 8 != 0):
                                    sys.stdout.write("\n")
                            except api.S5fsException as e:
                                self._parse_inode.error(str(e))
                        if (options.contents and inode.get_type() in set([ api.S5_TYPE_DATA, api.S5_TYPE_DIR ])):
                            print("contents:")
                            self.binary_print(inode.read(),prefix="  ")
                        if (options.list and inode.get_type() == api.S5_TYPE_DIR):
                            print("directory listing:")
                            self.dirents_print(inode.getdents(),prefix="  ")
                    except api.S5fsException as e:
                        self._parse_inode.error(str(e))
                print("- - - - -")

    def help_inode(self):
        self._parse_inode.print_help()

    def complete_inode(self, text, line, begidx, endidx):
        res = []
        try:
            string.atoi(text)
            res += [str(x) for x in xrange(self._simdisk.get_num_inodes()) if str(x).startswith(text)]
        except ValueError:
            None
        res += self.filepath_completion(text, line, begidx, endidx)
        return res

    def do_block(self, args):
        try:
            (options, args) = self._parse_block.parse_args(shlex.split(args))
        except ValueError as e:
            self._parse_block.error(str(e))
            return

        if (len(args) == 0):
            self._parse_block.error("command requires at least one argument")
        else:
            for arg in args:
                try:
                    blockno = string.atoi(arg)
                    print("block {0}:".format(blockno))
                    block = self._simdisk.get_block(blockno)
                    self.binary_print(block.read())
                except api.S5fsException as e:
                    self._parse_block.error(str(e))
                except ValueError as e:
                    self._parse_block.error(arg + " is not a valid block number")
                print("- - - - -")

    def help_block(self):
        self._parse_block.print_help()

    def do_ls(self, args):
        try:
            (options, args) = self._parse_ls.parse_args(shlex.split(args))
        except ValueError as e:
            self._parse_ls.error(str(e))
            return

        for arg in (args if len(args) > 0 else [self._curdir]):
            try:
                f = self.open(arg)
                if (f == None):
                    self._parse_ls.error("no such file or directory: {0}".format(arg))
                elif (f.get_type() != api.S5_TYPE_DIR):
                    self._parse_ls.error("not a directory: {0}".format(arg))
                else:
                    self.dirents_print(f.getdents())
            except api.S5fsException as e:
                self._parse_ls.error(str(e))
            print("- - - - -")

    def help_ls(self):
        self._parse_ls.print_help()

    def complete_ls(self, text, line, begidx, endidx):
        return self.filepath_completion(text, line, begidx, endidx, types=set([ api.S5_TYPE_DIR ]))

    def do_cat(self, args):
        try:
            (options, args) = self._parse_cat.parse_args(shlex.split(args))
        except ValueError as e:
            self._parse_cat.error(str(e))
            return

        if (len(args) == 0):
            self._parse_cat.error("command requires at least one argument")
        else:
            for arg in args:
                try:
                    f = self.open(arg)
                    if (f == None):
                        self._parse_cat.error("no such file or directory: {0}".format(arg))
                    elif (f.get_type() != api.S5_TYPE_DATA):
                        self._parse_cat.error("not a plain data file: {0}".format(arg))
                    else:
                        print(f.read())
                except api.S5fsException as e:
                    self._parse_cat.error(str(e))
            print("- - - - -")

    def help_cat(self):
        self._parse_cat.print_help()

    def complete_cat(self, text, line, begidx, endidx):
        return self.filepath_completion(text, line, begidx, endidx, types=set([ api.S5_TYPE_DATA ]))

    def do_truncate(self, args):
        try:
            (options, args) = self._parse_trunc.parse_args(shlex.split(args))
        except ValueError as e:
            self._parse_trunc.error(str(e))
            return

        if (len(args) == 0):
            self._parse_trunc.error("command requires at least one argument")
        else:
            for arg in args:
                try:
                    f = self.open(arg)
                    if (f == None):
                        self._parse_trunc.error("no such file or directory: {0}".format(arg))
                    elif (f.get_type() != api.S5_TYPE_DATA):
                        self._parse_trunc.error("not a plain data file: {0}".format(arg))
                    else:
                        f.truncate(size=options.size)
                except api.S5fsException as e:
                    self._parse_trunc.error(str(e))

    def help_truncate(self):
        self._parse_trunc.print_help()

    def complete_truncate(self, text, line, begidx, endidx):
        return self.filepath_completion(text, line, begidx, endidx, types=set([ api.S5_TYPE_DATA ]))

    def _dir_clear(self, directory):
        for dirent in directory.getdents():
            if (dirent.name != "." and dirent.name != ".."):
                inode = self._simdisk.get_inode(dirent.inode)
                if (inode.get_type() == api.S5_TYPE_DIR):
                    self._dir_clear(inode)
                    directory.rmdir(dirent.name)
                else:
                    directory.unlink(dirent.name)

    def do_rm(self, args):
        try:
            (options, args) = self._parse_rm.parse_args(shlex.split(args))
        except ValueError as e:
            self._parse_rm.error(str(e))
            return

        if (len(args) == 0):
            self._parse_rm.error("command requires at least one argument")
        else:
            for arg in args:
                try:
                    parentdir, name = self.get_parentdir(arg)
                    inode = parentdir.open(name)
                    if (options.recursive and inode.get_type() == api.S5_TYPE_DIR):
                        self._dir_clear(inode)
                        parentdir.rmdir(name)
                    else:
                        parentdir.unlink(name)
                except api.S5fsException as e:
                    self._parse_rm.error(str(e))
             
    def help_rm(self):
        self._parse_rm.print_help()
       
    def complete_rm(self, text, line, begin, end):
        return self.filepath_completion(text, line, begin, end)

    def do_rmdir(self, args):
        try:
            (options, args) = self._parse_rmdir.parse_args(shlex.split(args))
        except ValueError as e:
            self._parse_rmdir.error(str(e))
            return

        if (len(args) == 0):
            self._parse_rmdir.error("command requires at least one argument")
        else:
            for arg in args:
                try:
                    parentdir, name = self.get_parentdir(arg)
                    parentdir.rmdir(name)
                except api.S5fsException as e:
                    self._parse_rmdir.error(str(e))
                
    def help_rmdir(self):
        self._parse_rmdir.print_help()

    def complete_rmdir(self, text, line, begin, end):
        return self.filepath_completion(text, line, begin, end, types=set([ api.S5_TYPE_DIR ]))

    def do_touch(self, args):
        try:
            (options, args) = self._parse_touch.parse_args(shlex.split(args))
        except ValueError as e:
            self._parse_touch.error(str(e))
            return

        if (len(args) == 0):
            self._parse_touch.error("command requires at least one argument")
        else:
            for arg in args:
                try:
                    parentdir, name = self.get_parentdir(arg)
                    if (parentdir.open(name) == None):
                        parentdir.create(name)
                except api.S5fsException as e:
                    self._parse_touch.error(str(e))

    def help_touch(self):
        self._parse_touch.print_help()

    def complete_touch(self, text, line, begin, end):
        return self.filepath_completion(text, line, begin, end)

    def do_mkdir(self, args):
        try:
            (options, args) = self._parse_mkdir.parse_args(shlex.split(args))
        except ValueError as e:
            self._parse_mkdir.error(str(e))
            return

        if (len(args) == 0):
            self._parse_mkdir.error("command requires at least one argument")
        else:
            for arg in args:
                try:
                    parentdir, name = self.get_parentdir(arg)
                    parentdir.mkdir(name)
                except api.S5fsException as e:
                    self._parse_mkdir.error(str(e))

    def help_mkdir(self):
        self._parse_mkdir.print_help()

    def complete_mkdir(self, text, line, begin, end):
        return self.filepath_completion(text, line, begin, end)

    def getfile(self, source, dest):
        dest.truncate()
        
        loc = 0
        data = source.read(20000)
        while (len(data) != 0):
            dest.write(loc, data)
            loc += len(data)
            data = source.read(20000)
        source.close()

    def do_getfile(self, args):
        try:
            (options, args) = self._parse_getfile.parse_args(shlex.split(args))
        except ValueError as e:
            self._parse_getfile.error(str(e))
            return

        if (len(args) != 2):
            self._parse_getfile.error("command requires source and destination paths")
        else:
            try:
                source = open(args[0], 'r')
                dest = self.open(args[1], create=True)
                self.getfile(source, dest)
            except api.S5fsException as e:
                self._parse_getfile.error(str(e))
            except IOError as e:
                self._parse_getfile.error(str(e))

    def help_getfile(self):
        self._parse_getfile.print_help()

    def complete_getfile(self, text, line, begin, end):
        argnum = self.completion_argnum(text, line, begin, end)
        if (argnum == 1):
            try:
                return self.real_filepath_completion(text, line, begin, end)
            except Exception as e:
                print str(e)
        elif (argnum == 2):
            return self.filepath_completion(text, line, begin, end, types=set([ api.S5_TYPE_DATA ]))
        else:
            return []

    def do_putfile(self, args):
        try:
            (options, args) = self._parse_putfile.parse_args(shlex.split(args))
        except ValueError as e:
            self._parse_putfile.error(str(e))
            return

        if (len(args) != 2):
            self._parse_putfile.error("command requires a source and destination")
        else:
            try:
                source = self.open(args[0])
                if (None == source):
                    self._parse_putfile.error("no such file: {0}".format(args[0]))
                dest = open(args[1], 'w+')
                
                loc = 0
                data = source.read(loc, 20000)
                while (len(data) != 0):
                    dest.write(data)
                    loc += len(data)
                    data = source.read(loc, 20000)
                dest.close()
            except api.S5fsException as e:
                self._parse_getfile.error(str(e))
            except IOError as e:
                self._parse_getfile.error(str(e))

    def help_putfile(self):
        self._parse_putfile.print_help()

    def complete_putfile(self, text, line, begin, end):
        argnum = self.completion_argnum(text, line, begin, end)
        if (argnum == 1):
            return self.filepath_completion(text, line, begin, end, types=set([ api.S5_TYPE_DATA ]))
        elif (argnum == 2):
            return self.real_filepath_completion(text, line, begin, end)
        else:
            return []

    def do_format(self, args):
        try:
            (options, args) = self._parse_format.parse_args(shlex.split(args))
        except ValueError as e:
            self._parse_format.error(str(e))
            return

        if (options.size == None and options.blocks == None):
            self._parse_format.error("must specify either -s or -b option to give size of formatted disk")
        elif (options.size != None and options.blocks != None and options.size / api.S5_BLOCK_SIZE != options.blocks):
            self._parse_format.error("specified conflicting -s and -b options, please only specify one")
        elif (options.inodes == None):
            self._parse_format.error("must specify -i option to give number of inodes to reserve on disk")
        else:
            if (options.size != None):
                size = options.size
            else:
                size = options.blocks * api.S5_BLOCK_SIZE
            self._simdisk.format(options.inodes, size)

        if (options.directory):
            q = Queue.Queue()
            q.put(".")
            while(not q.empty()):
                curr = q.get()
                real = os.path.join(options.directory, curr)
                mode = os.stat(real).st_mode
                if (stat.S_ISDIR(mode)):
                    for path in os.listdir(real):
                        q.put(os.path.join(curr, path))
                    parent, name = self.get_parentdir(os.path.join("/", curr))
                    if ("." != name):
                        parent.mkdir(name)
                else:
                    source = open(real, 'r')
                    dest = self.open(os.path.join("/", curr), create=True)
                    self.getfile(source, dest)

    def default(self, line):
        if (line.strip() == "EOF"):
            print("\n")
            return True
        else:
            print(line.split()[0] + " is not a valid command")
            return False

_parser = optparse.OptionParser(usage="usage: %prog <simdisk> [options]", add_help_option=False,
                                description="command line tool to manipulate S5FS simdisks for Weenix")
_parser.add_option("-e", "--execute", action="append", default=[], metavar="COMMAND",
                   help="Executes the given command on the simdisk. This flag can be used multiple times, the commands will be executed "
                   "in the order they appear on the command line. To see a list of commands use the -h flag. For information on a "
                   "specific command use the -c <command> flag.")
_parser.add_option("-h", "--help", action="store_true", default=False,
                   help="Prints help information and a list of fsmaker commands.")
_parser.add_option("-c", "--command", action="append", default=[],
                   help="Prints usage information about the specified command.")
_parser.add_option("-i", "--interactive", action="store_true", default=False,
                   help="Causes fsmaker to open an interactive shell after executing all commands specified with the -e option.")
(options, args) = _parser.parse_args()

try:
    if (len(args) > 1):
        _parser.error("command takes at most one positional argument, but {0} were given".format(len(args)))
    if (len(args) < 1 and not options.help and len(options.command) == 0):
        _parser.error("command requires a simdisk file path")

    if (len(args) < 1):
        fs = FsmakerShell(api.Simdisk(tempfile.TemporaryFile()))
    else:
        try:
            fs = FsmakerShell(api.Simdisk(open(args[0], 'rb+')))
        except IOError as e:
            if (e.errno == errno.ENOENT):
                fs = FsmakerShell(api.Simdisk(open(args[0], 'wb+')))
            else:
                raise e

    if (options.help):
        _parser.print_help()
        fs.onecmd("help")
    for command in options.command:
        fs.onecmd("help {0}".format(command))
    for command in options.execute:
        fs.onecmd(command)
    if (options.interactive):
        fs.cmdloop()
except KeyboardInterrupt:
    print("\n")
