KERNEL_DIR     := .
BUILD_DIR      := build
DOC_DIR        := doc
CFLAGS         := -ffreestanding
RSFLAGS        := --sysroot=. -C no-redzone -Z gc
RDFLAGS        := --no-defaults --passes collapse-docs --passes strip-hidden
LDFLAGS        := -m elf_i386 -z nodefaultlib
KERNEL_LDFLAGS := --strip-all

SYMBOLS        := $(BUILD_DIR)/weenix.dbg
BSYMBOLS       := $(BUILD_DIR)/symbols.dbg
KERNEL         := $(BUILD_DIR)/kernel.bin
IMAGE          := $(BUILD_DIR)/weenix.img
ISO_IMAGE      := $(BUILD_DIR)/weenix.iso
GDBCOMM        := $(BUILD_DIR)/gdb-commands
ISO_DIR        := $(BUILD_DIR)/iso
# The target if we build LIBCOMPILER_RT from source
LIBCOMPILER_RT := $(BUILD_DIR)/external/libcompiler-rt.a

ISO_CFG        := $(ISO_DIR)/boot/grub/grub.cfg
ISO_KERNEL     := $(ISO_DIR)/boot/kernel.bin

include ../Global.mk
include mk/Funcs.mk

RDFLAGS += -L $(BUILD_DIR)/libs --cfg kernel --target $(TARGET)
RSFLAGS += -L $(BUILD_DIR)/libs --cfg kernel --target $(TARGET)
CFLAGS  += -D__KERNEL__

###

# Plugins these are in plugins/<name>
PLUGINS        := enabled hoare bassert

# Crates for the kernel. they are in <name>/
REENIX_CRATES  := main base mm startup procs drivers util umem

# Crates from the Rust standard library
BUILTIN_CRATES := alloc core collections unicode rand

# Crates from the Rust standard library that have been patched. They are in rustlibs/lib<name>/
PATCHED_CRATES := libc

# Crates that are not really part of the stdlib or kernel. They are in rustlibs/<name>/
OTHER_CRATES   :=

CRATES         := $(REENIX_CRATES) $(BUILTIN_CRATES) $(PLUGINS) $(PATCHED_CRATES) $(OTHER_CRATES)

# C code helpers for Rust
REENIX_CSRC    := drivers/pci.c               \
                  drivers/blockdev/disk/ata.c \
                  base/timer.c                \
                  procs/rcontext.c            \
                  entry/entry.c

# morestack.S location
MORESTACK      := $(RUST_SOURCE_DIR)/src/rt/arch/i386/morestack.S

# File containing multiboot header.
BOOT_FILE      := boot/boot.S

# grub.cfg file
GRUB_CFG       := boot/grub.cfg

