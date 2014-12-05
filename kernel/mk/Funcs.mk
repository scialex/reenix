# A bunch of reenix make functions.

# Get an doc name from the crate name
# $(1) is the name of the crate
define doc-name
$(DOC_DIR)/$(1)/index.html
endef

# $(1) is the name of the crate
# $(2) is the directory it is in.
define set-base-crate-name
$(eval $(1)_LIB := $(BUILD_DIR)/libs/$(shell $(RUST) --print-file-name $(2)/lib.rs))
endef

define set-lib-name
$(eval $(1)_LIB := $(BUILD_DIR)/libs/$(2))
endef

define set-builtin-crate-name
$(call set-base-crate-name,$(1),$(RUST_SOURCE_DIR)/src/lib$(1))
endef

define set-plugin-crate-name
$(call set-base-crate-name,$(1),plugins/$(1))
endef

define set-crate-name
$(call set-base-crate-name,$(1),$(1))
endef

# Get the file name of a crate name
# $(1) is the name of the crate
define lib-name
$($(1)_LIB)
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
$(1) : $(2) $(3) | $$(dir $(1))
	@ echo "[LD  ] Linking for \"kernel/$$@\"..."
ifeq (,$(3))
	$$(HIDE_SIGIL) $$(LD) $$(LDFLAGS) $(4) $(2) -o $$@
else
	$$(HIDE_SIGIL) $$(LD) -T $(3) $$(LDFLAGS) $(4) $(2) -o $$@
endif
endef

# invoke the archiver
define ar-rule
$(1) : $(2)
	@ echo "[AR  ] Archiving for \"kernel/$$@\"..."
	$$(HIDE_SIGIL) $$(AR) qsc $$@ $$<
endef

# run ./configure on a target
# $(1) what the directory is
# $(2) configure flags
# $(3) what it generates
define configure-targets
$(addprefix $(1)/,$(3)) : $(1)/configure
	@ echo "[CONF] configuring \"kernel/$$@\"..."
	$$(HIDE_SIGIL) cd $(1); ./configure $(2) $(SILENT_SUFFIX)
endef

# Get rules for external targets.
# $(1) is the name of the target under external
# $(2) is the name of the target we wish to build in external.
# $(3) is the name of the file we wish to take into $(BUILD_DIR)
# $(4) is the source-dir where we should look for changed files.
# $(5) are any flags we wish to pass down.
# $(6) are any additional prereqs we wish to give
define external-targets

$(call copy-rule,external/$(1)/$(3),$$(BUILD_DIR)/external/$(notdir $(3)))

./external/$(strip $(1))/$(strip $(3)) : $$(shell find ./external/$(strip $(1))/$(strip $(4)) -type f -not -path ./external/$(strip $(1))/$(strip $(3)) -not -name "* *") $(strip $(6))
	@ echo "[MAKE] Recursive make of \"kernel/$$@\"..."
	$$(HIDE_SIGIL) $$(MAKE) HIDE_SIGIL=$$(HIDE_SIGIL) $$(MFLAGS) --no-print-directory -C external/$(strip $(1)) $(strip $(2)) $(strip $(5))

.PHONEY:
clean-$(strip $(1)):
	$$(HIDE_SIGIL) rm -f $$(BUILD_DIR)/external/$(notdir $(strip $(3))) 2>/dev/null
	$$(HIDE_SIGIL) $$(MAKE) $$(MFLAGS) $$(SILENT_FLAG) -C external/$(strip $(1)) clean $(strip $(5))
endef

# Make rules to build a crate
# $(1) is the directory the library is in
# $(2) is the name of the crate
# $(3) is the list of crates that this library depends on.
# $(4) is any rust flags you want.
# $(5) is any rustdoc flags you want.
define base-crate-rule
$(call lib-name,$(2)) : $$(shell find $(1) -type f -name "*.rs") \
                          $$(foreach l,$(3), $$(call lib-name,$$(l))) \
                        | $$(dir $$(call lib-name,$(2)))
	@ echo "[RS  ] Compiling \"kernel/$(1)/lib.rs\"..." # for \"kernel/$$@\""
	$$(HIDE_SIGIL) $$(RUST) $(4) $(1)/lib.rs --out-dir $$(BUILD_DIR)/libs

$(call doc-name,$(2)) : $$(shell find $(1) -type f -name "*.rs") \
						$$(foreach l,$(3), $$(call lib-name,$$(l)))
	@ echo "[RDOC] Documenting \"kernel/$(1)\"..."
	$$(HIDE_SIGIL) $$(RUSTDOC) $(5) --output $(DOC_DIR) $(1)/lib.rs

endef

define kernel-crate-rule
$(eval $(call base-crate-rule,$(1),$(2),$(3),$(4) $$(RSFLAGS),$(5) $$(RDFLAGS)))
endef

# A Crate with custom flags
# $(1) is the name of the crate
# $(2) is the list of dependencies
# $(3) is a list of custom rust flags
define long-crate-rule
$(eval $(call kernel-crate-rule,$(strip $(1)),$(strip $(1)),$(2) $$(PLUGINS),$(3) $$(KERNEL_RSFLAGS),$$(KERNEL_RDFLAGS)))
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
# $(3) is a list of custom rust flags
define long-builtin-crate-rule
$(eval $(call kernel-crate-rule,$$(RUST_SOURCE_DIR)/src/lib$(strip $(1)),$(strip $(1)),$(2),$$(KERNEL_RSFLAGS) --opt-level=$$(DEFAULT_BUILTIN_CRATE_OPT) $(strip $(3)),$$(KERNEL_RDFLAGS) ))
endef

# A module that is part of rusts stdlib.
# $(1) is the name of the crate.
# $(2) is the list of dependencies
define builtin-crate-rule
$(eval $(call long-builtin-crate-rule,$(1),$(2), --allow=dead-code))
endef

# A module that is part of rusts stdlib but patched.
# $(1) is the name of the crate.
# $(2) is the path it is at.
# $(3) is the list of dependencies
define other-crate-rule
$(eval $(call kernel-crate-rule,$(strip $(2)),$(strip $(1)),$(3), --allow=dead-code $$(KERNEL_RSFLAGS) --opt-level=$$(DEFAULT_BUILTIN_CRATE_OPT),$$(KERNEL_RDFLAGS) ))
endef

# A plugin
# $(1) is the name of the plugin
# $(2) is the list of dependencies.
define plugin-rule
$(eval $(call base-crate-rule,plugins/$(strip $(1)),$(strip $(1)),$(2),,,))
endef

