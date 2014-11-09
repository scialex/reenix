
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
	@ rm *.log *.aux || true

clean: tidy
	@ rm $(DOCS) || true

