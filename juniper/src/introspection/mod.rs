/// From <https://github.com/graphql/graphql-js/blob/8c96dc8276f2de27b8af9ffbd71a4597d483523f/src/utilities/introspectionQuery.js#L21>
pub(crate) const INTROSPECTION_QUERY: &str = include_str!("./query.graphql");
pub(crate) const INTROSPECTION_QUERY_WITHOUT_DESCRIPTIONS: &str =
    include_str!("./query_without_descriptions.graphql");

/// The desired GraphQL introspection format for the canonical query
/// (<https://github.com/graphql/graphql-js/blob/8c96dc8276f2de27b8af9ffbd71a4597d483523f/src/utilities/introspectionQuery.js#L21>)
pub enum IntrospectionFormat {
    /// The canonical GraphQL introspection query.
    All,
    /// The canonical GraphQL introspection query without descriptions.
    WithoutDescriptions,
}

impl Default for IntrospectionFormat {
    fn default() -> Self {
        IntrospectionFormat::All
    }
}
