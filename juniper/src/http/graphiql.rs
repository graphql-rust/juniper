//! Utility module to generate a GraphiQL interface

/// Generate the HTML source to show a GraphiQL interface
///
/// The subscriptions endpoint URL can optionally be provided. For example:
///
/// ```
/// # use juniper::http::graphiql::graphiql_source;
/// let graphiql = graphiql_source("/graphql", Some("ws://localhost:8080/subscriptions"));
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

    let stylesheet_source = r#"
    <style>
        html, body, #app {
            height: 100%;
            margin: 0;
            overflow: hidden;
            width: 100%;
        }
    </style>
    "#;
    let fetcher_source = r#"
    <script>
        if (usingSubscriptions) {
            var subscriptionsClient = new window.SubscriptionsTransportWs.SubscriptionClient(GRAPHQL_SUBSCRIPTIONS_URL, { reconnect: true });
        }

        function graphQLFetcher(params) {
            return fetch(GRAPHQL_URL, {
                method: 'post',
                headers: {
                    'Accept': 'application/json',
                    'Content-Type': 'application/json',
                },
                credentials: 'include',
                body: JSON.stringify(params)
            }).then(function (response) {
                return response.text();
            }).then(function (body) {
                try {
                    return JSON.parse(body);
                } catch (error) {
                    return body;
                }
            });
        }

        var fetcher = usingSubscriptions ? window.GraphiQLSubscriptionsFetcher.graphQLFetcher(subscriptionsClient, graphQLFetcher) : graphQLFetcher;

        ReactDOM.render(
            React.createElement(GraphiQL, {
                fetcher,
            }),
            document.querySelector('#app'));
    </script>
    "#;

    format!(
        r#"
<!DOCTYPE html>
<html>
<head>
    <title>GraphQL</title>
    {stylesheet_source}
    <link rel="stylesheet" type="text/css" href="//cdn.jsdelivr.net/npm/graphiql@0.17.5/graphiql.min.css">
</head>
<body>
    <div id="app"></div>
    <script src="//cdnjs.cloudflare.com/ajax/libs/fetch/2.0.3/fetch.js"></script>
    <script src="//unpkg.com/subscriptions-transport-ws@0.8.3/browser/client.js"></script>
    <script src="//unpkg.com/graphiql-subscriptions-fetcher@0.0.2/browser/client.js"></script>
    <script src="//cdnjs.cloudflare.com/ajax/libs/react/16.10.2/umd/react.production.min.js"></script>
    <script src="//cdnjs.cloudflare.com/ajax/libs/react-dom/16.10.2/umd/react-dom.production.min.js"></script>
    <script src="//cdn.jsdelivr.net/npm/graphiql@0.17.5/graphiql.min.js"></script>
    <script>var GRAPHQL_URL = '{graphql_url}';</script>
    <script>var usingSubscriptions = {using_subscriptions};</script>
    <script>var GRAPHQL_SUBSCRIPTIONS_URL = '{graphql_subscriptions_url}';</script>
    {fetcher_source}
</body>
</html>
"#,
        graphql_url = graphql_endpoint_url,
        stylesheet_source = stylesheet_source,
        fetcher_source = fetcher_source,
        graphql_subscriptions_url = subscriptions_endpoint,
        using_subscriptions = subscriptions_endpoint_url.is_some(),
    )
}
