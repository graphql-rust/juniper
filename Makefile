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

book: book.build


codespell: book.codespell


fmt: cargo.fmt


lint: cargo.lint


test: test.cargo


release: cargo.release




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


# Release Rust crate.
#
# Read more about bump levels here:
#	https://github.com/crate-ci/cargo-release/blob/master/docs/reference.md#bump-level
#
# Usage:
#	make cargo.release crate=<crate-name> [ver=(release|<bump-level>)]
#	                   ([exec=no]|exec=yes [push=(yes|no)])
#	                   [install=(yes|no)]

cargo.release:
ifneq ($(install),no)
	cargo install cargo-release
endif
	cargo release -p $(crate) --all-features \
		$(if $(call eq,$(exec),yes),\
			--no-publish $(if $(call eq,$(push),no),--no-push,) --execute,\
			-v $(if $(call eq,$(CI),),,--no-publish)) \
		$(or $(ver),release)


cargo.test: test.cargo




####################
# Testing commands #
####################

# Run Rust tests of Book.
#
# Usage:
#	make test.book [clean=(no|yes)]

test.book:
ifeq ($(clean),yes)
	cargo clean
endif
	$(eval target := $(strip $(shell cargo -vV | sed -n 's/host: //p')))
	cargo build --all-features
	mdbook test book -L target/debug/deps $(strip \
		$(if $(call eq,$(findstring windows,$(target)),),,\
			$(shell cargo metadata -q \
			        | jq -r '.packages[] | select(.name == "windows_$(word 1,$(subst -, ,$(target)))_$(word 4,$(subst -, ,$(target)))") | .manifest_path' \
			        | sed -e "s/^/-L '/" -e 's/Cargo.toml/lib/' -e "s/$$/'/" )))


# Run Rust tests of project crates.
#
# Usage:
#	make test.cargo [crate=<crate-name>] [careful=(no|yes)]

test.cargo:
ifeq ($(careful),yes)
ifeq ($(shell cargo install --list | grep cargo-careful),)
	cargo install cargo-careful
endif
ifeq ($(shell rustup component list --toolchain=nightly \
              | grep 'rust-src (installed)'),)
	rustup component add --toolchain=nightly rust-src
endif
endif
	cargo $(if $(call eq,$(careful),yes),+nightly careful,\
	      $(if $(call eq,$(crate),juniper_codegen_tests),+nightly,)) \
		test $(if $(call eq,$(crate),),--workspace,-p $(crate)) --all-features




#################
# Book commands #
#################

# Build Book.
#
# Usage:
#	make book.build [out=<dir>]

book.build:
	mdbook build book/ $(if $(call eq,$(out),),,-d $(out))


# Spellcheck Book.
#
# Usage:
#	make book.codespell [fix=(no|yes)]

book.codespell:
	codespell book/ $(if $(call eq,$(fix),yes),--write-changes,)


# Serve Book on some port.
#
# Usage:
#	make book.serve [port=(3000|<port>)]

book.serve:
	mdbook serve book/ -p=$(or $(port),3000)




######################
# Forwarded commands #
######################

# Download and prepare actual version of GraphiQL static files, used for
# integrating it.
#
# Usage:
#	make graphiql

graphiql:
	@cd juniper/ && \
	make graphiql


# Download and prepare actual version of GraphQL Playground static files, used
# for integrating it.
#
# Usage:
#	make graphql-playground

graphql-playground:
	@cd juniper/ && \
	make graphql-playground




##################
# .PHONY section #
##################

.PHONY: book codespell fmt lint release test \
        book.build book.codespell book.serve \
        cargo.fmt cargo.lint cargo.release cargo.test \
        graphiql graphql-playground \
        test.book test.cargo
