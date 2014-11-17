
PRINT_DIRECTORY ?= false
ifneq ($(PRINT_DIRECTORY),true)
MFLAGS += --no-print-directory
endif

.PHONY: all clean all_kernel all_user clean_kernel clean_user nyi tidy_kernel tidy docs clean_doc tidy_doc reg_doc kernel_doc
all: all_kernel all_user reg_doc
	@ echo "[MAKE] Finished building Reenix"

all_kernel:
	@ echo "[MAKE] Building \"kernel\"..."
	@ $(MAKE) -C kernel $(MFLAGS) all

all_user:
	@ echo "[MAKE] Building \"user\"..."
	@ $(MAKE) -C user $(MFLAGS) all

docs: reg_doc kernel_doc
clean: clean_kernel clean_user clean_doc
tidy: tidy_kernel tidy_doc

reg_doc:
	@ echo "[MAKE] Building \"doc\"..."
	@ $(MAKE) -C doc $(MFLAGS) all

kernel_doc:
	@ echo "[MAKE] Building \"kernel/docs\"..."
	@ $(MAKE) -C kernel $(MFLAGS) docs


tidy_doc:
	@ $(MAKE) -C doc $(MFLAGS) tidy

clean_doc:
	@ $(MAKE) -C doc $(MFLAGS) clean

clean_kernel:
	@ $(MAKE) -C kernel $(MFLAGS) clean

tidy_kernel:
	@ $(MAKE) -C kernel $(MFLAGS) tidy

clean_user:
	@ $(MAKE) -C user $(MFLAGS) clean

nyi:
	@ $(MAKE) -C kernel $(MFLAGS) nyi
