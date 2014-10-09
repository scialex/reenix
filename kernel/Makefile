CFLAGS    := -ffreestanding
RSFLAGS   = -L libs -Z gc
LDFLAGS   := -m elf_i386 -z nodefaultlib
# XXX should have --omagic?

include ../Global.mk

CFLAGS    += -D__KERNEL__
RSFLAGS   += --cfg kernel

###

HEAD      := $(wildcard include/*/*.h include/*/*/*.h)
SRCDIR    := main boot util mm entry procs
#SRC       := $(foreach dr, $(SRCDIR), $(wildcard $(dr)/*.S) $(wildcard $(dr)/*.rs))
SRC       := $(foreach dr, $(SRCDIR), $(wildcard $(dr)/*.[cS]) ) drivers/pci.c
OBJS      := $(addsuffix .o,$(basename $(SRC))) libs/libreenix.a
SCRIPTS   := $(foreach dr, $(SRCDIR), $(wildcard $(dr)/*.gdb $(dr)/*.py))

MODULES := base mm alloc libc main startup collections procs
BUILT_MODULES := $(foreach dr, $(MODULES), libs/lib$(dr).rlib)

RSFLAGS += $(foreach dr, $(MODULES), --extern $(dr)=libs/lib$(dr).rlib)

# List of prebuilt modules that do not include the source
PREBUILT  :=
# List of external modules for various things (Such as a small libm).
EXTERNAL_LIBS  := openlibm
EXTERNAL := $(foreach l, $(EXTERNAL_LIBS), libs/lib$(l).a)

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

RSFLAGS := $(strip $(RSFLAGS))

# $(1) is the name of the target under external
# $(2) is the name of the target we wish to build in external.
# $(3) is the name of the file we wish to take into libs
# $(4) are any flags we wish to pass down.
define external-targets
./libs/$(notdir $(3)) : external/$(1)/$(3)
	@ echo "  Copying \"kernel/$$@\"..."
	$(HIDE_SIGIL) cp external/$(1)/$(3) $$@

./external/$(1)/$(3) : $$(shell find ./external/$(1) -type f -not -path ./external/$(1)/$(3))
	@ echo "  Recursive make of \"kernel/$$@\"..."
	$(HIDE_SIGIL) $$(MAKE) $(SILENT_FLAG) $$(MFLAGS) -C external/$(1) $(2) $(4)

.PHONEY:
clean-$(1):
	$(HIDE_SIGIL) rm libs/$(notdir $(3)) || true
	$(HIDE_SIGIL) $(MAKE) $(SILENT_FLAG) -C external/$(1) clean $(4)
endef
$(eval $(call external-targets,openlibm,libopenlibm.a,libopenlibm.a,SFLAGS="-m32 -march=i386" LDFLAGS="-m32 -march=i386 -ffreestanding -nostdlib -static" CFLAGS="-Wno-maybe-uninitialized -g -m32 -march=i386 -ffreestanding" ARCH=i386))

# $(1) is the directory the library is in
# $(2) is the name of the crate
# $(3) is the list of crates that this library depends on.
# $(4) is any additional rust flags you want.
define base-crate-rule
libs/lib$(2).rlib : $$(shell find $(1) -type f) $$(foreach l,$(3), libs/lib$$(l).rlib)
	@ echo "  Compiling \"kernel/$(1)/lib.rs\"..." # for \"kernel/$$@\""
	$(HIDE_SIGIL) $(RUST) $(RSFLAGS) $(4) --crate-type=rlib $(1)/lib.rs --out-dir libs
endef

define crate-rule
$(eval $(call base-crate-rule,$(1),$(1),$(2),))
endef

# A module that is part of rusts stdlib.
# $(1) is the name of the crate.
# $(2) is the list of dependencies
define builtin-crate-rule
$(eval $(call base-crate-rule,rustlibs/lib$(1),$(1),$(2),--allow=dead-code))
endef

$(eval $(call builtin-crate-rule,libc,))
$(eval $(call builtin-crate-rule,collections,alloc libc))
$(eval $(call crate-rule,base,libc))
$(eval $(call crate-rule,mm,base libc))
$(eval $(call crate-rule,startup,mm base libc))
$(eval $(call builtin-crate-rule,alloc,startup mm base libc))
$(eval $(call crate-rule,procs,startup alloc base libc mm collections))


$(eval $(call crate-rule,main,$(filter-out main,$(MODULES))))

.PHONY: all cscope clean

%.a:
	touch $@

.DEFAULT_GOAL := all
all: $(SYMBOLS) $(BSYMBOLS) $(ISO_IMAGE) $(GDBCOMM)

$(KERNEL): $(OBJS) $(PREBUILT) $(EXTERNAL)
	@ echo "  Linking for \"kernel/$@\"..."
	@ # entry.o included from link.ld. boot/boot.S must be the first file so that the multiboot header is close enough to the front.
	$(HIDE_SIGIL) $(LD) $(LDFLAGS) -T link.ld boot/boot.o $(filter-out boot/boot.o entry/entry.o,$^) -o $@

$(BSYMBOLS): $(SYMBOLS)
	@ echo "  Generating kernel symbols list..."
	$(HIDE_SIGIL) readelf -Ws $(SYMBOLS) | grep -Ev 'SECTION|UND|FILE|Num:|Symbol|^$$' | awk '{printf "0x%s %s\n", $$2, $$8}' > $@

$(SYMBOLS): $(OBJS) $(PREBUILT) $(EXTERNAL)
	@ echo "  Generating a image for debugging..."
	@ # TODO This is bad and fragile. We really should be modifying $(KERNEL) to make it's paddr's match the vaddr's in its program headers.
	@ # entry.o included from link.ld. boot/boot.S must be the first file so that the multiboot header is close enough to the front.
	$(HIDE_SIGIL) $(LD) $(LDFLAGS) -T debug.ld boot/boot.o $(filter-out boot/boot.o entry/entry.o,$^) -o $@

$(ISO_IMAGE): $(KERNEL)
	@ echo "  Creating \"kernel/$@\" from floppy disk image..."
	$(HIDE_SIGIL) mkdir -p .iso/boot/grub
	$(HIDE_SIGIL) ln -f $< .iso/boot/$<
	$(HIDE_SIGIL) echo "default=0"                           >  .iso/boot/grub/grub.cfg
	$(HIDE_SIGIL) echo "timeout=0"                           >> .iso/boot/grub/grub.cfg
	$(HIDE_SIGIL) echo "menuentry \"$@\" {"                  >> .iso/boot/grub/grub.cfg
	$(HIDE_SIGIL) echo " echo \"Booting $@ from /boot/$<\" " >> .iso/boot/grub/grub.cfg
	$(HIDE_SIGIL) echo " echo \"Welcome To Weenix!\" "       >> .iso/boot/grub/grub.cfg
	$(HIDE_SIGIL) echo " multiboot /boot/$< "                >> .iso/boot/grub/grub.cfg
	$(HIDE_SIGIL) echo " boot "                              >> .iso/boot/grub/grub.cfg
	$(HIDE_SIGIL) echo "}"                                   >> .iso/boot/grub/grub.cfg
	$(HIDE_SIGIL) grub-mkrescue -o $@ ./.iso

$(GDBCOMM): $(SCRIPTS)
	@ echo "  Creating gdb command list..."
	$(HIDE_SIGIL) $(foreach script, $(SCRIPTS), echo $(abspath $(script)) >> $(dir $(script))$(shell basename $(dir $(script))).gdbcomm; )
	$(HIDE_SIGIL) cat */*.gdbcomm > $@

libs/libreenix.a : reenix.rs $(BUILT_MODULES)
	@ echo "  Compiling \"kernel/$<\"..."
	$(HIDE_SIGIL) $(RUST) --crate-type=staticlib $(RSFLAGS) $< --out-dir=libs

%.o: %.c
	@ echo "  Compiling \"kernel/$<\"..."
	$(HIDE_SIGIL) $(CC) -c $(CFLAGS) $< -o $@

%.o: %.S
	@ echo "  Compiling \"kernel/$<\"..."
	$(HIDE_SIGIL) $(CC) -g -c $(ASFLAGS) $(CFLAGS) $< -o $@

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
	@ git grep --heading --break -Epnh "((\/\/|\/?\*) TODO|^ *\*t )" \
| sed -E "s/^\t?[0-9]+=/  /" \
| sed -E "/^  .*;$$/d" \
| awk '                                                       \
    BEGIN {                                                   \
        FS       = ":";                                       \
        count    = 0;                                         \
        cur_file = "";                                        \
        prev     = "";                                        \
        prev_cnt = 0;                                         \
    }                                                         \
    {                                                         \
        if ($$0 !~ /^[0-9]+:/) {                              \
            if ($$0 !~ /^ +/) {                               \
                if (cur_file != "") {                         \
                    plural = "";                              \
                    if (prev_cnt != 1) {                      \
                        plural = "s";                         \
                    }                                         \
                    printf("%s: %d thing%s to do\n%s\n",      \
                           cur_file, prev_cnt, plural, prev); \
                }                                             \
                cur_file = $$0;                               \
                prev_cnt = 0;                                 \
                prev     = "";                                \
            } else {                                          \
                prev = prev "" $$0 "\n";                      \
            }                                                 \
        } else {                                              \
            count    += 1;                                    \
            prev_cnt += 1;                                    \
            first     = $$1;                                  \
            $$1       = "";                                   \
            prev      = prev "" sprintf("%7s: %s\n",          \
                                        first, $$0);          \
        }                                                     \
    }                                                         \
    END {                                                     \
        if (cur_file != "") {                                 \
            plural = "";                                      \
            if (prev_cnt != 1) {                              \
                plural = "s";                                 \
            }                                                 \
            printf("%s: %d thing%s to do\n%s",                \
                   cur_file, prev_cnt, plural, prev);         \
        }                                                     \
        printf("\nTotal of %d TODOs\n", count);               \
    }                                                         \
    '

.PHONY:
tidy:
	$(HIDE_SIGIL) rm -f $(BUILT_MODULES) $(OBJS) $(SYMBOLS) $(BSYMBOLS) $(KERNEL) $(IMAGE) $(ISO_IMAGE) $(GDBCOMM) */*.gdbcomm cscope*.out cscope.files
	$(HIDE_SIGIL) rm -rf .iso

.PHONY:
clean: tidy $(foreach d,$(EXTERNAL_LIBS),clean-$(d))
