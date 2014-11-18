# A bunch of reenix make functions.

# Get an doc name from the crate name
# $(1) is the name of the crate
define doc-name
docs/$(1)/index.html
endef

# $(1) is the name of the crate
# $(2) is the directory it is in.
define set-base-crate-name
$(eval $(1)_CRATE := libs/$(shell $(RUST) --print-file-name $(2)/lib.rs))
endef

define set-builtin-crate-name
$(call set-base-crate-name,$(1),rustlibs/lib$(1))
endef

define set-plugin-crate-name
$(call set-base-crate-name,$(1),plugins/$(1))
endef

define set-crate-name
$(call set-base-crate-name,$(1),$(1))
endef

# Get the file name of a crate name
# $(1) is the name of the crate
define crate-name
$($(1)_CRATE)
endef

# Get rules for external targets.
# $(1) is the name of the target under external
# $(2) is the name of the target we wish to build in external.
# $(3) is the name of the file we wish to take into libs
# $(4) are any flags we wish to pass down.
define external-targets
./libs/$(notdir $(3)) : external/$(1)/$(3)
	@ echo "[CP  ] Copying \"kernel/$$@\"..."
	$$(HIDE_SIGIL) cp external/$(1)/$(3) $$@

./external/$(1)/$(3) : $$(shell find ./external/$(1) -type f -not -path ./external/$(1)/$(3))
	@ echo "[MAKE] Recursive make of \"kernel/$$@\"..."
	$$(HIDE_SIGIL) $$(MAKE) HIDE_SIGIL=$$(HIDE_SIGIL) $$(MFLAGS) --no-print-directory -C external/$(1) $(2) $(4)

.PHONEY:
clean-$(1):
	$$(HIDE_SIGIL) rm -f libs/$(notdir $(3)) 2>/dev/null
	$$(HIDE_SIGIL) $$(MAKE) $$(MFLAGS) $$(SILENT_FLAG) -C external/$(1) clean $(4)
endef

# Make rules to build a crate
# $(1) is the directory the library is in
# $(2) is the name of the crate
# $(3) is the list of crates that this library depends on.
# $(4) is any additional rust flags you want.
# $(5) is any additional rustdoc flags you want.
define base-crate-rule
$(call crate-name,$(2)) : $$(shell find $(1) -type f -name "*.rs") $$(foreach l,$(3), $$(call crate-name,$$(l)))
	@ echo "[RS  ] Compiling \"kernel/$(1)/lib.rs\"..." # for \"kernel/$$@\""
	$$(HIDE_SIGIL) $$(RUST) $$(RSFLAGS) $(4) $(1)/lib.rs --out-dir libs

$(call doc-name,$(2)) : $$(shell find $(1) -type f -name "*.rs") $$(foreach l,$(3), $$(call crate-name,$$(l)))
	@ echo "[RDOC] Documenting \"kernel/$(1)\"..."
	$$(HIDE_SIGIL) $$(RUSTDOC) $$(RDFLAGS) $(5) --output docs $(1)/lib.rs

endef

# A Crate with custom flags
# $(1) is the name of the crate
# $(2) is the list of dependencies
# $(3) is a list of custom rust flags
define long-crate-rule
$(eval $(call base-crate-rule,$(strip $(1)),$(strip $(1)),$(2) $$(PLUGINS),$(3) $$(KERNEL_RSFLAGS),$$(KERNEL_RDFLAGS)))
endef

# A Crate from reenix
# $(1) is the name of the crate
# $(2) is the list of dependencies
define crate-rule
$(eval $(call long-crate-rule,$(strip $(1)),$(2),--opt-level=$$(DEFAULT_CRATE_OPT)))
endef

# A module that is part of rusts stdlib.
# $(1) is the name of the crate.
# $(2) is the list of dependencies
define builtin-crate-rule
$(eval $(call base-crate-rule,rustlibs/lib$(strip $(1)),$(strip $(1)),$(2),$$(KERNEL_RSFLAGS) --allow=dead-code --opt-level=$$(DEFAULT_BUILTIN_CRATE_OPT),$$(KERNEL_RDFLAGS) ))
endef

# A plugin
# $(1) is the name of the plugin
# $(2) is the list of dependencies.
define plugin-rule
$(eval $(call base-crate-rule,plugins/$(strip $(1)),$(strip $(1)),$(2),,,))
endef

# a rule that copies a file.
# $(1) is the source
# $(2) is the destination
define copy-rule
$(2) : $(1)
	@ echo "[CP  ] Copying \"kernel/$$@\"..."
	$$(HIDE_SIGIL) mkdir -p $$(dir $$@)
	$$(HIDE_SIGIL) cp $$< $$@
endef

# invoke the linker
# $(1) is the file to link
# $(2) is the list of all files to give the linker
# $(3) is the ld to give the linker
# $(4) is the extra ldflags to pass
define ld-rule
$(1) : $(2) $(3)
	@ echo "[LD  ] Linking for \"kernel/$$@\"..."
ifeq ("",$(3))
	$$(HIDE_SIGIL) $$(LD) $$(LDFLAGS) $(4) $(2) -o $$@
else
	$$(HIDE_SIGIL) $$(LD) -T $(3) $$(LDFLAGS) $(4) $(2) -o $$@
endif
endef

