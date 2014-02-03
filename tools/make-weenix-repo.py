#!/usr/bin/env python
# -*- coding: utf-8 -*-
# -----------------------------------------------------------------------
# Copyright (C) 2011
# Chris Siden <chris (at) cs.brown.edu>
# Marcelo Martins <martins (at) cs.brown.edu>
#
# Permission is hereby granted, free of charge, to any person obtaining
# a copy of this software and associated documentation files (the
# "Software"), to deal in the Software without restriction, including
# without limitation the rights to use, copy, modify, merge, publish,
# distribute, sublicense, and/or sell copies of the Software, and to
# permit persons to whom the Software is furnished to do so, subject to
# the following conditions:
#
# The above copyright notice and this permission notice shall be
# included in all copies or substantial portions of the Software.
#
# THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
# EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
# MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT
# IN NO EVENT SHALL THE AUTHORS BE LIABLE FOR ANY CLAIM, DAMAGES OR
# OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE,
# ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR
# OTHER DEALINGS IN THE SOFTWARE.
# -----------------------------------------------------------------------

from repotools import cutCode, dbg_msg, dbgPrint, setVerbose, srcCutdir
import argparse
import os
import os.path
import re
import stat
import sys
import shutil
import subprocess
import types

## ==============================================
## binKeepdir
## ==============================================
def binKeepdir(root_dir, kernel_dir, module_dirs, module_names, keep_exts=[], keep_files=[]):
    """Performs binary keeping of directory list from root.
       Keep object files based on extensions provided in keep_exts
       from dir_list
    """
    assert type(root_dir) == types.StringType
    assert type(kernel_dir) == types.StringType
    assert type(module_dirs) == types.ListType or type(module_dirs) == types.TupleType
    assert type(module_names) == types.ListType or type(module_names) == types.TupleType
    assert type(keep_exts) == types.ListType or type(keep_exts) == types.TupleType
    assert type(keep_files) == types.ListType or type(keep_exts) == types.TupleType
    # Assumes we have compiled the tree before working on it.
    # Compiling the tree is necessary if we want to submit the .a
    # libraries to non-lab students.

    # Keeps object files from subdir of dir_list based on file
    # extension pattern. Non-lab students must only have access to
    # libraries, but not be permitted to view source code. This prevents
    # lab students from retrieving the non-lab repo and peeking at the
    # library source code they should be implementing.

    # Call external script to build libraries. Assumes the script is correct.
    for i, module_dir in enumerate(module_dirs):
        subprocess.call([os.path.join(root_dir, "tools", "archive_module.sh"), kernel_dir, module_dir, module_names[i]])

        path = os.path.join(kernel_dir, module_dir)

        # Let's start by ignoring all files. We will add exceptions
        # later. This guarantees that no source code is kept in our new
        # git repo

        # WARNING: relies on git. If not using git, we need to remove
        # the file
        with open(os.path.join(path, ".gitignore"), "a") as fin:
            fin.write("*\n")

        # Now let's include our wanted files based on their extensions.
        # We can use this to keep the static libraries.
        # Note that we force its inclusion with git add since the files
        # might be ignore by .gitignore the root dir

        filenames = os.listdir(path)
        for file in filenames:
            basename, ext = os.path.splitext(file)
            if ext in keep_exts:
                filename = os.path.join(path, file)
                dbgPrint("Keeping file %s\n" % filename, dbg_msg.DEBUG)
                subprocess.call(["git", "add", "-f", filename])

        # Finally, let's keep files based on their names
        for file in keep_files:
            if file in filenames:
                dbgPrint("\tKeeping %s\n" % file, dbg_msg.DEBUG)
                subprocess.call(["git", "add", "-f", os.path.join(path, file)])

## ==============================================
## findRoot
## ==============================================
def findRoot(base, max_depth=5):
    """Find root of to-be-modified source tree"""
    assert type(base) == types.StringType
    assert os.path.isdir(base)

    depth = 0
    root_dir = ""

    # Keep going up the tree until we find the root
    while depth < max_depth:
        filenames = os.listdir(base)

        # Root for Weenix repo contains weenix and fsmaker files
        if ("weenix" in filenames) and ("fsmaker" in filenames):
            root_dir = os.path.abspath(base)
            break
        base = os.path.join(base, "..")
        depth += 1

    return root_dir

## ==============================================
## listModules
## ==============================================
def listModules(prefix="", modules=[]):
    assert type(prefix) == types.StringType
    assert type(modules) == types.ListType or type(modules) == types.TupleType

    sys.stdout.write(prefix)
    print " ".join(m for m in modules)

