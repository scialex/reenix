
RUST_VERSION := $(shell $(RUST) --version | cut -d - -f1)
RUST_NAME := $(firstword $(RUST_VERSION))
RUST_NUMBER := $(subst ., ,$(lastword $(RUST_VERSION)))
RUST_MAJOR := $(word 1,$(RUST_NUMBER))
RUST_MINOR := $(word 2,$(RUST_NUMBER))
RUST_PATCH := $(word 3,$(RUST_NUMBER))

ifneq ($(RUST_NAME),rustc)
$(error Found $(RUST_NAME) but expected rustc)
else
ifneq ($(RUST_MAJOR),0)
$(error Found version $(RUST_MAJOR) expected 0)
else
ifneq ($(RUST_MINOR),13)
$(error Found $(RUST) version $(RUST_MINOR) expected 13)
else
#$(info Using $(RUST) version '$(RUST_VERSION)')
endif
endif
endif
