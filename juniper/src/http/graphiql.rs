//! Utility module to generate a GraphiQL interface

/// Generate the HTML source to show a GraphiQL interface
///
/// The subscriptions endpoint URL can optionally be provided. For example:
///
/// ```
/// # use juniper::http::graphiql::graphiql_source;
/// let graphiql = graphiql_source("/graphql", Some("/subscriptions"));
/// ```
pub fn graphiql_source(
    graphql_endpoint_url: &str,
    subscriptions_endpoint_url: Option<&str>,
) -> String {
    include_str!("graphiql.html").replace(
        "<!-- inject -->",
        &format!(
            // language=JavaScript
            "
      var JUNIPER_URL = '{juniper_url}';
      var JUNIPER_SUBSCRIPTIONS_URL = '{juniper_subscriptions_url}';

{grahiql_js}

            ",
            juniper_url = graphql_endpoint_url,
            juniper_subscriptions_url = subscriptions_endpoint_url.unwrap_or_default(),
            grahiql_js = include_str!("graphiql.js"),
        ),
    )
}
