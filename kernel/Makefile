CFLAGS    := -ffreestanding
RSFLAGS   := -L libs -C no-redzone -Z gc
LDFLAGS   := -m elf_i386 -z nodefaultlib
KERNEL_LDFLAGS := --strip-all

include ../Global.mk
include Funcs.mk

CFLAGS    += -D__KERNEL__
RSFLAGS   += --cfg kernel

ifeq ("true",$(LD_OPT))
LDFLAGS += --gc-sections
endif

###

HEAD      := $(wildcard include/*/*.h include/*/*/*.h)
# TODO I need to remove these at some point.
SRCDIR    := main boot util mm entry procs
SRC       := $(foreach dr, $(SRCDIR), $(wildcard $(dr)/*.[cS])) drivers/pci.c drivers/blockdev/disk/ata.c base/timer.c
OBJS      := $(addsuffix .o,$(basename $(SRC))) libs/libreenix.a
SCRIPTS   := $(foreach dr, $(SRCDIR), $(wildcard $(dr)/*.gdb $(dr)/*.py))

CRATES := base mm alloc libc main startup collections procs drivers util
BUILT_CRATES := $(foreach dr, $(CRATES), $(call rlib-name,$(dr)))

RSFLAGS += $(foreach dr, $(CRATES), --extern $(dr)=$(call rlib-name,$(dr)))

# List of prebuilt modules that do not include the source
PREBUILT  :=

# List of external modules for various things (Such as a small libm).
EXTERNAL_LIBS  := openlibm
EXTERNAL := $(foreach l, $(EXTERNAL_LIBS), libs/lib$(l).a)

SYMBOLS    := weenix.dbg
BSYMBOLS   := symbols.dbg
KERNEL     := kernel.bin
IMAGE      := weenix.img
ISO_IMAGE  := weenix.iso
GDBCOMM    := gdb-commands
ISO_DIR    := .iso

ISO_CFG    := $(ISO_DIR)/boot/grub/grub.cfg
ISO_KERNEL := $(ISO_DIR)/boot/$(KERNEL)

RSFLAGS := $(strip $(RSFLAGS))

.PHONY: all cscope clean tidy

.DEFAULT_GOAL := all
all: $(SYMBOLS) $(BSYMBOLS) $(ISO_IMAGE) $(GDBCOMM)
	@ echo "[MAKE] Finished building \"kernel\"..."

$(eval $(call builtin-crate-rule,libc,))
$(eval $(call builtin-crate-rule,collections,alloc libc))
$(eval $(call builtin-crate-rule,alloc,mm libc))

$(eval $(call crate-rule,base,libc))
$(eval $(call crate-rule,mm,base libc))
$(eval $(call crate-rule,startup,alloc mm base libc collections))
$(eval $(call crate-rule,procs,startup alloc base libc mm collections))
$(eval $(call crate-rule,util,alloc base libc mm collections))
# TODO For some reason LLVM doesn't like --opt-level=2 in drivers.
$(eval $(call long-crate-rule,drivers,procs alloc mm collections base,--opt-level=1))

$(eval $(call crate-rule,main,$(filter-out main,$(CRATES))))

# Make the target for libm
$(eval $(call external-targets,openlibm,libopenlibm.a,libopenlibm.a,SFLAGS="-m32 -march=i386" LDFLAGS="-m32 -march=i386 -ffreestanding -nostdlib -static" CFLAGS="-Wno-maybe-uninitialized -g3 -gdwarf-3 -m32 -march=i386 -ffreestanding" ARCH=i386))

$(KERNEL): $(OBJS) $(PREBUILT) $(EXTERNAL)
	@ echo "[LD  ] Linking for \"kernel/$@\"..."
	@ # boot/boot.S must be the first file so that the multiboot header is close enough to the front.
	$(HIDE_SIGIL) $(LD) $(LDFLAGS) $(KERNEL_LDFLAGS) -T link.ld boot/boot.o $(filter-out boot/boot.o,$^) -o $@

$(BSYMBOLS): $(SYMBOLS)
	@ echo "[ELF ] Generating kernel symbols list..."
	$(HIDE_SIGIL) readelf -Ws $(SYMBOLS) | grep -Ev 'SECTION|UND|FILE|Num:|Symbol|^$$' | awk '{printf "0x%s %s\n", $$2, $$8}' > $@

$(SYMBOLS): $(OBJS) $(PREBUILT) $(EXTERNAL)
	@ echo "[LD  ] Linking for \"kernel/$@\"..."
	@ # TODO This is bad and fragile. We really should be modifying $(KERNEL) to make it's paddr's match the vaddr's in its program headers.
	@ # boot/boot.S must be the first file so that the multiboot header is close enough to the front.
	$(HIDE_SIGIL) $(LD) $(LDFLAGS) -T debug.ld boot/boot.o $(filter-out boot/boot.o,$^) -o $@

$(ISO_CFG): boot/grub.cfg
	@ echo "[CP  ] Copying \"kernel/$@\"..."
	$(HIDE_SIGIL) mkdir -p $(dir $@)
	$(HIDE_SIGIL) cp $< $@

$(ISO_KERNEL) : $(KERNEL)
	@ echo "[CP  ] Copying \"kernel/$@\"..."
	$(HIDE_SIGIL) mkdir -p $(dir $@)
	$(HIDE_SIGIL) cp $< $@

$(ISO_IMAGE): $(ISO_KERNEL) $(ISO_CFG)
	@ echo "[GRUB] Creating \"kernel/$@\"..."
	$(HIDE_SIGIL) $(MKRESCUE) -o $@ $(ISO_DIR)

$(GDBCOMM): $(SCRIPTS)
	@ echo "[GDB ] Creating gdb command list..."
	$(HIDE_SIGIL) $(foreach script, $(SCRIPTS), echo $(abspath $(script)) >> $(dir $(script))$(shell basename $(dir $(script))).gdbcomm; )
	$(HIDE_SIGIL) cat */*.gdbcomm > $@

libs/libreenix.a : reenix.rs $(BUILT_CRATES)
	@ echo "[RS  ] Compiling \"kernel/$<\"..."
	$(HIDE_SIGIL) $(RUST) --crate-type=staticlib $(RSFLAGS) $< --out-dir=libs

%.o: %.c
	@ echo "[CC  ] Compiling \"kernel/$<\"..."
	$(HIDE_SIGIL) $(CC) -c $(CFLAGS) $< -o $@

%.o: %.S
	@ echo "[AS  ] Compiling \"kernel/$<\"..."
	$(HIDE_SIGIL) $(CC) -g -c $(ASFLAGS) $(CFLAGS) $< -o $@

include Helpers.mk
