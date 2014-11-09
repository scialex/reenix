
include ../Global.mk
DOCS := design.pdf

.PHONY: all clean tidy

all: $(DOCS)
	@ echo "[MAKE] Finished building \"doc\"..."

%.pdf : %.tex
	@ echo "[TEX ] Building \"doc/$@\"..."
	$(HIDE_SIGIL) pdflatex -halt-on-error $< $(SILENT_SUFFIX)
	$(HIDE_SIGIL) pdflatex -halt-on-error $< $(SILENT_SUFFIX)

tidy:
	@ rm -f *.log *.aux 2>/dev/null

clean: tidy
	@ rm -f $(DOCS) 2>/dev/null

