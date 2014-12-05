BUILD_DIR := build
DOC_DIR   := doc
CFLAGS    := -ffreestanding
RSFLAGS   := --sysroot=. --target mk/i686-reenix.json
RDFLAGS   := --no-defaults --passes collapse-docs --passes strip-hidden --target mk/i686-reenix.json
LDFLAGS   := -m elf_i386 -z nodefaultlib
KERNEL_LDFLAGS := --strip-all

include ../Global.mk
include mk/Funcs.mk

CFLAGS    += -D__KERNEL__

ifeq ("true",$(LD_OPT))
LDFLAGS += --gc-sections
endif

###

# Plugins
PLUGINS        := enabled hoare bassert
# Crates for the kernel
REENIX_CRATES  := main base mm startup procs drivers util umem
# Crates from the Rust standard library
BUILTIN_CRATES := alloc core collections unicode
# Crates that are included but are not directly part of the kernel or stdlib
OTHER_CRATES   := libc
# C code helpers for Rust
REENIX_CSRC    := drivers/pci.c \
                  drivers/blockdev/disk/ata.c \
                  base/timer.c \
                  procs/rcontext.c \
                  entry/entry.c

BOOT_ENTRY     := $(BUILD_DIR)/boot/boot.o

CRATES         := $(REENIX_CRATES) $(BUILTIN_CRATES) $(PLUGINS) $(OTHER_CRATES)

$(foreach dr,$(REENIX_CRATES), $(call set-crate-name,$(dr)))
$(foreach dr,$(BUILTIN_CRATES),$(call set-builtin-crate-name,$(dr)))
$(foreach dr,$(PLUGINS),       $(call set-plugin-crate-name,$(dr)))
$(call set-base-crate-name,libc,rustlibs/liblibc)

BUILT_CRATES := $(foreach dr, $(CRATES), $(call lib-name,$(dr)))
CRATE_DOCS   := $(foreach dr, $(CRATES), $(call doc-name,$(dr)))