remove_files = [ "tools/make-weenix-repo.help", "kernel/test/pipes.c", "user/usr/bin/tests/pipetest.c" ]
keep_files = [ "Submodules" ]
executable_files = [ "fsmaker","weenix","tools/make-weenix-repo.py" ]

remove_extensions = [ ".bin",".dbg",".files",".gdbcomm",".img",".o",".out" ]
src_extensions = [ ".h",".c",".py" ]
lib_extensions = [ ".a",".gdb" ]

kernel_dir_prefix = "kernel"

# Map between module names, source-code location and submodule names
modules = {
    "DRIVERS": { "dirs": { "src": [ "drivers/disk","drivers/tty","drivers" ], "lib": [ "drivers/disk","drivers/tty","drivers" ] }, \
                 "names": [ "disk","tty","drivers" ] },
    "PROCS": { "dirs": { "src": [ "main","proc" ], "lib": [ "proc" ] }, \
               "names": [ "proc" ] },
    "S5FS": { "dirs": { "src": [ "fs/s5fs","mm" ], "lib": [ "fs/s5fs" ] }, \
              "names": [ "s5fs" ] },
    "VFS":  { "dirs": { "src": [ "fs" ], "lib": [ "fs" ] }, \
              "names": [ "fs" ] },
    "VM":   { "dirs": { "src": [ "api","drivers","fs/s5fs","proc","vm" ], "lib": [ "vm" ] }, \
              "names": [ "vm" ] },
}

# Map between extra features and their source-code location
extras = {
    "GETCWD":   { "dirs": [ "fs" ] }, \
    "MOUNTING": { "dirs": [ "fs" ] }, \
    "MTP":      { "dirs": [ "proc" ] }, \
    "UPREEMPT": { "dirs": [ "util" ] }, \
    "PIPES":    { "dirs": [ "fs" ] }
}

