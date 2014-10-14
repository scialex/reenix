CC        := gcc
LD        := ld
AR        := ar
PYTHON    := python
CSCOPE    := cscope
RUST      ?= rustc

RSFLAGS   += -g --target=i686-unknown-linux-gnu -Z no-landing-pads
CFLAGS    += -fno-builtin -nostdinc -std=c99 -g3 -gdwarf-3 -fno-stack-protector -m32 -march=i686 -fsigned-char -Iinclude
CFLAGS    += -Wall -Wredundant-decls -Wundef -Wpointer-arith -Wfloat-equal -Wnested-externs -Wvla -Winline -Wextra -Wno-unused-parameter -Wno-unused-function -Wno-unused-variable -Wno-attributes
ASFLAGS   := -D__ASSEMBLY__

###

include ../Config.mk
include ../CheckTools.mk

RSFLAGS   += $(foreach bool,$(COMPILE_CONFIG_BOOLS),$(if $(findstring 1,$($(bool))),--cfg $(bool),))
RSFLAGS   += $(foreach r,$(REMOVE_DBG),--cfg $(r) )

CFLAGS    += $(foreach bool,$(COMPILE_CONFIG_BOOLS), \
             $(if $(findstring 1,$($(bool))),-D__$(bool)__=$(strip $($(bool)))))
CFLAGS    += $(foreach def,$(COMPILE_CONFIG_DEFS), \
             $(if $($(def)),-D__$(def)__=$(strip $($(def))),))
