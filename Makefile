###############################
# Common defaults/definitions #
###############################

comma := ,

# Checks two given strings for equality.
eq = $(if $(or $(1),$(2)),$(and $(findstring $(1),$(2)),\
                                $(findstring $(2),$(1))),1)




###########
# Aliases #
###########

fmt: cargo.fmt


lint: cargo.lint


test: test.cargo




##################
# Cargo commands #
##################

# Format Rust sources with rustfmt.
#
# Usage:
#	make cargo.fmt [check=(no|yes)]

cargo.fmt:
	cargo +nightly fmt --all $(if $(call eq,$(check),yes),-- --check,)


# Lint Rust sources with Clippy.
#
# Usage:
#	make cargo.lint

cargo.lint:
	cargo clippy --workspace --all-features -- -D warnings


cargo.test: test.cargo




####################
# Testing commands #
####################

# Run Rust tests of project crates.
#
# Usage:
#	make test.cargo [crate=<crate-name>]

test.cargo:
	cargo $(if $(call eq,$(crate),juniper_codegen_tests),+nightly,) test \
		$(if $(call eq,$(crate),),--workspace,-p $(crate)) --all-features




##################
# .PHONY section #
##################

.PHONY: fmt lint test \
        cargo.fmt cargo.lint \
        test.cargo
