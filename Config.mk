# This file is meant to change some aspect of how weenix is built or run.

# Variables in this file should meet the following criteria:
# * They change some behavior in the building or running of weenix that someone
#   using weenix for educational purposes could reasonably want to change on a regular
#   basis. Note that variables like CFLAGS are not defined here because they should
#   generally not be changed.

#
# Setting any of these variables will control which parts of the source tree
# are built. To enable something set it to 1, otherwise set it to 0.
#
     DRIVERS=0
         VFS=0
        S5FS=0
          VM=0
     DYNAMIC=0
# When you finish S5FS, first enable "VM"; once this is working, then enable
# "DYNAMIC".

#
# Set the number of terminals that we should be launching.
#
        NTERMS=3

#
# Set the number of disks that we should be launching
#
        NDISKS=1

# Switches for non-required components. If you wish to try implementing
# some extra features in Weenix, there are some pre-designed features
# you can add. Turn on one of these flags and re-compile Weenix. Please
# see the Wiki for details on what is provided by changing these flags
# and what you will need to implement to complete them, of course you
# are always free to implement your own features as well. Remember, though
# these features are not "extra-credit" they are purely for academic
# interest. The most important thing is that you have a working core
# implementation, and that is what you will be graded on. If you decide
# to implement extra features please make sure your core Weenix is working
# first, and make sure to make a copy of your working Weenix before you
# go breaking it, which we promise you will happen.

        MOUNTING=0 # be able to mount multiple file systems
          GETCWD=0 # getcwd(3) syscall-like functionality
        UPREEMPT=0 # userland preemption
             MTP=0 # multiple kernel threads per process
           PIPES=0 # pipe(2) functionality
         SHADOWD=0 # shadow page cleanup

# Boolean options specified in this specified in this file that should be
# included as definitions at compile time
        COMPILE_CONFIG_BOOLS=" DRIVERS VFS S5FS VM FI DYNAMIC MOUNTING MTP SHADOWD GETCWD UPREEMPT PIPES "
# As above, but not booleans
        COMPILE_CONFIG_DEFS=" NTERMS NDISKS DBG DISK_SIZE BOCHS_INSTALL_DIR "

# Parameters for the hard disk we build (must be compatible!)
# If the FS is too big for the disk, BAD things happen!
        DISK_BLOCKS=2048 # For fsmaker
        DISK_INODES=240 # for fsmaker

# Debug message behavior. Note that this can be changed at runtime by
# modifying the dbg_modes global variable.
# All debug statements
        DBG = all
# Change to this for no debug statements
#       DBG =-all

# terminal binary to use when opening a second terminal for gdb
        GDB_TERM=xterm
        GDB_PORT=1234

# The amount of physical memory which will be available to Weenix (in megabytes)
# XXX MEMORY=32 is hardcoded in ./weenix right now -- this line here is
#     currently ignored
        MEMORY=32
