SHELL := /bin/sh

# XXX why is this here?
CFLAGS :=

include ../Global.mk

DISK_IMAGE := disk0.img
STAGING_DIR := .staging

.PHONY: all clean

all: $(DISK_IMAGE)

########
# compile step
########

# CFLAGS_LIB: separate set of CFLAGS used for shared libraries (which, for
# example, need -fpic to build, but other executables can't be built with
# -fpic)
CFLAGS_LIB := $(CFLAGS) -fpic

# XXX is this being defined multiple times?
ASFLAGS := -D__ASSEMBLY__

# - all source files are compiled to an object file of the same name
#   (no .c file and .S file shares the same prefix, so we have no conflicts)
# - for each .o, make picks the appropriate rule to generate it (only one of
#   the two rules has dependencies that exist)
# - pass -fpic only to shared libraries

lib/%.o: lib/%.c
	@ echo "  Compiling \"user/$<\"..."
	@ $(CC) -c $< -o $@ $(CFLAGS_LIB)

%.o: %.c
	@ echo "  Compiling \"user/$<\"..."
	@ $(CC) -c $< -o $@ $(CFLAGS)

lib/%.o: lib/%.S
	@ echo "  Compiling \"user/$<\"..."
	@ $(CC) -c $< -o $@ $(ASFLAGS) $(CFLAGS_LIB)

%.o: %.S
	@ echo "  Compiling \"user/$<\"..."
	@ $(CC) -c $< -o $@ $(ASFLAGS) $(CFLAGS)

########
# link step for static libraries and shared libraries
########

LDFLAGS := -m elf_i386 -z nodefaultlib -Llib/

# - there are 3 libraries: libc, ld-weenix, libtest
# - each library is built from the set of object files contained in its
#   directory; this set is generated from the list of sources by replacing
#   the .c suffix (or .S suffix) with a .o suffix
# - shared libraries are built with ld -shared, static libraries are built
#   with ar

# 1. libc

