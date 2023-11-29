Releasing new crate versions
============================

Releasing of [workspace] crates of this project is performed by pushing the Git release tag (having `<crate-name>@<version>` format), following by the [CI pipeline] creating a [GitHub release] and publishing the crate to [crates.io].

> __WARNING__: Only one [workspace] crate may be released at a time. So, if you need to release multiple [workspace] crates, do this sequentially.




## Prerequisites

We use [`cargo-release`] to automate crate releases. You will need to install it locally:
```bash
cargo install cargo-release
```




## Preparing

To produce a new release a [workspace] crate, perform the following steps:

1. Check its `CHANGELOG.md` file to be complete and correctly formatted. The section for the new release __should start__ with `## master` header. Commit any changes you've made.

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




[`cargo-release`]: https://crates.io/crates/cargo-release
[CI pipeline]: /../../blob/master/.github/workflows/ci.yml
[crates.io]: https://crates.io
[GitHub release]: https://docs.github.com/repositories/releasing-projects-on-github/about-releases
[release level]: https://github.com/crate-ci/cargo-release/blob/master/docs/reference.md#bump-level
[workspace]: https://doc.rust-lang.org/cargo/reference/workspaces.html
