//! Utility module to generate a GraphQL Playground interface.

/// Generate the HTML source to show a GraphQL Playground interface.
pub fn playground_source(
    graphql_endpoint_url: &str,
    subscriptions_endpoint_url: Option<&str>,
) -> String {
    let subscriptions_endpoint = if let Some(sub_url) = subscriptions_endpoint_url {
        sub_url
    } else {
        graphql_endpoint_url
    };

    include_str!("playground.html")
        .replace("JUNIPER_URL", graphql_endpoint_url)
        .replace("JUNIPER_SUBSCRIPTIONS_URL", subscriptions_endpoint)
}
