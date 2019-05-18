# Juniper Contribution Guide

Juniper is always looking for new contributors, so don't be afraid to jump in and help!  
The maintainers are happy to provide guidance if required.

To get started, you can look for [issues with the "help wanted" label](https://github.com/graphql-rust/juniper/issues?q=is%3Aissue+is%3Aopen+label%3A%22help+wanted%22).

## PR Checklist

Before submitting a PR, you should follow these steps to prevent redundant churn or CI failures:

- [ ] Ensure proper formatting
- [ ] Run all tests
- [ ] Update the CHANGELOG

### Ensure proper formatting

Consistent formatting is enforced on the CI.

Before you submit your PR, you should run `cargo fmt` in the root directory.

Formatting should be run on the **stable** compiler. 
(You can do `rustup run stable cargo fmt` when developing on nightly)

### Run all tests

To run all available tests, including verifying the code examples in the book,
you can use [cargo-make](https://github.com/sagiegurari/cargo-make).

1. Install cargo-make with `cargo install cargo-make`
2. Run `cargo make ci-flow` in the root directory
   (You can do `rustup run nightly cargo make ci-flow` to run all tests when developing on stable)

### Update the CHANGELOG

Add your changes to the relevant changelog if they affect users in any way.
Each sub-crate has it's own CHANGELOG.md.

Your changes should be added to a `[master]` section on top of the file.
