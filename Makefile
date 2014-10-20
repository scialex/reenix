.PHONY: all clean all_kernel all_user clean_kernel clean_user nyi tidy_kernel tidy all_doc clean_doc tidy_doc
all: all_kernel all_user all_doc

all_kernel:
	@ $(MAKE) -C kernel $(MFLAGS) all

all_user:
	@ $(MAKE) -C user $(MFLAGS) all

clean: clean_kernel clean_user clean_doc
tidy: tidy_kernel tidy_doc

%.pdf : %.tex
	@ echo " Building $@ document"
	@ pdflatex -halt-on-error $< >/dev/null
	@ pdflatex -halt-on-error $< >/dev/null

all_doc: design.pdf

tidy_doc:
	@ rm design.log 2>/dev/null || true
	@ rm design.aux 2>/dev/null || true

clean_doc: tidy_doc
	@ rm design.pdf

clean_kernel:
	@ $(MAKE) -C kernel $(MFLAGS) clean

tidy_kernel:
	@ $(MAKE) -C kernel $(MFLAGS) tidy

clean_user:
	@ $(MAKE) -C user $(MFLAGS) clean

nyi:
	@ $(MAKE) -C kernel $(MFLAGS) nyi
