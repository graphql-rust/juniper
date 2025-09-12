pub(crate) const INTROSPECTION_QUERY: &str = include_str!("./query.graphql");
pub(crate) const INTROSPECTION_QUERY_WITHOUT_DESCRIPTIONS: &str =
    include_str!("./query_without_descriptions.graphql");

/// Desired GraphQL introspection format for the [canonical introspection query][0].
///
/// [0]: https://github.com/graphql/graphql-js/blob/v16.11.0/src/utilities/getIntrospectionQuery.ts#L75
#[derive(Clone, Copy, Debug, Default)]
pub enum IntrospectionFormat {
    /// The canonical GraphQL introspection query.
    #[default]
    All,

    /// The canonical GraphQL introspection query without descriptions.
    WithoutDescriptions,
}
