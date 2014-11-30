BUILD_DIR := build
DOC_DIR   := doc
CFLAGS    := -ffreestanding
RSFLAGS   :=
RDFLAGS   := --no-defaults --passes unindent-comments --passes collapse-docs --passes strip-hidden
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
BUILTIN_CRATES := alloc libc collections
# C code helpers for Rust
REENIX_CSRC    := drivers/pci.c \
                  drivers/blockdev/disk/ata.c \
                  base/timer.c \
                  procs/rcontext.c \
                  entry/entry.c

BOOT_ENTRY     := $(BUILD_DIR)/boot/boot.o

CRATES         := $(REENIX_CRATES) $(BUILTIN_CRATES) $(PLUGINS)

$(foreach dr,$(REENIX_CRATES), $(call set-crate-name,$(dr)))
$(foreach dr,$(BUILTIN_CRATES),$(call set-builtin-crate-name,$(dr)))
$(foreach dr,$(PLUGINS),       $(call set-plugin-crate-name,$(dr)))

BUILT_CRATES := $(foreach dr, $(CRATES), $(call crate-name,$(dr)))
CRATE_DOCS   := $(foreach dr, $(CRATES), $(call doc-name,$(dr)))

HEAD      := $(wildcard include/*/*.h include/*/*/*.h)
SRCDIR    := main util mm
SRC       := $(filter-out $(REENIX_CSRC), $(foreach dr, $(SRCDIR), $(wildcard $(dr)/*.[cS]))) $(REENIX_CSRC)
COBJS     := $(addprefix $(BUILD_DIR)/,$(addsuffix .o,$(basename $(SRC))))
SCRIPTS   := $(foreach dr, $(SRCDIR), $(wildcard $(dr)/*.gdb $(dr)/*.py))
GRUB_CFG  := boot/grub.cfg

KERNEL_RSFLAGS := $(foreach dr, $(CRATES), --extern $(dr)=$(call crate-name,$(dr)))
KERNEL_RSFLAGS += -L $(BUILD_DIR)/libs -C no-redzone -Z gc --cfg kernel --target=i686-unknown-linux-gnu

KERNEL_RDFLAGS := $(foreach dr, $(CRATES), --extern $(dr)=$(call crate-name,$(dr)))
KERNEL_RDFLAGS += -L $(BUILD_DIR)/libs --cfg kernel --target=i686-unknown-linux-gnu

# List of prebuilt modules that do not include the source
PREBUILT  :=

# List of external modules for various things (Such as a small libm).
EXTERNAL_LIBS  := openlibm
EXTERNAL       := $(foreach l, $(EXTERNAL_LIBS), $(BUILD_DIR)/external/lib$(l).a)

# boot/boot.S must be the first file so that the multiboot header is close enough to the front.
OBJS := $(BOOT_ENTRY) $(COBJS) $(PREBUILT) $(EXTERNAL) $(call crate-name,main)

SYMBOLS    := $(BUILD_DIR)/weenix.dbg
BSYMBOLS   := $(BUILD_DIR)/symbols.dbg
KERNEL     := $(BUILD_DIR)/kernel.bin
IMAGE      := $(BUILD_DIR)/weenix.img
ISO_IMAGE  := $(BUILD_DIR)/weenix.iso
GDBCOMM    := $(BUILD_DIR)/gdb-commands
ISO_DIR    := $(BUILD_DIR)/.iso

ISO_CFG    := $(ISO_DIR)/boot/grub/grub.cfg
ISO_KERNEL := $(ISO_DIR)/boot/kernel.bin

RSFLAGS := $(strip $(RSFLAGS))
RDFLAGS := $(strip $(RDFLAGS))
KERNEL_RSFLAGS := $(strip $(KERNEL_RSFLAGS))
KERNEL_RDFLAGS := $(strip $(KERNEL_RDFLAGS))

.PHONY: all cscope clean tidy docs

.DEFAULT_GOAL := all
all:  $(SYMBOLS) $(BSYMBOLS) $(ISO_IMAGE) $(GDBCOMM)
docs: $(CRATE_DOCS)

# Rust libs
$(eval $(call builtin-crate-rule, libc,))
$(eval $(call builtin-crate-rule, alloc,       libc mm))
$(eval $(call builtin-crate-rule, collections, libc mm alloc))

# Plugins
$(eval $(call plugin-rule, bassert,))
$(eval $(call plugin-rule, enabled,))
$(eval $(call plugin-rule, hoare  ,))

# Reenix Crates.
BASIC_REQS := alloc base collections libc mm
$(eval $(call crate-rule,      base,    libc))
$(eval $(call crate-rule,      mm,      libc base))
$(eval $(call crate-rule,      startup, $(BASIC_REQS)))
$(eval $(call crate-rule,      util,    $(BASIC_REQS)))
$(eval $(call crate-rule,      procs,   $(BASIC_REQS) startup))
$(eval $(call crate-rule,      umem,    $(BASIC_REQS) util procs))
$(eval $(call long-crate-rule, drivers, $(BASIC_REQS) procs, --opt-level=1)) # TODO For some reason LLVM doesn't like -O2 in drivers.

# The `main` crate.
$(eval $(call crate-rule, main,  $(filter-out main, $(CRATES))))

# Make the target for libm
$(eval $(call external-targets,openlibm,libopenlibm.a,libopenlibm.a,                \
	SFLAGS="-m32 -march=i386"                                                       \
   	LDFLAGS="-m32 -march=i386 -ffreestanding -nostdlib -static"                     \
	CFLAGS="-Wno-maybe-uninitialized -g3 -gdwarf-3 -m32 -march=i386 -ffreestanding" \
	ARCH=i386))

# TODO linking twice is bad and fragile.
# TODO We really should be modifying $(KERNEL) to make it's paddr's match the vaddr's in its program headers.
$(eval $(call ld-rule,   $(SYMBOLS),  $(OBJS), mk/debug.ld, ))
$(eval $(call ld-rule,   $(KERNEL),   $(OBJS), mk/link.ld,  $(KERNEL_LDFLAGS)))

$(eval $(call copy-rule, $(KERNEL),   $(ISO_KERNEL)))
$(eval $(call copy-rule, $(GRUB_CFG), $(ISO_CFG)))

$(BSYMBOLS): $(SYMBOLS)
	@ echo "[ELF ] Generating kernel symbols list..."
	$(HIDE_SIGIL) readelf -Ws $(SYMBOLS) | grep -Ev 'SECTION|UND|FILE|Num:|Symbol|^$$' | awk '{printf "0x%s %s\n", $$2, $$8}' > $@

$(ISO_IMAGE): $(ISO_KERNEL) $(ISO_CFG)
	@ echo "[GRUB] Creating \"kernel/$@\"..."
	$(HIDE_SIGIL) $(MKRESCUE) -o $@ $(ISO_DIR)

$(GDBCOMM): $(SCRIPTS)
	@ echo "[GDB ] Creating gdb command list..."
	$(HIDE_SIGIL) $(foreach script,$^,echo $(abspath $(script)) >> $@; )

# Make sure build-directory dirs are there.
$(OBJS) : | $(foreach l,$(OBJS), $(dir $(l))) $(BUILD_DIR)
$(BUILD_DIR) $(sort $(foreach l,$(OBJS), $(dir $(l)))) :
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
