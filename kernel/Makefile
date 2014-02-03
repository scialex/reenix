CFLAGS    := -ffreestanding
LDFLAGS   := -m elf_i386 -z nodefaultlib
# XXX should have --omagic?

include ../Global.mk

CFLAGS    += -D__KERNEL__

###

HEAD      := $(wildcard include/*/*.h include/*/*/*.h)
SRCDIR    := main boot util drivers/disk drivers/tty drivers mm proc fs/ramfs fs/s5fs fs vm api test test/kshell entry test/vfstest
SRC       := $(foreach dr, $(SRCDIR), $(wildcard $(dr)/*.[cS]))
OBJS      := $(addsuffix .o,$(basename $(SRC)))
SCRIPTS   := $(foreach dr, $(SRCDIR), $(wildcard $(dr)/*.gdb $(dr)/*.py))

# List of prebuilt modules that do not include the source
PREBUILT  :=

ifneq "$(shell which mkisofs 2>/dev/null)" ""
    MAKEISO := mkisofs
else
    MAKEISO := genisoimage
endif

SYMBOLS   := weenix.dbg
BSYMBOLS  := symbols.dbg
KERNEL    := kernel.bin
IMAGE     := weenix.img
ISO_IMAGE := weenix.iso
GDBCOMM   := gdb-commands

.PHONY: all cscope clean

%.a:
	touch $@

all: $(BSYMBOLS) $(ISO_IMAGE) $(GDBCOMM)

$(SYMBOLS): $(OBJS) $(PREBUILT)
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

%.o: %.c 
	@ echo "  Compiling \"kernel/$<\"..."
	@ $(CC) -c $(CFLAGS) $< -o $@

%.o: %.S
	@ echo "  Compiling \"kernel/$<\"..."
	@ $(CC) -c $(ASFLAGS) $(CFLAGS) $< -o $@

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

clean:
	@ rm -f $(OBJS) $(SYMBOLS) $(BSYMBOLS) $(KERNEL) $(IMAGE) $(ISO_IMAGE) $(GDBCOMM) */*.gdbcomm cscope*.out cscope.files
