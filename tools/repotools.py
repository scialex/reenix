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

import argparse
import os
import os.path
import re
import sys
import types

# Project code cutting regex
# /* PROJECT_NAME [BLANK] { */
project_regex = re.compile(r"(?P<whitespace>\s*)/\*\s*(?P<project>\w*)\s*(?P<blank>BLANK)?\s*(?P<brace>\{\{\{|\}\}\})\s*\*/")

# Function definition regex (extremely terrible!)
func_regex = re.compile(r"(\w+)\(")

verbose_mode = False

class Enum(set):
    """Implements enum in a Pythonic way (sigh)"""
    def __getattr__(self, name):
        if name in self:
            return name
        raise AttributeError

dbg_msg = Enum(["ERROR", "WARNING", "NORMAL", "DEBUG", "UNKNOWN"])

class CutcodeException(Exception):
    def __init__(self, msg):
        self.msg = msg

## ==============================================
## setVerbose
## ==============================================
def setVerbose(verbose=True):
    global verbose_mode
    verbose_mode = verbose

## ==============================================
## dbgPrint
## ==============================================
def dbgPrint(msg, msg_type=dbg_msg.NORMAL):
    global dbg_msg
    global verbose_mode

    assert type(msg) == types.StringType

    if msg_type == dbg_msg.WARNING:
        msg = "WARNING: %s" % msg
    elif msg_type == dbg_msg.ERROR:
        msg = "ERROR: %s" % msg
    elif msg_type == dbg_msg.DEBUG:
        msg = "DEBUG: %s" % msg

    if verbose_mode or msg_type != dbg_msg.DEBUG:
        sys.stdout.write(msg)

## ==============================================
## cutCode
## ==============================================
def cutCode(file, keep):
    """ Reads C code from file, iterating over the lines and removing code
        between PROJECT {{{ and PROJECT }}}, any project names specified in
        the keep list are not cut. Source stripping occurs on all tags,
		indiscriminately, unless if tag belong to keep.
    """
    projects_to_keep = {}
    for project_name in keep:
        projects_to_keep[project_name] = 1

    in_projects = {}
    cur_func = "***none***"
    line_num = 0
    lines_skipped = 0
    skip_line_start = 0
    skip_line_end = 0

    while (True):
        line = file.readline()
        if (not line):
            break

        match = func_regex.match(line)
        if (match):
            # Found a function definition
            pieces = match.group(1).rpartition(" ")
            cur_func = pieces[2]

        match = project_regex.match(line)
        if (match):
            # Found a project cutting delimiter
            m = match.groupdict()
            project_name = m["project"]
            if project_name not in projects_to_keep:
                is_blank = (None != m["blank"])
                brace = m["brace"]
                whitespace = m["whitespace"]
                if (brace == "{{{"):
                    if project_name in in_projects:
                        in_projects[project_name] += 1
                        skip_line_start = line_num
                    else:
                        in_projects[project_name] = 1
                elif (brace == "}}}"):
                    if project_name in in_projects:
                        in_projects[project_name] -= 1
                        skip_line_end = line_num
                        lines_skipped += skip_line_end - skip_line_start
                        dbgPrint("\t\tStripping lines %s to %s of %s. New line number should be %s\n" % (skip_line_start, skip_line_end, file.name, (line_num - lines_skipped)), dbg_msg.DEBUG)
                    else:
                        in_projects[project_name] = -1

                if (in_projects[project_name] < 0 or in_projects[project_name] > 1):
                    raise CutcodeException("Mismatched code cut delimiter\n")

                if (in_projects[project_name] == 1 and not is_blank):
                    yield whitespace + "NOT_YET_IMPLEMENTED(\"" + project_name + ": " + cur_func + "\");\n"
            else:
                yield line
        else:
            # Only print line if we are not currently in a project
            should_print = True
            for proj in in_projects:
                if in_projects[proj] > 0:
                    should_print = False
            if should_print:
                yield line

            line_num += 1

## ==============================================
## cleanDir
## ==============================================
def cleanDir(root_dir, remove_exts):
    """New repo should ignore object files (.o) from all subdirs based
	on file extension pattern (probably there is a better way to do
	this) applied to .gitignore. All students must compile their own
	code.
    """
    assert type(root_dir) == types.StringType
    assert type(remove_exts) == types.ListType or type(remove_exts) == types.TupleType

    # Our new repo should not contain certain files based on their
    # extension. We do this by applying patterns to .gitignore
    # WARNING: relies on git. If not using git, we need to remove
    # the file
    for ext in remove_exts:
        dbgPrint("Ignoring all files ended with %s\n" % ext, dbg_msg.DEBUG)
        with open(os.path(root_dir, ".gitignore"), "a") as fin:
            fin.write("*" + ext + "\n")

## ==============================================
## srcCutdir
## ==============================================
def srcCutdir(root_dir, dir_list, keep_module_names=[]):
    """Performs source-code stripping of C and header files"""
    assert type(root_dir) == types.StringType
    assert type(dir_list) == types.ListType or type(dir_list) == types.TupleType

    for directory in dir_list:
        path = os.path.join(root_dir, directory)
        if not os.path.isdir(path):
            dbgPrint("\t%s is not a valid directory\n" % path, dbg_msg.WARNING)
            continue

        for root, dirs, files in os.walk(path):
            for file in files:
                if not (file.endswith(".c") or file.endswith(".h")):
                    continue
                filepath = os.path.realpath(os.path.join(root, file))

                dbgPrint("\tStripping source file %s...\n" % filepath, dbg_msg.DEBUG)
                tmp = os.tmpfile()
                f = open(filepath)
                tmp.writelines(cutCode(f, keep_module_names))
                f = open(filepath, "w")
                tmp.seek(0)
                f.writelines(tmp)
                tmp.close()
                f.close()

## ==============================================
## main
## ==============================================
def main():
    global verbose_mode

    src_strip = False

    aparser = argparse.ArgumentParser(description="support code cutter")
    aparser.add_argument("--verbose", action="store_true", help="enable verbose output mode")
    aparser.add_argument("--cutsource", action="store_true", help="enable source stripping")

    if len(sys.argv) < 2:
        aparser.print_help()
        sys.exit(1)

    args = vars(aparser.parse_args())

    if args["verbose"]: verbose_mode = True
    if args["cutsource"]: src_strip = True

    if src_strip:
        # Remove support code and produce stencil for students
        dbgPrint("Generating stencil code for %s...\n" % args["cutsource"])
        srcCutdir(".", ["."])
        dbgPrint("DONE.\n")


if __name__ == "__main__":
    sys.exit(main())
