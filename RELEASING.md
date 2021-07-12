# How to release new crate versions

## Prerequisites

It is generally best to start with a clean respository dedicated to a release so that no git weirdness happens:

```
git clone git@github.com:graphql-rust/juniper.git juniper_release;
cd juniper_release;
```

We use the `nightly` toolchain when releasing. This is because some of our crates require nightly:

`rustup default nightly`

We use [`cargo-make`](cargo-make) and [`cargo-release`](cargo-release) to automate crate releases. You will need to install them locally:

- `cargo install -f cargo-make`
- `cargo install -f cargo-release`

## Preparing for a release

There are two general classes of releases:

1. All public workspace crates should be released and all share the same release level ("patch", "minor", "major").

2. A subset of workspace crates need to be released, or not all crate releases share the same release level.

**All release commands must be run from the root directory of the repository.**

## Determine new release level

For each crate, determine the desired release level (`patch`, `minor`, `major`). Set the `RELEASE_LEVEL` env variable to the desired release level.

## Determine which crates to release

If a subset of workspace crates need to be released, or not all crate releases share the same release level, set the `CARGO_MAKE_WORKSPACE_INCLUDE_MEMBERS` env
variable to choose specific workspace crates. The value is a list of semicolon-delineated crate names or a regular expressions.

## Dry run

It is a good idea to do a dry run to sanity check what actions will be performed.

- For case #1 above, run `cargo make release-dry-run`.
- For case #2 above, run `CARGO_MAKE_WORKSPACE_INCLUDE_MEMBERS="crate1;crate2" cargo make release-dry-run`.

  If the command finishes super quickly with no output you likely did not set `RELEASE_LEVEL`.

## Local test

Not everything is captured in the dry run. It is a good idea to run a local test.
In a local test, all the release actions are performed on your local checkout
but nothing is pushed to Github or crates.io.

- For case #1 above, run `cargo make release-local-test`.
- For case #2 above, run `CARGO_MAKE_WORKSPACE_INCLUDE_MEMBERS="crate1;crate2" cargo make release-local-test`.

  If the command finishes super quickly with no output you likely did not set `RELEASE_LEVEL`.

After, your local git repository should have the changes ready to push to Github.
Use `git rebase -i HEAD~10` and drop the new commits.

## Release

After testing locally and via a dry run, it is time to release. A release
consists of bumping versions, starting a new changelog section, pushing a tag to Github, and updating crates.io. This should all be handled by running the automated commands.

- For case #1 above, run `cargo make release`.
- For case #2 above, run `CARGO_MAKE_WORKSPACE_INCLUDE_MEMBERS="crate1;crate2" cargo make release`.

  If the command finishes super quickly with no output you likely did not set `RELEASE_LEVEL`,

[cargo-make]: https://github.com/sagiegurari/cargo-make
[cargo-release]: https://github.com/sunng87/cargo-release
