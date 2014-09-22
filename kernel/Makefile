CFLAGS    := -ffreestanding
RSFLAGS   ?=
LDFLAGS   := -m elf_i386 -z nodefaultlib
# XXX should have --omagic?

include ../Global.mk

CFLAGS    += -D__KERNEL__
RSFLAGS   += --cfg kernel

###

HEAD      := $(wildcard include/*/*.h include/*/*/*.h)
SRCDIR    := . main boot util drivers/disk drivers/tty drivers mm proc fs/ramfs fs/s5fs fs vm api test test/kshell entry test/vfstest
#SRC       := $(foreach dr, $(SRCDIR), $(wildcard $(dr)/*.S) $(wildcard $(dr)/*.rs))
SRC       := $(foreach dr, $(SRCDIR), $(wildcard $(dr)/*.[cS]) )
OBJS      := $(addsuffix .o,$(basename $(SRC))) reenix.a
SCRIPTS   := $(foreach dr, $(SRCDIR), $(wildcard $(dr)/*.gdb $(dr)/*.py))

MODULES := base mm alloc libc
BUILT_MODULES := $(foreach dr, $(MODULES), libs/lib$(dr).rlib)

RSFLAGS += $(foreach dr, $(MODULES), --extern $(dr)=libs/lib$(dr).rlib)

# List of prebuilt modules that do not include the source
PREBUILT  :=
# List of external modules for various things (Such as a small libm).
EXTERNAL_LIBS  := openlibm
EXTERNAL := $(foreach l, $(EXTERNAL_LIBS), libs/lib$(l).a)

ifneq "$(shell which mkisofs 2>/dev/null)" ""
    MAKEISO := mkisofs
else
    MAKEISO := genisoimage
endif

HIDE ?= "true"
ifeq ("true",$(HIDE))
HIDE_SIGIL := @
SILENT_FLAG := --silent
else
HIDE_SIGIL :=
SILENT_FLAG :=
endif

SYMBOLS   := weenix.dbg
BSYMBOLS  := symbols.dbg
KERNEL    := kernel.bin
IMAGE     := weenix.img
ISO_IMAGE := weenix.iso
GDBCOMM   := gdb-commands

# $(1) is the name of the target under external
# $(2) is the name of the target we wish to build in external.
# $(3) is the name of the file we wish to take into libs
# $(4) are any flags we wish to pass down.
define external-targets
libs/$(notdir $(3)) : external/$(1)/$(3)
	@ echo "   Copying \"kernel/$$@\"..."
	$(HIDE_SIGIL) cp external/$(1)/$(3) $$@

./external/$(1)/$(3) : $$(shell find ./external/$(1) -type f -not -path ./external/$(1)/$(2))
	@ echo "   Recursive make of \"kernel/$$@\"..."
	$(HIDE_SIGIL) $(MAKE) $(SILENT_FLAG) -C external/$(1) $(2) $(4)

.PHONEY:
clean-$(1):
	$(HIDE_SIGIL) rm libs/$(notdir $(3))
	$(HIDE_SIGIL) $(MAKE) $(SILENT_FLAG) -C external/$(1) clean $(4)
endef
$(eval $(call external-targets,openlibm,libopenlibm.a,libopenlibm.a,$(MFLAGS) SFLAGS="-m32 -march=i386" LDFLAGS="-m32 -march=i386 -ffreestanding -nostdlib -static" CFLAGS="-g -m32 -march=i386 -ffreestanding" ARCH=i386))

# $(1) is the directory the library is in
# $(2) is the name of the crate
# $(3) is the list of crates that this library depends on.
define base-crate-rule
libs/lib$(2).rlib : $$(shell find $(1) -type f) $$(foreach l,$(3), libs/lib$$(l).rlib)
	$(HIDE_SIGIL) $(RUST) $(RSFLAGS) --crate-type=rlib -L libs $(1)/lib.rs --out-dir libs
endef

define crate-rule
$(eval $(call base-crate-rule,$(1),$(1),$(2)))
endef

# A module that is part of rusts stdlib.
# $(1) is the name of the crate.
# $(2) is the list of dependencies
define builtin-crate-rule
$(eval $(call base-crate-rule,rustlibs/lib$(1),$(1),$(2)))
endef

$(eval $(call builtin-crate-rule,libc,))
$(eval $(call crate-rule,base,libc))
$(eval $(call crate-rule,mm,base libc))
$(eval $(call builtin-crate-rule,alloc,mm base libc))

.PHONY: all cscope clean

%.a:
	touch $@

.DEFAULT_GOAL := all
all: $(BSYMBOLS) $(ISO_IMAGE) $(GDBCOMM)

$(SYMBOLS): $(OBJS) $(PREBUILT) $(EXTERNAL)
	@ echo "  Linking for \"kernel/$@\"..."
	@ $(LD) $(LDFLAGS) -T link.ld $(filter-out entry/entry.o,$^) -o $@ # entry.o included from link.ld

$(BSYMBOLS): $(SYMBOLS)
	@ echo "  Generating kernel symbols list..."
	@ readelf -s $(SYMBOLS) | grep -Ev 'SECTION|UND|FILE|Num:|Symbol|^$$' | awk '{printf "0x%s %s\n", $$2, $$8}' > $@

$(KERNEL): $(SYMBOLS)
	@ echo "  Setting addresses for kernel stage 1 and stage 2..."
	@ objcopy -j .stage1 --set-section-flags .stage1=load --change-section-address .stage1=0 -O binary $< /tmp/temptemp
	@ cat /tmp/temptemp > kernel.bin
	@ objcopy -j .stage2 --set-section-flags .stage2=load --change-section-address .stage2=0 -O binary $< /tmp/temptemp
	@ cat /tmp/temptemp >> kernel.bin
	@ objcopy -O binary $< /tmp/temptemp
	@ cat /tmp/temptemp >> kernel.bin
	@ rm /tmp/temptemp # XXX find better solution than this

$(IMAGE): $(KERNEL)
	@ echo "  Creating floppy disk image from kernel binary..."
	@ dd if=/dev/zero of=$@ bs=1024 count=1440 2> /dev/null
	@ dd if=$< of=$@ conv=notrunc 2> /dev/null

$(ISO_IMAGE): $(IMAGE)
	@ echo "  Creating \"kernel/$@\" from floppy disk image..."
	@ $(MAKEISO) -quiet -input-charset utf-8 -o $@ -b $< $<

$(GDBCOMM): $(SCRIPTS)
	@ echo "  Creating gdb command list..."
	@ $(foreach script, $(SCRIPTS), echo $(abspath $(script)) >> $(dir $(script))$(shell basename $(dir $(script))).gdbcomm; )
	@ cat */*.gdbcomm > $@

#-include out.mk
#main.o: reenix.rs main/mod.rs
#	@ echo "  Compiling \"kernel/$<\"..."
#	$(RUST) --emit=obj $(RSFLAGS) $< -o $@

reenix.a : reenix.rs main/mod.rs $(BUILT_MODULES)
	@ echo "  Compiling \"kernel/$<\"..."
	$(HIDE_SIGIL) $(RUST) --crate-type=staticlib -L libs $(RSFLAGS) $< -o $@
	$(HIDE_SIGIL) ranlib $@

%.o: %.c
	@ echo "  Compiling \"kernel/$<\"..."
	$(HIDE_SIGIL) $(CC) -c $(CFLAGS) $< -o $@

%.o: %.S
	@ echo "  Compiling \"kernel/$<\"..."
	$(HIDE_SIGIL) $(CC) -c $(ASFLAGS) $(CFLAGS) $< -o $@

cscope: $(HEAD) $(SRC)
	@ echo "  Updating cscope symbol cross-reference..."
	@ echo $(HEAD) $(SRC) > cscope.files
	@ $(CSCOPE) -k -b -q -v > /dev/null

FILTER=`echo "DRIVERS $(DRIVERS)\nVFS $(VFS)\nS5FS $(S5FS)\nVM $(VM)" | grep 1 | cut -f1 -d" " | tr "\n" "|"`PROCS
nyi:
	@ echo "  Not yet implemented:"
	@ echo
	@ find . -name \*.c -printf "%P\n" \
| xargs grep -Hn "NOT_YET_IMPLEMENTED" \
| sed -e 's/^\(.*:.*\):.*\"\(.*\): \(.*\)\".*/\2 \1 \3/' \
| grep -E "^($(FILTER))" \
| awk '{printf("%25s %30s() %8s\n", $$2, $$3, $$1)}'

todo:
	@ echo "  Not yet done:"
	@ echo
	@ git grep --heading --break -Epnh "((\/\/|\/?\*) TODO|^ *\*t )" | sed -E "s/^([0-9]+)/\t\1/" \
| sed -E "s/^\t?[0-9]+=/    /" \
| sed -E "/^    .*;$$/d"

clean: $(foreach d,$(EXTERNAL_LIBS),clean-$(d))
	$(HIDE_SIGIL) rm -f $(BUILT_MODULES) $(OBJS) $(SYMBOLS) $(BSYMBOLS) $(KERNEL) $(IMAGE) $(ISO_IMAGE) $(GDBCOMM) $(EXTERNAL) */*.gdbcomm cscope*.out cscope.files
