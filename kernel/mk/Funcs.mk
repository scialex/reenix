# A bunch of reenix make functions.

# Get an doc name from the crate name
# $(1) is the name of the crate
define base-doc-name
$(DOC_DIR)/$(1)/index.html
endef

# Create a local variable to be destroyed with local-var-destroy
# $(1) the name of the variable
# $(2) the initial value
define local-var-init
    ifneq (,$(value $(value 1)))
        $$(error local variable $(1) is in use and is "$($(1))"!)
    endif
    $(eval $(value 1) := $(strip $(value 2)))
endef

# destroy local variable.
define local-var-destroy
$(1) :=
endef
# destroy local variables.
define local-vars-destroy
$(foreach l,$(1),$(eval $(call local-var-destroy,$(l))))
endef

# $(1) is the name of the crate
# $(2) is the directory it is in.
define set-base-crate-name
$(call local-var-init,TMP_SBCN_FLAG,)

ifneq (,$$(findstring $(1),$$(HOST_CRATES)))
    TMP_SBCN_FLAG :=
else ifneq (,$$(findstring $(1),$$(TARGET_CRATES)))
    TMP_SBCN_FLAG := --target $(TARGET)
else
    $$(error $(1) is not given any type of crate!)
endif
$(strip $(1))_LIB  := $$(firstword $$(BUILD_DIR)/libs/$$(shell $$(RUST) $$(TMP_SBCN_FLAG) --print-file-name $(strip $(2))/lib.rs 2>/dev/null))
$(strip $(1))_DIR  := $(strip $(2))
$(strip $(1))_DOC  := $$(call base-doc-name,$(strip $(1)))
$(call local-var-destroy,TMP_SBCN_FLAG)
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
$(foreach l,$(1),$($(l)_DIR))
endef

# Get the file name of a crate name
# $(1) is the name of the crate
define lib-name
$(foreach l,$(1),$($(l)_LIB))
endef

# Get the file name of documentation for a crate
# $(1) is the name of the crate
define doc-name
$(foreach l,$(1),$($(l)_DOC))
endef

# Get the compiled object's name.
# $(1) a c/S file to get the object name of
define obj-name
$(addprefix $(BUILD_DIR)/,$(addsuffix .o,$(basename $(strip $(1)))))
endef

# a rule that copies a file.
# $(1) is the source
# $(2) is the destination
define copy-rule
$(strip $(2)) : $(strip $(1))
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
$(strip $(1)) : $(strip $(2)) $(strip $(3))
	@ echo "[LD  ] Linking for \"kernel/$$@\"..."
ifeq ("",$(3))
	$$(HIDE_SIGIL) $$(LD) $$(LDFLAGS) $(4) $(2) -o $$@
else
	$$(HIDE_SIGIL) $$(LD) -T $(3) $$(LDFLAGS) $(4) $(2) -o $$@
endif
endef

# invoke the archiver
define ar-rule
$(strip $(1)) : $(strip $(2))
	@ echo "[AR  ] Archiving for \"kernel/$$@\"..."
	$$(HIDE_SIGIL) $$(AR) rsc $$@ $$<
endef

# invoke as
# $(1) is the object to compile
# $(2) is the source files.
define as-rule
$(strip $(1)) : $(strip $(2))
	@ echo "[AS  ] Compiling \"kernel/$$<\"..."
	$$(HIDE_SIGIL) $$(CC) -c $$(ASFLAGS) $$(CFLAGS) $$< -o $$@

endef

# invoke cc
# $(1) is the object to compile
# $(2) is the source files.
define cc-rule
$(strip $(1)) : $(strip $(2))
	@ echo "[CC  ] Compiling \"kernel/$$<\"..."
	$$(HIDE_SIGIL) $$(CC) -c $$(CFLAGS) $$< -o $$@

endef

# run ./configure on a target
# $(1) what the directory is
# $(2) configure flags
# $(3) what it generates
define configure-targets
$(addprefix $(1)/,$(3)) : $(1)/configure
	@ echo "[CONF] configuring \"kernel/$$@\"..."
	$$(HIDE_SIGIL) cd $(1) && ./configure $(2) $$(SILENT_SUFFIX)
endef

# $(1) the objects that need directories.
define make-build-dir
# Make sure build-directory dirs are there.
$(1) : | $(sort $(dir $(1)))
$(sort $(dir $(1))) :
	@ echo "[MKDR] Make build-directory \"kernel/$$@\"..."
	$$(HIDE_SIGIL) mkdir -p $$@

