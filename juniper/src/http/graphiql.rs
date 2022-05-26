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
      body {
        height: 100%;
        margin: 0;
        width: 100%;
        overflow: hidden;
      }

      #graphiql {
        height: 100vh;
      }
    </style>
    "#;
    let fetcher_source = r#"
    <script>
        if (usingSubscriptions) {
            var subscriptionEndpoint = normalizeSubscriptionEndpoint(GRAPHQL_URL, GRAPHQL_SUBSCRIPTIONS_URL);
            var subscriptionsClient = new window.SubscriptionsTransportWs.SubscriptionClient(subscriptionEndpoint, { reconnect: true });
        }

        function normalizeSubscriptionEndpoint(endpoint, subscriptionEndpoint) {
            if (subscriptionEndpoint) {
                if (subscriptionEndpoint.startsWith('/')) {
                    const secure =
                        endpoint.includes('https') || location.href.includes('https')
                        ? 's'
                        : ''
                    return `ws${secure}://${location.host}${subscriptionEndpoint}`
                } else {
                    return subscriptionEndpoint.replace(/^http/, 'ws')
                }
            }
            return null
        }

        function graphQLFetcher(graphQLParams, opts) {
            const { headers = {} } = opts;

            return fetch(
                GRAPHQL_URL,
                {
                    method: 'post',
                    headers: {
                        Accept: 'application/json',
                        'Content-Type': 'application/json',
                        ...headers,
                    },
                    body: JSON.stringify(graphQLParams),
                    credentials: 'omit',
                },
            ).then(function (response) {
                return response.json().catch(function () {
                    return response.text();
                });
            });
        }

        var fetcher = usingSubscriptions ? window.GraphiQLSubscriptionsFetcher.graphQLFetcher(subscriptionsClient, graphQLFetcher) : graphQLFetcher;

        ReactDOM.render(
            React.createElement(GraphiQL, {
              fetcher,
              defaultVariableEditorOpen: true,
            }),
            document.getElementById('graphiql'),
        );
    </script>
    "#;

    format!(
        r#"
<!DOCTYPE html>
<html>
<head>
    <title>GraphQL</title>
    {stylesheet_source}
    <script
      crossorigin
      src="https://unpkg.com/react@17/umd/react.development.js"
    ></script>
    <script
      crossorigin
      src="https://unpkg.com/react-dom@17/umd/react-dom.development.js"
    ></script>
    <link rel="stylesheet" href="https://unpkg.com/graphiql/graphiql.min.css" />
</head>
<body>
    <div id="graphiql">Loading...</div>
    <script
        src="https://unpkg.com/graphiql/graphiql.min.js"
        type="application/javascript"
    ></script>
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
