SHELL     := /bin/sh
# We want to have this option so we can update on rustc update even if we have
# a symlinked rustc
MAKEFLAGS += "--check-symlink-times"
CC        := gcc
LD        := ld
AR        := ar
PYTHON    := python
CSCOPE    := cscope
RUST      := rustc
RUSTDOC   := rustdoc
MKRESCUE  := grub-mkrescue
RUST_FULL := $(shell which $(RUST))

RSFLAGS   += -g -Z no-landing-pads
CFLAGS    += -fno-builtin -nostdinc -std=c99 -g3 -fno-stack-protector -m32 -march=i686 -fsigned-char -Iinclude
CFLAGS    += -Wall -Wredundant-decls -Wundef -Wpointer-arith -Wfloat-equal -Wnested-externs -Wvla -Winline -Wextra -Wno-unused-parameter -Wno-unused-function -Wno-unused-variable -Wno-attributes
ASFLAGS   := -D__ASSEMBLY__

.SUFFIXES:

###

include ../Config.mk
include ../CheckTools.mk

RSFLAGS += $(foreach lint,$(ALLOW_LINTS),--allow $(lint))
RSFLAGS += $(foreach lint,$(WARN_LINTS),--warn $(lint))
RSFLAGS += $(foreach lint,$(DENY_LINTS),--deny $(lint))
RSFLAGS += $(foreach lint,$(FORBID_LINTS),--forbid $(lint))
RSFLAGS += $(foreach bool,$(COMPILE_CONFIG_BOOLS),$(if $(findstring 1,$($(bool))),--cfg $(bool),))
RDFLAGS += $(foreach bool,$(COMPILE_CONFIG_BOOLS),$(if $(findstring 1,$($(bool))),--cfg $(bool),))
RSFLAGS += $(foreach r,$(REMOVE_DBG), --cfg NDEBUG_$(r) )
RDFLAGS += $(foreach r,$(REMOVE_DBG), --cfg NDEBUG_$(r) )
RSFLAGS += $(foreach r,$(ADDITIONAL_CFGS),--cfg $(r) )
RDFLAGS += $(foreach r,$(ADDITIONAL_CFGS),--cfg $(r) )

ifneq ($(USE_STACK_CHECK),"true")
    RSFLAGS += -C no-stack-check --cfg NSTACK_CHECK
endif

CFLAGS    += $(foreach bool,$(COMPILE_CONFIG_BOOLS), \
             $(if $(findstring 1,$($(bool))),-D__$(bool)__=$(strip $($(bool)))))
CFLAGS    += $(foreach def,$(COMPILE_CONFIG_DEFS), \
             $(if $($(def)),-D__$(def)__=$(strip $($(def))),))

ifeq ("true",$(HIDE))
    HIDE_SIGIL := @
    SILENT_FLAG := --silent
    SILENT_SUFFIX := >/dev/null
else
    HIDE_SIGIL :=
    SILENT_FLAG :=
    SILENT_SUFFIX :=
endif

# Get which LIBCOMPILER_RT we will use.
ifneq ("false",$(BUILD_COMPILER_RT))
    # Build it from the rust tree. This might take a while.
    LIBCOMPILER_RT_SOURCE := $(LIBCOMPILER_RT)
else
    ifneq ("",$(COMPILER_RT_PATH))
        # Take it from the configured place
        LIBCOMPILER_RT_SOURCE := $(COMPILER_RT_PATH)
    else
        # By default we will just take libgcc.a
        LIBCOMPILER_RT_SOURCE := $(shell $(CC) $(CFLAGS) -print-libgcc-file-name)
    endif
endif

ifeq ("true",$(LD_OPT))
    LDFLAGS += --gc-sections
endif

ifneq ("",$(wildcard "$(TARGET)"))
    TARGET_FILENAME := $(TARGET)
else
    TARGET_FILENAME :=
endif
