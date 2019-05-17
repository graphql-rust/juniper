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

There are two general classes of release and each require running different automation commands:

1. All public workspace crates should be released and all share the same release level ("patch", "minor", "major"). _These commands take the form `release-[whatever]`._

2. A subset of workspace crates need to be released, or not all crate releases share the same release level. _These commands start with `release-skip-[whatever]`._

**All release commands must be run from the root directory of the repository.**

## Determine new release level

For each crate, determine the desired release level (`patch`, `minor`, `major`). Set the `RELEASE_LEVEL` env variable to the desired release level.

## Determine which crates to exclude

If a subset of workspace crates need to be released, or not all crate releases share the same release level, set the `CARGO_MAKE_WORKSPACE_SKIP_MEMBERS` env
variable to filter out specific workspace crates. The value is a list of semicolon-delineated crate names or a regular expression.

**Important:** You likely want to always exclude `integration_tests/*`.

## Dry run

It is a good idea to do a dry run to sanity check what actions will be performed.

- For case #1 above, run `cargo make release-dry-run`.

  If the command finishes super quickly with no output you likely did not set `RELEASE_LEVEL`.

- For case #2 above, run `cargo make release-some-dry-run`.

  If the command finishes super quickly with no output you likely did not set `RELEASE_LEVEL` or `CARGO_MAKE_WORKSPACE_SKIP_MEMBERS`.

## Local test

Not everything is captured in the dry run. It is a good idea to run a local test.
In a local test, all the release actions are performed on your local checkout
but nothing is pushed to Github or crates.io.

- For case #1 above, run `cargo make release-local-test`.

  If the command finishes super quickly with no output you likely did not set `RELEASE_LEVEL`.

- For case #2 above, run `cargo make release-some-local-test`.

  If the command finishes super quickly with no output you likely did not set `RELEASE_LEVEL` or `CARGO_MAKE_WORKSPACE_SKIP_MEMBERS`.

After, your local git repository should have the changes ready to push to Github.
Use `git rebase -i HEAD~10` and drop the new commits.

## Release

After testing locally and via a dry run, it is time to release. A release
consists of bumping versions, starting a new changelog section, pushing a tag to Github, and updating crates.io. This should all be handled by running the automated commands.

- For case #1 above, run `cargo make release`.

  If the command finishes super quickly with no output you likely did not set `RELEASE_LEVEL`.

- For case #2 above, run `cargo make release-some`.

  If the command finishes super quickly with no output you likely did not set `RELEASE_LEVEL` or `CARGO_MAKE_WORKSPACE_SKIP_MEMBERS`.

[cargo-make]: https://github.com/sagiegurari/cargo-make
[cargo-release]: https://github.com/sunng87/cargo-release
