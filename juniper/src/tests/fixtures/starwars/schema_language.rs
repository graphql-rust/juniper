#![allow(missing_docs)]

/// The schema as a static/hardcoded GraphQL SDL (schema definition language).
pub const STATIC_GRAPHQL_SCHEMA_DEFINITION: &str = include_str!("starwars.graphql");

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use crate::{
        schema::model::RootNode,
        tests::fixtures::starwars::{
            schema::{Database, Query},
            schema_language::STATIC_GRAPHQL_SCHEMA_DEFINITION,
        },
        types::scalars::{EmptyMutation, EmptySubscription},
    };

    #[test]
    fn dynamic_schema_language_matches_static() {
        let schema = RootNode::new(
            Query,
            EmptyMutation::<Database>::new(),
            EmptySubscription::<Database>::new(),
        );

        //dbg!("{}", schema.as_sdl());

        // `include_str()` keeps line endings. `git` will sadly by default
        // convert them, making this test fail without runtime tweaks on
        // Windows.
        //
        // See https://github.com/rust-lang/rust/pull/63681.
        #[cfg(windows)]
        let expected = STATIC_GRAPHQL_SCHEMA_DEFINITION.replace("\r\n", "\n");
        #[cfg(not(windows))]
        let expected = STATIC_GRAPHQL_SCHEMA_DEFINITION;

        assert_eq!(schema.as_sdl(), expected);
    }
}