LIBC_SOURCES := $(wildcard lib/libc/*.[cS])
LIBC_OBJECTS := $(addsuffix .o,$(basename $(LIBC_SOURCES)))

lib/libc.so: $(LIBC_OBJECTS)
	@ echo "  Linking for \"user/$@\"..."
	@ $(LD) -o $@ $^ $(LDFLAGS) -shared -soname=/lib/libc.so \
--dynamic-linker /lib/ld-weenix.so

lib/libc.a: $(LIBC_OBJECTS)
	@ echo "  Creating \"user/$@\"..."
	@ $(AR) crs $@ $^

# 2. libtest

LIBTEST_SOURCES := $(wildcard lib/libtest/*.[cS])
LIBTEST_OBJECTS := $(addsuffix .o,$(basename $(LIBTEST_SOURCES)))

lib/libtest.so: $(LIBTEST_OBJECTS) lib/libc.so
	@ echo "  Linking for \"user/$@\"..."
	@ $(LD) -o $@ $^ $(LDFLAGS) -shared -soname=/lib/libtest.so -lc

lib/libtest.a: $(LIBTEST_OBJECTS)
	@ echo "  Creating \"user/$@\"..."
	@ $(AR) crs $@ $^

# 3. ld-weenix

LDWEENIX_SOURCES := $(wildcard lib/ld-weenix/*.[cS])
LDWEENIX_OBJECTS := $(addsuffix .o,$(basename $(LDWEENIX_SOURCES)))

lib/ld-weenix.so: $(LDWEENIX_OBJECTS) lib/libc.a
	@ echo "  Linking for \"user/$@\"..."
	@ $(LD) -o $@ $(LDWEENIX_OBJECTS) $(LDFLAGS) -shared \
-soname=/lib/ld-weenix.so -Bsymbolic -e _bootstrap -static -lc

LIB_OBJECTS := $(LIBC_OBJECTS) $(LIBTEST_OBJECTS) $(LDWEENIX_OBJECTS)

########
# link step for executables
########

# - all executables are generated from a single .o file
# - executables are either statically or dynamically linked, and get built
#   with a ".exec" suffix (the suffix makes the rule definition easier, and
#   gets removed in the staging step)
# - all executables build with libc
# - statically-linked executables build with entry.o

# 1. executables that require libtest:

ifeq ($(DYNAMIC),0)
usr/bin/%.exec: usr/bin/tests/%.o lib/libc.a lib/libc/entry.o lib/libtest.a
	@ echo "  Linking for \"user/$@\" (static)..."
	@ $(LD) -o $@ $< lib/libc/entry.o $(LDFLAGS) -static \
-e __libc_static_entry -lc -ltest
else
usr/bin/%.exec: usr/bin/tests/%.o lib/libc.so lib/libtest.so
	@ echo "  Linking for \"user/$@\" (dynamic)..."
	@ $(LD) -o $@ $< $(LDFLAGS) -e main --dynamic-linker /lib/ld-weenix.so \
-lc -ltest
endif

# 2. all other executables

ifeq ($(DYNAMIC),0)
%.exec: %.o lib/libc.a lib/libc/entry.o
	@ echo "  Linking for \"user/$@\" (static)..."
	@ $(LD) -o $@ $< lib/libc/entry.o $(LDFLAGS) -static \
-e __libc_static_entry -lc
else
%.exec: %.o lib/libc.so
	@ echo "  Linking for \"user/$@\" (dynamic)..."
	@ $(LD) -o $@ $< $(LDFLAGS) -e main --dynamic-linker /lib/ld-weenix.so \
-lc
endif

########
# generate the disk image layout
########

# - below is an explicit list of files that will reside on the disk image
# - all of these files get staged in the staging directory
# - executables get their ".exec" suffix stripped off in this step

# TODO also create make /tmp directory (some userspace test expects it)
BASE_TARGETS := README hamlet test/stuff
LIB_TARGETS := lib/ld-weenix.so lib/libc.a lib/libc.so lib/libtest.a \
lib/libtest.so
EXEC_TARGETS := bin/ed bin/ls bin/sh bin/uname bin/hd bin/stat \
sbin/halt sbin/init \
usr/bin/args usr/bin/hello usr/bin/kshell usr/bin/segfault usr/bin/spin \
usr/bin/eatmem usr/bin/forkbomb usr/bin/memtest usr/bin/stress usr/bin/vfstest \
usr/bin/wc usr/bin/forktest usr/bin/eatinodes usr/bin/pipetest

EXEC_SUFFIX := .exec
EXEC_TARGETS_WITH_SUFFIX := $(addsuffix $(EXEC_SUFFIX),$(EXEC_TARGETS))

# don't include any executables in the disk until VM
ifeq ($(VM),1)
TARGETS := $(BASE_TARGETS) $(LIB_TARGETS) $(EXEC_TARGETS_WITH_SUFFIX)
else
TARGETS := $(BASE_TARGETS)
endif

# TODO get rid of "mkdir -p", "cp --parents" (they are not portable)
$(STAGING_DIR): $(TARGETS)
	@ echo "  Staging initial disk contents..."
	@ mkdir -p $(STAGING_DIR)
	@ cp --parents $? $(STAGING_DIR)
# below: strip .exec suffix from binaries
# (portable implementation of "rename 's/\.exec//' $?")
	@ cd $(STAGING_DIR) \
&& for i in `ls $? | grep "\.exec"`; do \
mv $$i `echo $$i | cut -f1 -d.`; \
done

########
# build the disk image
########

$(DISK_IMAGE): $(STAGING_DIR)
	@ echo "  Running fsmaker to create \"user/$@\"..."
	@ echo "  Disk Blocks: $(DISK_BLOCKS)"
	@ echo "  Disk Inodes: $(DISK_INODES)"
	@ $(PYTHON) ../tools/fsmaker/sh.py $@ -e "format -b $(DISK_BLOCKS) -i $(DISK_INODES) -d $<"

########
# clean
########

clean:
	rm -f $(DISK_IMAGE) $(LIB_TARGETS) $(EXEC_TARGETS_WITH_SUFFIX) \
$(LIB_OBJECTS)
	rm -rf $(STAGING_DIR)
