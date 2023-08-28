/// From <https://github.com/graphql/graphql-js/blob/90bd6ff72625173dd39a1f82cfad9336cfad8f65/src/utilities/getIntrospectionQuery.ts#L62>
pub(crate) const INTROSPECTION_QUERY: &str = include_str!("./query.graphql");
pub(crate) const INTROSPECTION_QUERY_WITHOUT_DESCRIPTIONS: &str =
    include_str!("./query_without_descriptions.graphql");

/// The desired GraphQL introspection format for the canonical query
/// (<https://github.com/graphql/graphql-js/blob/90bd6ff72625173dd39a1f82cfad9336cfad8f65/src/utilities/getIntrospectionQuery.ts#L62>)
#[derive(Clone, Copy, Debug, Default)]
pub enum IntrospectionFormat {
    /// The canonical GraphQL introspection query.
    #[default]
    All,

    /// The canonical GraphQL introspection query without descriptions.
    WithoutDescriptions,
}
