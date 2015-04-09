
PRINT_DIRECTORY ?= false
ifneq ($(PRINT_DIRECTORY),true)
MFLAGS += --no-print-directory
endif

.PHONY: all clean all_kernel all_user clean_kernel clean_user nyi tidy_kernel tidy kernel_doc
all: all_kernel all_user
	@ echo "[MAKE] Finished building Reenix"

all_kernel:
	@ echo "[MAKE] Building \"kernel\"..."
	@ $(MAKE) -C kernel $(MFLAGS) all

all_user:
	@ echo "[MAKE] Building \"user\"..."
	@ $(MAKE) -C user $(MFLAGS) all

docs: kernel_doc
clean: clean_kernel clean_user
tidy: tidy_kernel

kernel_doc:
	@ echo "[MAKE] Building \"kernel/docs\"..."
	@ $(MAKE) -C kernel $(MFLAGS) docs

clean_kernel:
	@ $(MAKE) -C kernel $(MFLAGS) clean

tidy_kernel:
	@ $(MAKE) -C kernel $(MFLAGS) tidy

clean_user:
	@ $(MAKE) -C user $(MFLAGS) clean

nyi:
	@ $(MAKE) -C kernel $(MFLAGS) nyi
