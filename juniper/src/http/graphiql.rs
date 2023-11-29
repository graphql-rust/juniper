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
    let subscriptions_endpoint = if let Some(sub_url) = subscriptions_endpoint_url {
        sub_url
    } else {
        ""
    };

    include_str!("graphiql.html").replace(
        "<!-- inject -->",
        &format!(
            // language=JavaScript
            "
      var JUNIPER_URL = '{graphql_url}';
      var JUNIPER_SUBSCRIPTIONS_URL = '{graphql_subscriptions_url}';

{grahiql_js}

            ",
            graphql_url = graphql_endpoint_url,
            graphql_subscriptions_url = subscriptions_endpoint,
            grahiql_js = include_str!("graphiql.js"),
        ),
    )
}
