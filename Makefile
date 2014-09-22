.PHONY: all clean all_kernel all_user clean_kernel clean_user nyi
all: all_kernel all_user

all_kernel:
	@ cd kernel && $(MAKE) all

all_user:
	@ cd user && $(MAKE) all

clean: clean_kernel clean_user
tidy: tidy_kernel


clean_kernel:
	@ cd kernel && $(MAKE) clean
tidy_kernel:
	@ cd kernel && $(MAKE) tidy

clean_user:
	@ cd user && $(MAKE) clean

nyi:
	@ cd kernel && $(MAKE) nyi
