# A bunch of reenix make functions.

# Get an doc name from the crate name
# $(1) is the name of the crate
define doc-name
$(DOC_DIR)/$(1)/index.html
endef

# $(1) is the name of the crate
# $(2) is the directory it is in.
define set-base-crate-name
$(strip $(1))_LIB := $$(BUILD_DIR)/libs/$$(shell $$(RUST) --print-file-name $(strip $(2))/lib.rs)
$(strip $(1))_DIR := $(strip $(2))
endef

define set-lib-name
$(eval $(strip $(1))_LIB := $(BUILD_DIR)/libs/$(strip $(2)))
endef

define set-builtin-crate-name
$(eval $(call set-base-crate-name,$(1),$(RUST_SOURCE_DIR)/src/lib$(1)))
endef

define set-plugin-crate-name
$(eval $(call set-base-crate-name,$(1),plugins/$(1)))
endef

define set-other-crate-name
$(eval $(call set-base-crate-name,$(1),rustlibs/$(1)))
endef

define set-patched-crate-name
$(eval $(call set-base-crate-name,$(1),rustlibs/lib$(1)))
endef

define set-crate-name
$(eval $(call set-base-crate-name,$(1),$(1)))
endef

# Get the director name of a crate name
# $(1) is the name of the crate
define dir-name
$($(strip $(1))_DIR)
endef
# Get the file name of a crate name
# $(1) is the name of the crate
define lib-name
$($(strip $(1))_LIB)
endef

# Get the compiled object's name.
# $(1) a c/S file to get the object name of
define obj-name
$(addprefix $(BUILD_DIR)/,$(addsuffix .o,$(basename $(1))))
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
ifeq ("",$(3))
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
	$$(HIDE_SIGIL) cd $(1) && ./configure $(2) $(SILENT_SUFFIX)
endef

# $(1) the objects that need directories.
define ensure-build-dir
# Make sure build-directory dirs are there.
$(1) : | $(foreach l,$(1), $(dir $(l)))
$(sort $(foreach l,$(1), $(dir $(l)))) :
	@ echo "[MKDR] Make build-directory \"kernel/$$@\"..."
	$(HIDE_SIGIL) mkdir -p $$@

endef

# Get rules for external targets.
# $(1) is the name of the target under external
# $(2) is the name of the target we wish to build in external.
# $(3) is the name of the file we wish to take into $(BUILD_DIR)
# $(4) is the source-dir where we should look for changed files.
# $(5) are any flags we wish to pass down.
# $(6) are any additional prereqs we wish to give
define external-targets

$(call copy-rule,external/$(strip $(1))/$(strip $(3)),$$(BUILD_DIR)/external/$(notdir $(strip $(3))))

./external/$(strip $(1))/$(strip $(3)) : $$(shell find ./external/$(strip $(1))/$(strip $(4)) -type f -not -path ./external/$(strip $(1))/$(strip $(3)) -not -name "* *") $(strip $(6))
	@ echo "[MAKE] Recursive make of \"kernel/$$@\"..."
	$$(HIDE_SIGIL) $$(MAKE) HIDE_SIGIL=$$(HIDE_SIGIL) $$(MFLAGS) --no-print-directory -C external/$(strip $(1)) $(strip $(2)) $(strip $(5))

.PHONEY:
clean-$(strip $(1)):
	$$(HIDE_SIGIL) rm -f $$(BUILD_DIR)/external/$(notdir $(strip $(3))) 2>/dev/null
	$$(HIDE_SIGIL) $$(MAKE) $$(MFLAGS) $$(SILENT_FLAG) -C external/$(strip $(1)) clean $(strip $(5)) 2>/dev/null || true
endef

# Make rules to build a crate
# $(1) is the name of the crate
# $(2) is the list of crates that this library depends on.
# $(3) is any rust flags you want.
# $(4) is any rustdoc flags you want.
# $(5) is any addional files to depend on
define base-crate-rule

$(call lib-name,$(1)) : $$(shell find $(call dir-name,$(1)) -type f -name "*.rs") \
                          $$(foreach l,$(2), $$(call lib-name,$$(l)))             \
						  $(5)                                                    \
                        | $$(dir $$(call lib-name,$(2)))
	@ echo "[RUST] Compiling \"kernel/$$(call dir-name,$(1))/lib.rs\"..." # for \"kernel/$$@\""
	$$(HIDE_SIGIL) $$(RUST) $$(foreach l,$(2), --extern $$(l)=$$(call lib-name,$$(l))) \
		                    $(3)                                                       \
						   	$$(call dir-name,$(1))/lib.rs                              \
						   	--out-dir $$(dir $(call lib-name,$(1)))

$(call doc-name,$(1)) : $$(shell find $(call dir-name,$(1)) -type f -name "*.rs") \
	                    $(5)                                                      \
						$$(foreach l,$(2), $$(call lib-name,$$(l)))
	@ echo "[RDOC] Documenting \"kernel/$$(call dir-name,$(1))\"..."
	$$(HIDE_SIGIL) $$(RUSTDOC) $$(foreach l,$(2),--extern $$(l)=$$(call lib-name,$$(l))) \
	                           $(4)                                                      \
							   --output $$(DOC_DIR)                                      \
							   $$(call dir-name,$(1))/lib.rs

endef

# A Crate with custom flags
# $(1) is the name of the crate
# $(2) is the list of dependencies
# $(3) is a list of custom rust flags
define long-crate-rule
$(eval $(call base-crate-rule,$(strip $(1)),$(2) $$(PLUGINS),$(3) $$(RSFLAGS),$$(RDFLAGS),$$(TARGET_FILENAME)))
endef

# A Crate
# $(1) is the name of the crate
# $(2) is the list of dependencies
define crate-rule
$(eval $(call long-crate-rule,$(strip $(1)),$(sort $(2)),--opt-level=$$(DEFAULT_CRATE_OPT)))
endef

# A plugin
# $(1) is the name of the plugin
# $(2) is the list of dependencies.
define plugin-rule
$(eval $(call base-crate-rule,$(strip $(1)),$(2),,,,))
endef

