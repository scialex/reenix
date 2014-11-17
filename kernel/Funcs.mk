# A bunch of reenix make functions.

# Get an doc name from the crate name
# $(1) is the name of the crate
define doc-name
docs/$(1)/index.html
endef

# Get an dylib name from the crate name
# $(1) is the name of the crate
# TODO This should be less fragile.
define dylib-name
libs/lib$(1).so
endef

# Get an rlib name from the crate name
# $(1) is the name of the crate
define rlib-name
libs/lib$(1).rlib
endef

define set-crate-name
$(eval $(1)_CRATE := $(2))
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
	$$(HIDE_SIGIL) $$(MAKE) $(SILENT_FLAG) $$(MFLAGS) -C external/$(1) $(2) $(4)

.PHONEY:
clean-$(1):
	$$(HIDE_SIGIL) rm -f libs/$(notdir $(3)) 2>/dev/null
	$$(HIDE_SIGIL) $$(MAKE) $$(MFLAGS) $(SILENT_FLAG) -C external/$(1) clean $(4)
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
	$$(HIDE_SIGIL) $$(RUSTDOC) $$(RDFLAGS) $(5) --crate-name $(2) --output docs $(1)/lib.rs

endef

# A Crate with custom flags
# $(1) is the name of the crate
# $(2) is the list of dependencies
# $(3) is a list of custom rust flags
define long-crate-rule
$(eval $(call base-crate-rule,$(1),$(1),$(2) $$(PLUGINS),$(3) $$(KERNEL_RSFLAGS),$$(KERNEL_RDFLAGS)))
endef

# A Crate from reenix
# $(1) is the name of the crate
# $(2) is the list of dependencies
define crate-rule
$(eval $(call long-crate-rule,$(1),$(2),--opt-level=$$(DEFAULT_CRATE_OPT)))
endef

# A module that is part of rusts stdlib.
# $(1) is the name of the crate.
# $(2) is the list of dependencies
define builtin-crate-rule
$(eval $(call base-crate-rule,rustlibs/lib$(1),$(1),$(2),$$(KERNEL_RSFLAGS) --allow=dead-code --opt-level=$$(DEFAULT_BUILTIN_CRATE_OPT),$$(KERNEL_RDFLAGS) ))
endef

# A plugin
# $(1) is the name of the plugin
# $(2) is the list of dependencies.
define plugin-rule
$(eval $(call base-crate-rule,plugins/$(1),$(1),$(2),,,))
endef