## ==============================================
## main
## ==============================================
def main():
    global remove_files
    global executable_files
    global verbose_mode

    global remove_extensions
    global src_extensions
    global lib_extensions
    global modules
    global extras

    list_modules = False
    list_extra = False
    binary_keep = False
    src_strip = False
    extra_strip = False

    aparser = argparse.ArgumentParser(description="Weenix repository generator from support code")
    aparser.add_argument("--list-modules", action="store_true", help="list available Weenix modules")
    aparser.add_argument("--list-extra", action="store_true", help="list available extra Weenix features")
    aparser.add_argument("--verbose", action="store_true", help="enable verbose output mode")
    aparser.add_argument("--binary", type=str, metavar="module_list", help="enable binary keeping for Weenix modules")
    aparser.add_argument("--cutsource", type=str, metavar="module_list", help="enable source stripping for Weenix modules")
    aparser.add_argument("--cutextra", type=str, metavar="extra_list", help="enable source stripping for extra Weenix features")

    if len(sys.argv) < 2:
        aparser.print_help()
        sys.exit(1)

    args = vars(aparser.parse_args())

    if args["verbose"]:
        verbose_mode = True
        setVerbose(True)
    if args["list_modules"]: list_modules = True
    if args["list_extra"]: list_extra = True
    if args["binary"]:
        binary_keep = True
        if args["binary"] == "all":
            binary_list = set(modules.keys())
        else:
            binary_list = set(args["binary"].split(","))
    if args["cutsource"]:
        src_strip = True
        if args["cutsource"] == "all":
            src_list = set(modules.keys())
        else:
            src_list = set(args["cutsource"].split(","))
    if args["cutextra"]:
        extra_strip = True
        if args["cutextra"] == "all":
            extra_list = set(extras.keys())
        else:
            extra_list = set(args["cutextra"].split(","))

    root_dir = findRoot(".")
    os.umask(0007)
    os.chdir(root_dir)

    # List modules
    if list_modules or list_extra:
        if binary_keep or src_strip:
            sys.stderr.write("ERROR: Listing, binary-keeping, and source-stripping are mutually-exclusive operations.\n")
            sys.exit(1)
        if list_modules:
            listModules("Modules: ", modules.keys())
        if list_extra:
            listModules("Extras: ", extras.keys())
        sys.exit()
    elif not binary_keep and not src_strip:
        sys.stderr.write("ERROR: Please define operation: module-listing, binary-keeping or source-stripping.\n")
        sys.exit(1)

    dbgPrint("Generating student tree...\n")

    # Make sure we are only working with supported modules
    if src_strip:
        diff = src_list - set(modules.keys())
        if len(diff) > 0:
            sys.stderr.write("ERROR: The following modules are not supported: %s\n" % str(diff))
            sys.exit(1)

    # Make sure we are only working with supported extras
    if extra_strip:
        diff = extra_list - set(extras.keys())
        if len(diff) > 0:
            sys.stderr.write("ERROR: The following extras are not supported: %s\n" % str(diff))
            sys.exit(1)

    # Make sure we are only working with supported modules
    if binary_keep:
        diff = binary_list - set(modules.keys())
        if len(diff) > 0:
            sys.stderr.write("ERROR: The following modules are not supported: %s\n" % str(diff))
            sys.exit(1)

    # Finally make sure we are not applying multiple operations to the
    # same module
    if (binary_keep and src_strip) and len(set(binary_list).intersection(src_list)) > 0:
        sys.stderr.write("ERROR: We can't apply multiple operations to the same modules\n")
        sys.exit(1)

    # Compile code first to make sure that there are no errors. Also
    # needed for keeping binaries
    dbgPrint("Compiling code...\n")

    # If you don't use redo, substitute the following clode block
    subprocess.call(["make", "-j", "8"])

    dbgPrint("Removing .git if it exists...\n")
    try:
        shutil.rmtree(os.path.join(root_dir, ".git"))
    except OSError:
        pass

    dbgPrint("Removing staff-only files...\n")
    for rfile in remove_files:
        path = os.path.join(root_dir, rfile)
        if (os.path.isdir(path)):
            dbgPrint("\tRemoving %s\n" % path, dbg_msg.DEBUG)
            shutil.rmtree(path)
        else:
            dbgPrint("\tRemoving %s\n" % path, dbg_msg.DEBUG)
            os.remove(path)
 
    dbgPrint("Creating student weenix git repository...\n")
    subprocess.call(["git", "init"])

    if src_strip:
        diff = set(modules.keys()) - src_list

        # Remove support code and produce stencil for students
        dbgPrint("Generating stencil code for modules %s...\n" % args["cutsource"])
        module_dirs = []
        for m in src_list:
            if m in modules.keys():
                module_dirs.extend([os.path.join(kernel_dir_prefix, m) for m in modules[m]["dirs"]["src"]])
        srcCutdir(root_dir, module_dirs, diff.union(extras.keys()))

        if len(diff) > 0:
            dbgPrint("The following modules were left intact: %s\n" % (", ".join(m for m in diff)), dbg_msg.WARNING)

    if extra_strip:
        diff = set(extras.keys()) - extra_list

        # Remove support code and produce stencil for students
        dbgPrint("Generating stencil code for extras %s...\n" % args["cutextra"])
        extras_dirs = []
        for m in extra_list:
            if m in extras.keys(): extras_dirs.extend([os.path.join(kernel_dir_prefix, e) for e in extras[m]["dirs"]])
        srcCutdir(root_dir, extras_dirs, diff.union(modules.keys()))

        if len(diff) > 0:
            dbgPrint("The following extras were left intact: %s\n" % (", ".join(m for m in diff)), dbg_msg.WARNING)

    if binary_keep:
        # Do not include library source code for non-lab students
        dbgPrint("Ignoring source from modules %s...\n" % args["binary"])
        module_dirs = []
        module_names = []
        for m in binary_list:
            if m in modules.keys():
                module_dirs.extend(modules[m]["dirs"]["lib"])
                module_names.extend(modules[m]["names"])

        binKeepdir(root_dir, os.path.join(root_dir, kernel_dir_prefix), module_dirs, module_names, lib_extensions)

        diff = set(modules.keys()) - binary_list
        if len(diff) > 0:
            dbgPrint("The following modules were left intact: %s\n" % (", ".join(m for m in diff)), dbg_msg.WARNING)

    dbgPrint("Setting correct file/dir permissions...\n")
    os.chmod(root_dir, 0770)
    for root, dirs, files, in os.walk(root_dir):
        for file in files:
            dbgPrint("\tSetting %s permissions to 0664\n" %(os.path.join(root, file)), dbg_msg.DEBUG)
            os.chmod(os.path.join(root, file), 0664)
        for dir in dirs:
            dbgPrint("\tSetting %s permissions to 0775\n" %(os.path.join(root, dir)), dbg_msg.DEBUG)
            os.chmod(os.path.join(root, dir), 0775 | stat.S_ISGID)

    dbgPrint("Setting executable files...\n")
    for efile in executable_files:
        path = os.path.join(root_dir, efile)
        dbgPrint("\tMaking %s executable\n" % path, dbg_msg.DEBUG)
        os.chmod(path, 0775)

    subprocess.call(["git", "add", "."])
    subprocess.call(["git", "commit", "-m", "Created student weenix repository"])
    dbgPrint("DONE.\n")

if __name__ == "__main__":
    sys.exit(main())