HEAD      := $(wildcard include/*/*.h include/*/*/*.h)
SRCDIR    := main util mm
SRC       := $(sort $(REENIX_CSRC), $(foreach dr, $(SRCDIR), $(wildcard $(dr)/*.[cS])))
COBJS     := $(addprefix $(BUILD_DIR)/,$(addsuffix .o,$(basename $(SRC))))
SCRIPTS   := $(foreach dr, $(SRCDIR), $(wildcard $(dr)/*.gdb $(dr)/*.py))
GRUB_CFG  := boot/grub.cfg

KERNEL_RDFLAGS := $(foreach dr, $(CRATES), --extern $(dr)=$(call lib-name,$(dr))) \
                  -L $(BUILD_DIR)/external -L $(BUILD_DIR)/libs --cfg kernel
KERNEL_RSFLAGS := $(KERNEL_RDFLAGS)
KERNEL_RSFLAGS += -C no-redzone -Z gc

# List of prebuilt modules that do not include the source
PREBUILT  :=

# List of external modules for various things (Such as a small libm).
EXTERNAL_LIBS  := openlibm
EXTERNAL       := $(foreach l, $(EXTERNAL_LIBS), $(BUILD_DIR)/external/lib$(l).a)

# boot/boot.S must be the first file so that the multiboot header is close enough to the front.
OBJS := $(BOOT_ENTRY) $(COBJS) $(PREBUILT) $(EXTERNAL) $(call lib-name,main)

$(call set-lib-name,morestack,libmorestack.a)
$(call set-lib-name,compiler-rt,libcompiler-rt.a)
MORESTACK  := $(BUILD_DIR)/external/rust/src/rt/arch/i386/morestack.o

SECONDARY_OBJS := $(MORESTACK) $(call lib-name, morestack) $(call lib-name,compiler-rt)

ifeq (true,$(BUILD_COMPILER_RT))
# Build it from the rust tree. This might take a while.
LIBCOMPILER_RT_SOURCE := $(BUILD_DIR)/external/libcompiler-rt.a
else
ifneq ("",$(COMPILER_RT_PATH))
# Take it from the configured place
LIBCOMPILER_RT_SOURCE := $(COMPILER_RT_PATH)
else
# By default we will just take libgcc.a
LIBCOMPILER_RT_SOURCE := $(shell $(CC) $(CFLAGS) -print-libgcc-file-name)
endif
endif


SYMBOLS    := $(BUILD_DIR)/weenix.dbg
BSYMBOLS   := $(BUILD_DIR)/symbols.dbg
KERNEL     := $(BUILD_DIR)/kernel.bin
IMAGE      := $(BUILD_DIR)/weenix.img
ISO_IMAGE  := $(BUILD_DIR)/weenix.iso
GDBCOMM    := $(BUILD_DIR)/gdb-commands
ISO_DIR    := $(BUILD_DIR)/iso

ISO_CFG    := $(ISO_DIR)/boot/grub/grub.cfg
ISO_KERNEL := $(ISO_DIR)/boot/kernel.bin

RSFLAGS := $(strip $(RSFLAGS))
RDFLAGS := $(strip $(RDFLAGS))
KERNEL_RSFLAGS := $(strip $(KERNEL_RSFLAGS))
KERNEL_RDFLAGS := $(strip $(KERNEL_RDFLAGS))

ifeq ("$(wildcard external/rust/src/rt/arch/i386/morestack.S)","")
$(error "Unable to find external/rust/..., have you run `git submodule init && git submodule update`")
endif

.PHONY: all cscope clean tidy docs

.DEFAULT_GOAL := all
all:  $(SYMBOLS) $(BSYMBOLS) $(ISO_IMAGE) $(GDBCOMM)
docs: $(CRATE_DOCS)

# Other libs
$(eval $(call other-crate-rule, libc, rustlibs/liblibc, core))

# Rust libs
$(eval $(call builtin-crate-rule,      core,))
$(eval $(call builtin-crate-rule,      unicode,     core))
$(eval $(call long-builtin-crate-rule, alloc,       core libc mm, \
              -A dead_code --cfg external_crate --extern external=$(call lib-name,mm)))
$(eval $(call builtin-crate-rule,      collections, core libc mm alloc unicode))

# Plugins
$(eval $(call plugin-rule, bassert,))
$(eval $(call plugin-rule, enabled,))
$(eval $(call plugin-rule, hoare  ,))

# Reenix Crates.
BASIC_REQS := alloc base collections core libc mm
$(eval $(call crate-rule,      base,    core libc))
$(eval $(call crate-rule,      mm,      core libc base))
$(eval $(call crate-rule,      startup, $(BASIC_REQS)))
$(eval $(call crate-rule,      util,    $(BASIC_REQS)))
$(eval $(call crate-rule,      procs,   $(BASIC_REQS) util startup))
$(eval $(call crate-rule,      umem,    $(BASIC_REQS) util procs))
$(eval $(call long-crate-rule, drivers, $(BASIC_REQS) procs umem, --opt-level=1)) # TODO For some reason LLVM doesn't like -O2 in drivers.

# The `main` crate.
$(eval $(call crate-rule, main,  $(filter-out main, $(CRATES)) morestack compiler-rt))

# Make the target for libm
$(eval $(call external-targets,openlibm,libopenlibm.a,libopenlibm.a,.,              \
	SFLAGS="-m32 -march=i386"                                                       \
   	LDFLAGS="-m32 -march=i386 -ffreestanding -nostdlib -static"                     \
	CFLAGS="-Wno-maybe-uninitialized -g3 -gdwarf-3 -m32 -march=i386 -ffreestanding" \
	ARCH=i386,))

# Make the targets for building libcompiler-rt.a, if we want to.
$(eval $(call configure-targets,external/rust,--target=i686-unknown-linux-gnu,Makefile))
$(eval $(call external-targets,rust,                                                      \
	i686-unknown-linux-gnu/rt/libcompiler-rt.a,i686-unknown-linux-gnu/rt/libcompiler-rt.a,\
    src,,external/rust/Makefile))

# TODO linking twice is bad and fragile.
# TODO We really should be modifying $(KERNEL) to make it's paddr's match the vaddr's in its program headers.
$(eval $(call ld-rule,   $(SYMBOLS),  $(OBJS), mk/debug.ld, ))
$(eval $(call ld-rule,   $(KERNEL),   $(OBJS), mk/link.ld,  $(KERNEL_LDFLAGS)))

# Make libmorestack.a by compiling morestack.S
$(eval $(call ar-rule,   $(call lib-name,morestack), $(MORESTACK)))
# get either libgcc.a or libcompiler-rt.a to where it belongs
$(eval $(call copy-rule, $(LIBCOMPILER_RT_SOURCE), $(call lib-name,compiler-rt)))

$(eval $(call copy-rule, $(KERNEL),   $(ISO_KERNEL)))
$(eval $(call copy-rule, $(GRUB_CFG), $(ISO_CFG)))

$(BSYMBOLS): $(SYMBOLS)
	@ echo "[ELF ] Generating kernel symbols list..."
	$(HIDE_SIGIL) readelf -Ws $(SYMBOLS) | grep -Ev 'SECTION|UND|FILE|Num:|Symbol|^$$' | awk '{printf "0x%s %s\n", $$2, $$8}' > $@

$(ISO_IMAGE): $(ISO_KERNEL) $(ISO_CFG)
	@ echo "[GRUB] Creating \"kernel/$@\"..."
	$(HIDE_SIGIL) $(MKRESCUE) -o $@ $(ISO_DIR) $(SILENT_SUFFIX)

$(GDBCOMM): $(SCRIPTS)
	@ echo "[GDB ] Creating gdb command list..."
	$(HIDE_SIGIL) $(foreach script,$^,echo $(abspath $(script)) >> $@; )

# Make sure build-directory dirs are there.
$(OBJS) $(SECONDARY_OBJS) : | $(foreach l,$(OBJS) $(SECONDARY_OBJS), $(dir $(l))) $(BUILD_DIR)
$(BUILD_DIR) $(sort $(foreach l,$(OBJS) $(SECONDARY_OBJS), $(dir $(l)))) :
	@ echo "[MKDR] Make build-directory \"kernel/$@\"..."
	$(HIDE_SIGIL) mkdir -p $@

$(BUILD_DIR)/%.o: %.c
	@ echo "[CC  ] Compiling \"kernel/$<\"..."
	$(HIDE_SIGIL) $(CC) -c $(CFLAGS) $< -o $@

$(BUILD_DIR)/%.o: %.S
	@ echo "[AS  ] Compiling \"kernel/$<\"..."
	$(HIDE_SIGIL) $(CC) -g3 -c $(ASFLAGS) $(CFLAGS) $< -o $@

tidy:
	$(HIDE_SIGIL) rm -f	cscope*.out cscope.files 2>/dev/null
	$(HIDE_SIGIL) rm -rf $(BUILD_DIR)            2>/dev/null

clean: tidy $(foreach d,$(EXTERNAL_LIBS),clean-$(d))
	$(HIDE_SIGIL) rm -rf $(DOC_DIR)

include mk/Helpers.mk
