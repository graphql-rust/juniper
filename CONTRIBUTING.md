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

Before you submit your PR, you should run `cargo +nightly fmt --all` in the root directory (or use the `make fmt` shortcut).

Formatting should be run on the **nightly** compiler.

### Run all tests

To run all available tests, including verifying the code examples in the book:

1. Run `cargo test` in the root directory.
2. Run `make test.book` in the root directory.

### Update the CHANGELOG

Add your changes to the relevant changelog if they affect users in any way.
Each sub-crate has it's own CHANGELOG.md.

Your changes should be added to a `[master]` section on top of the file.
