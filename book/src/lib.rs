//! Doc tests of the Rust code examples in the Book.
//!
//! Each Book chapter is the documentation of one module of this crate. `cargo test` runs the Rust
//! code examples of all the chapters with the dependencies of this crate. Clippy also checks the
//! Markdown of the chapters with its documentation lints.

#![doc(test(no_crate_inject))]

use anyhow as _;
use dataloader as _;
use derive_more as _;
use futures as _;
use jiff as _;
use juniper as _;
use juniper_subscriptions as _;
use serde_json as _;
use tokio as _;

/// Includes each listed chapter as the documentation of a new module.
///
/// Chapter paths are relative to the `src` directory of the Book.
macro_rules! chapters {
    ($($module:ident => $path:literal),* $(,)?) => {
        $(
            #[doc = include_str!($path)]
            pub mod $module {}
        )*

        /// Paths of all the included chapters.
        #[cfg(test)]
        const CHAPTERS: &[&str] = &[$($path),*];
    };
}

chapters! {
    introduction => "introduction.md",
    quickstart => "quickstart.md",
    types => "types/index.md",
    types_objects => "types/objects/index.md",
    types_objects_complex_fields => "types/objects/complex_fields.md",
    types_objects_context => "types/objects/context.md",
    types_objects_error => "types/objects/error/index.md",
    types_objects_error_field => "types/objects/error/field.md",
    types_objects_error_schema => "types/objects/error/schema.md",
    types_objects_generics => "types/objects/generics.md",
    types_interfaces => "types/interfaces.md",
    types_unions => "types/unions.md",
    types_enums => "types/enums.md",
    types_input_objects => "types/input_objects.md",
    types_scalars => "types/scalars.md",
    schema => "schema/index.md",
    schema_subscriptions => "schema/subscriptions.md",
    schema_introspection => "schema/introspection.md",
    schema_limits => "schema/limits.md",
    serve => "serve/index.md",
    serve_batching => "serve/batching.md",
    advanced => "advanced/index.md",
    advanced_implicit_and_explicit_null => "advanced/implicit_and_explicit_null.md",
    advanced_n_plus_1 => "advanced/n_plus_1.md",
    advanced_dataloader => "advanced/dataloader.md",
    advanced_lookahead => "advanced/lookahead.md",
    advanced_eager_loading => "advanced/eager_loading.md",
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::CHAPTERS;

    /// Returns the paths of the chapters linked from the `summary` content of a `SUMMARY.md` file.
    ///
    /// The `SUMMARY.md` file of mdBook contains only chapter links. The function thus only searches
    /// for the `](path.md)` part of Markdown links.
    fn linked_chapters(summary: &str) -> BTreeSet<&str> {
        summary
            .split("](")
            .skip(1)
            .filter_map(|link| link.split_once(')'))
            .map(|(path, _)| path)
            .filter(|path| path.ends_with(".md"))
            .collect()
    }

    #[test]
    fn includes_every_chapter_of_summary() {
        assert_eq!(
            CHAPTERS.iter().copied().collect::<BTreeSet<_>>(),
            linked_chapters(include_str!("SUMMARY.md")),
            "`chapters!` in `book/src/lib.rs` must list the same chapters as `SUMMARY.md`",
        );
    }

    #[test]
    fn finds_nested_chapter_links() {
        let summary = "# Summary\n\n- [Intro](intro.md)\n    - [`null`](types/null.md) \n";

        assert_eq!(
            linked_chapters(summary),
            BTreeSet::from(["intro.md", "types/null.md"]),
        );
    }

    #[test]
    fn ignores_links_to_non_chapters() {
        let summary = "- [Draft]()\n- [Site](https://example.com)\n";

        assert!(linked_chapters(summary).is_empty());
    }
}
