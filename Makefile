.PHONY: all clean all_kernel all_user clean_kernel clean_user nyi tidy_kernel tidy
all: all_kernel all_user

all_kernel:
	@ $(MAKE) -C kernel $(MFLAGS) all

all_user:
	@ $(MAKE) -C user $(MFLAGS) all

clean: clean_kernel clean_user
tidy: tidy_kernel


clean_kernel:
	@ $(MAKE) -C kernel $(MFLAGS) clean

tidy_kernel:
	@ $(MAKE) -C kernel $(MFLAGS) tidy

clean_user:
	@ $(MAKE) -C user $(MFLAGS) tidy

nyi:
	@ $(MAKE) -C kernel $(MFLAGS) nyi