# Remaining C source directories.
SRCDIR         := main util mm
SRC            := $(sort $(REENIX_CSRC) $(foreach dr, $(SRCDIR), $(wildcard $(dr)/*.[cS])))
SCRIPTS        := $(foreach dr, $(SRCDIR), $(wildcard $(dr)/*.gdb $(dr)/*.py))

# List of prebuilt modules that do not include the source
PREBUILT  :=

# List of external modules for various things (Such as a small libm).
EXTERNAL_LIBS  := openlibm
EXTERNAL       := $(foreach l, $(EXTERNAL_LIBS), $(BUILD_DIR)/external/lib$(l).a)

# List of external dependencies that are more then just libs.
EXTERNAL_DEPS  := rust

###

# Set all the name/dir metadata.
$(foreach dr, $(REENIX_CRATES),  $(call set-crate-name,$(dr)))
$(foreach dr, $(BUILTIN_CRATES), $(call set-builtin-crate-name,$(dr)))
$(foreach dr, $(PLUGINS),        $(call set-plugin-crate-name,$(dr)))
$(foreach dr, $(PATCHED_CRATES), $(call set-patched-crate-name,$(dr)))
$(foreach dr, $(OTHER_CRATES),   $(call set-other-crate-name,$(dr)))

$(call set-lib-name, morestack,   libmorestack.a)
$(call set-lib-name, compiler-rt, libcompiler-rt.a)

# boot/boot.S must be the first file so that the multiboot header is close enough to the front.
OBJS := $(call obj-name, $(BOOT_FILE)) \
    	$(call obj-name, $(SRC))       \
	   	$(PREBUILT)                    \
		$(EXTERNAL)                    \
	   	$(call lib-name,main)

SECONDARY_OBJS := $(call obj-name, $(MORESTACK)) \
              	  $(call lib-name, morestack)    \
				  $(call lib-name,compiler-rt)

RSFLAGS := $(strip $(RSFLAGS))
RDFLAGS := $(strip $(RDFLAGS))
KERNEL_RSFLAGS := $(strip $(KERNEL_RSFLAGS))
KERNEL_RDFLAGS := $(strip $(KERNEL_RDFLAGS))

.PHONY: all clean tidy docs

.DEFAULT_GOAL := all
all:  $(SYMBOLS) $(BSYMBOLS) $(ISO_IMAGE) $(GDBCOMM)
docs: $(foreach dr, $(CRATES), $(call doc-name,$(dr)))

# We need to tell liballoc to use our memory manager.
LIBALLOC_FLAGS := -A dead_code --cfg external_crate --extern external=$(call lib-name,mm) \
                  --opt-level=$(DEFAULT_CRATE_OPT)
# TODO For some reason LLVM doesn't like -O2 in drivers.
DRIVERS_FLAGS  := --opt-level=1

# Plugins
$(eval $(call plugin-rule, bassert,))
$(eval $(call plugin-rule, enabled,))
$(eval $(call plugin-rule, hoare  ,))

# The `main` crate dependencies.
MAIN_REQS := $(filter-out main, $(CRATES)) morestack compiler-rt

# Reenix Crates.
BASIC_REQS := alloc base collections core libc mm
$(eval $(call crate-rule,      core, ))
$(eval $(call crate-rule,      libc,        core))
$(eval $(call crate-rule,      rand,        core))
$(eval $(call crate-rule,      unicode,     core))
$(eval $(call crate-rule,      base,        core libc))
$(eval $(call crate-rule,      mm,          core libc base))
$(eval $(call long-crate-rule, alloc,       core libc mm,                $(LIBALLOC_FLAGS)))
$(eval $(call crate-rule,      collections, core libc alloc unicode))
$(eval $(call crate-rule,      startup,     $(BASIC_REQS)))
$(eval $(call crate-rule,      util,        $(BASIC_REQS)))
$(eval $(call crate-rule,      procs,       $(BASIC_REQS) util startup))
$(eval $(call crate-rule,      umem,        $(BASIC_REQS) util procs))
$(eval $(call long-crate-rule, drivers,     $(BASIC_REQS) procs umem,    $(DRIVERS_FLAGS)))
$(eval $(call crate-rule,      main,        $(MAIN_REQS)))

# Make the target for libm
$(eval $(call external-targets, openlibm,                                           \
	libopenlibm.a,                                                                  \
	libopenlibm.a,. ,                                                               \
	SFLAGS="-m32 -march=i386"                                                       \
   	LDFLAGS="-m32 -march=i386 -ffreestanding -nostdlib -static"                     \
	CFLAGS="-Wno-maybe-uninitialized -g3 -gdwarf-3 -m32 -march=i386 -ffreestanding" \
	ARCH=i386,))

# Make the targets for building libcompiler-rt.a, if we want to.
$(eval $(call configure-targets,external/rust,--target=i686-unknown-linux-gnu,Makefile))
$(eval $(call external-targets, rust,           \
	i686-unknown-linux-gnu/rt/libcompiler-rt.a, \
	i686-unknown-linux-gnu/rt/libcompiler-rt.a, \
    src/compiler-rt/, ,                         \
	external/rust/Makefile))

# TODO linking twice is bad and fragile.
# TODO We really should be modifying $(KERNEL) to make it's paddr's match the vaddr's in its program headers.
$(eval $(call ld-rule,   $(SYMBOLS),  $(OBJS), mk/debug.ld, ))
$(eval $(call ld-rule,   $(KERNEL),   $(OBJS), mk/link.ld,  $(KERNEL_LDFLAGS)))

# Make libmorestack.a by archiving morestack.o
$(eval $(call ar-rule,   $(call lib-name,morestack), $(call obj-name, $(MORESTACK))))

# get either libgcc.a or libcompiler-rt.a to where it belongs
$(eval $(call copy-rule, $(LIBCOMPILER_RT_SOURCE), $(call lib-name,compiler-rt)))

$(eval $(call copy-rule, $(KERNEL),   $(ISO_KERNEL)))
$(eval $(call copy-rule, $(GRUB_CFG), $(ISO_CFG)))

# Make sure build-directory dirs are there.
BUILT_FILES := $(OBJS) $(SECONDARY_OBJS) $(foreach l,$(CRATES),$(call lib-name,$(l)))
$(eval $(call ensure-build-dir, $(BUILT_FILES)))

$(BSYMBOLS): $(SYMBOLS)
	@ echo "[ELF ] Generating kernel symbols list..."
	$(HIDE_SIGIL) readelf -Ws $(SYMBOLS) | grep -Ev 'SECTION|UND|FILE|Num:|Symbol|^$$' | awk '{printf "0x%s %s\n", $$2, $$8}' > $@

$(ISO_IMAGE): $(ISO_KERNEL) $(ISO_CFG)
	@ echo "[GRUB] Creating \"kernel/$@\"..."
	$(HIDE_SIGIL) $(MKRESCUE) -o $@ $(ISO_DIR) $(SILENT_SUFFIX)

$(GDBCOMM): $(SCRIPTS)
	@ echo "[GDB ] Creating gdb command list..."
	$(HIDE_SIGIL) $(foreach script,$^,echo $(abspath $(script)) >> $@; )

$(BUILD_DIR)/%.o: %.c
	@ echo "[CC  ] Compiling \"kernel/$<\"..."
	$(HIDE_SIGIL) $(CC) -c $(CFLAGS) $< -o $@

$(BUILD_DIR)/%.o: %.S
	@ echo "[AS  ] Compiling \"kernel/$<\"..."
	$(HIDE_SIGIL) $(CC) -g3 -c $(ASFLAGS) $(CFLAGS) $< -o $@

tidy:
	$(HIDE_SIGIL) rm -f	cscope*.out cscope.files 2>/dev/null
	$(HIDE_SIGIL) rm -rf $(BUILD_DIR)            2>/dev/null

clean: tidy $(foreach d,$(EXTERNAL_LIBS) $(EXTERNAL_DEPS),clean-$(d))
	$(HIDE_SIGIL) rm -rf $(DOC_DIR)

include mk/Helpers.mk
