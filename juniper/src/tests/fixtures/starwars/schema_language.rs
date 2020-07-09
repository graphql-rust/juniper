/// The schema as a static/hardcoded GraphQL Schema Language.
pub const STATIC_GRAPHQL_SCHEMA_DEFINITION: &str = include_str!("starwars.graphql");

#[cfg(test)]
mod tests {
    use crate::{
        schema::model::RootNode,
        tests::fixtures::starwars::{
            model::Database, schema::Query, schema_language::STATIC_GRAPHQL_SCHEMA_DEFINITION,
        },
        types::scalars::{EmptyMutation, EmptySubscription},
    };
    use pretty_assertions::assert_eq;

    #[test]
    fn dynamic_schema_language_matches_static() {
        let schema = RootNode::new(
            Query,
            EmptyMutation::<Database>::new(),
            EmptySubscription::<Database>::new(),
        );

        dbg!("{}", schema.as_schema_language());

        assert_eq!(
            &schema.as_schema_language(),
            STATIC_GRAPHQL_SCHEMA_DEFINITION,
        );
    }
}
