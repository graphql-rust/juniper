Releasing new crate versions
============================

A [workspace] crate is released by pushing its Git release tag in the `<crate-name>-v<version>` format. The [CI pipeline] of the tag creates a [GitHub release] and publishes the crate to [crates.io] with [trusted publishing].




## Prerequisites

We use [`cargo-release`] to automate crate releases. You will need to install it locally:
```bash
cargo install cargo-release
```

[`cargo-release`] pushes the release commit and the release tag to the `origin` remote. Run the release from a clone with `graphql-rust/juniper` as its `origin` remote:
```bash
git clone git@github.com:graphql-rust/juniper.git juniper-release
cd juniper-release
```

The release commit goes directly to the `master` branch. Branch protection lets only admins of the repository push to `master` directly.




## Order of releases

Only one [workspace] crate may be released at a time. Release the crates in the order of their dependencies:
1. `juniper_codegen`
2. `juniper`
3. `juniper_subscriptions`
4. `juniper_graphql_ws`
5. `juniper_actix`, `juniper_axum`, `juniper_hyper`, `juniper_rocket`, and `juniper_warp`

The release of a crate updates the version requirements of the [workspace] crates that depend on it. Wait until [crates.io] has the new version of a crate before you release the crates that depend on it. Publishing a crate fails otherwise.




## Preparing

To produce a new release of a [workspace] crate, perform the following steps:

1. Check its `CHANGELOG.md` file to be complete and correctly formatted. The section for the new release __must start__ with the `## master` header. The release replaces only this header with the header of the new version. A new minor version of `juniper` needs a "Switched to 0.X version of [`juniper` crate]." entry in the changelog of each crate that depends on it. Commit any changes you've made.

2. Determine a new release [bump level] (`patch`, `minor`, `major`, or default `release`).

3. Run the release process in dry-run mode and check the produced diffs to be made in the returned output.
    ```bash
    make release crate=juniper ver=minor
    ```

4. (Optional) Not everything may be captured in dry-run mode. It may be a good idea to run a local test, without pushing the created Git commit and tag.
    ```bash
    make release crate=juniper ver=minor exec=yes push=no
    ```




## Executing

Once everything is prepared and checked, just execute the releasing process:
```bash
make release crate=juniper ver=minor exec=yes
```

Once the [CI pipeline] for the pushed Git tag successfully finishes, the crate is fully released.

[`cargo-release`] writes the current UTC date into the changelog. The [CI pipeline] fails if this date differs from its own UTC date. Start a release early enough for the [CI pipeline] to finish on the same UTC day.

If the [CI pipeline] fails to publish the crate, publish it from a checkout of the release tag:
```bash
git checkout juniper-v0.18.0
cargo publish -p juniper --all-features
```




## Releasing a patch of an older version

The `master` branch may already contain breaking changes. A patch of an older version thus starts from the release tag of that version:

1. Create a branch from the release tag and add the fixes to it. Add a `## master` section with the changes to the `CHANGELOG.md` file.

2. Run the release process on the branch:
    ```bash
    make release crate=juniper ver=patch exec=yes
    ```

3. The [CI pipeline] of an old commit can fail on current toolchains. Cancel the run in that case. Then publish the crate from a checkout of the release tag, as described above, and create its [GitHub release] by hand.

4. Add the section of the new version to the `CHANGELOG.md` file on the `master` branch.




[`cargo-release`]: https://crates.io/crates/cargo-release
[`juniper` crate]: https://docs.rs/juniper
[bump level]: https://github.com/crate-ci/cargo-release/blob/master/docs/reference.md#bump-level
[CI pipeline]: /../../blob/master/.github/workflows/ci.yml
[crates.io]: https://crates.io
[GitHub release]: https://docs.github.com/repositories/releasing-projects-on-github/about-releases
[trusted publishing]: https://rust-lang.github.io/rfcs/3691-trusted-publishing-cratesio.html
[workspace]: https://doc.rust-lang.org/cargo/reference/workspaces.html