endef

# Get rules for external targets.
# $(1) is the name of the target under external
# $(2) is the name of the target we wish to build in external.
# $(3) is the name of the file we wish to take into $(BUILD_DIR)
# $(4) is the source-dir where we should look for changed files.
# $(5) are any flags we wish to pass down.
# $(6) are any additional prereqs we wish to give
define external-targets
$(call local-var-init, TMP_EXT_TARGET,       $$(BUILD_DIR)/external/$(notdir $(strip $(3))))
$(call local-var-init, TMP_EXT_DIR,          $$(PROJECT_ROOT)/external/$(strip $(1)))
$(call local-var-init, TMP_EXT_INTERMEDIATE, $(TMP_EXT_DIR)/$(strip $(3)))
$(call local-var-init, TMP_EXT_SOURCE,       $(TMP_EXT_DIR)/$(strip $(4)))
$(call local-var-init, TMP_EXT_PREREQS,      $$(shell find $(TMP_EXT_SOURCE) -type f -not -path $(TMP_EXT_INTERMEDIATE) -not -name "* *"))

$(call copy-rule, $(TMP_EXT_INTERMEDIATE), $(TMP_EXT_TARGET))

$(TMP_EXT_INTERMEDIATE) : $(TMP_EXT_PREREQS) $(strip $(6))
	@ echo "[MAKE] Recursive make of \"kernel/$$@\"..."
	$$(HIDE_SIGIL) $$(MAKE) HIDE_SIGIL=$$(HIDE_SIGIL) $$(MFLAGS) --no-print-directory -C $(TMP_EXT_DIR) $(strip $(2)) $(strip $(5)) $$(SILENT_SUFFIX)

.PHONEY:
clean-$(strip $(1)) :
	$$(HIDE_SIGIL) $(RM) $(TMP_EXT_TARGET) 2>/dev/null
	$$(HIDE_SIGIL) $$(MAKE) $$(MFLAGS) $$(SILENT_FLAG) -C $(TMP_EXT_DIR) clean $(strip $(5)) $$(SILENT_SUFFIX) 2>/dev/null || true

$(call local-vars-destroy, TMP_EXT_DIR TMP_EXT_TARGET TMP_EXT_INTERMEDIATE TMP_EXT_SOURCE TMP_EXT_TARGET TMP_EXT_PREREQS)
endef

# Make rules to build a crate
# $(1) is the name of the crate
# $(2) is the list of crates that this library depends on.
# $(3) is any rust flags you want.
# $(4) is any rustdoc flags you want.
# $(5) is any addional files to depend on
define base-crate-rule
$(call local-var-init, TMP_BCR_RSFILES,$$(shell find $(call dir-name,$(1)) -type f -name "*.rs"))
$(call local-var-init, TMP_BCR_CRATES,)

$(call lib-name,$(1)) :  $(TMP_BCR_RSFILES) $(5) $(call lib-name,$(2))
	@ echo "[RUST] Compiling \"kernel/$$(call dir-name,$(1))/lib.rs\"..."
	$$(HIDE_SIGIL) $$(RUST) $$(foreach l,$(2), --extern $$(l)=$$(call lib-name,$$(l))) \
		                    $(3) $$(call dir-name,$(1))/lib.rs                         \
						   	--out-dir $$(dir $(call lib-name,$(1)))

$(call doc-name,$(1)) : $(TMP_BCR_RSFILES) $(5) $$(call lib-name,$(2)) | $$(call doc-name,$(2))
	@ echo "[RDOC] Documenting \"kernel/$$(call dir-name,$(1))\"..."
	$$(HIDE_SIGIL) $$(RUSTDOC) $$(foreach l,$(2),--extern $$(l)=$$(call lib-name,$$(l))) \
	                           $(4)                                                      \
							   --output $$(DOC_DIR)                                      \
							   $$(call dir-name,$(1))/lib.rs

$(call local-var-destroy, TMP_BCR_RSFILES)
$(call local-var-destroy, TMP_BCR_CRATES)
endef

# A Crate with custom flags
# $(1) is the name of the crate
# $(2) is the list of dependencies
# $(3) is a list of custom rust flags
# $(4) is a list of custom rustdoc flags
define long-crate-rule
$(eval $(call base-crate-rule,$(strip $(1)),$(2) ,$(3) $$(RSFLAGS),$(4) $$(RDFLAGS),$$(TARGET_FILENAME)))
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

