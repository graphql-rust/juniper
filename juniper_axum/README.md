`juniper_axum` crate
====================

[![Crates.io](https://img.shields.io/crates/v/juniper_axum.svg?maxAge=2592000)](https://crates.io/crates/juniper_warp)
[![Documentation](https://docs.rs/juniper_warp/badge.svg)](https://docs.rs/juniper_warp)
[![CI](https://github.com/graphql-rust/juniper/workflows/CI/badge.svg?branch=master "CI")](https://github.com/graphql-rust/juniper/actions?query=workflow%3ACI+branch%3Amaster)

- [Changelog](https://github.com/graphql-rust/juniper/blob/master/juniper_axum/CHANGELOG.md)

[`axum`] web server integration for [`juniper`] ([GraphQL] implementation for [Rust]).

## Getting started

The best way to get started is to examine the `simple` example in the `examples` directory. To execute
this example run

`cargo run --example simple`

Open your browser and navigate to `127.0.0.1:3000`. A GraphQL Playground opens. The 
following commands are available in the playground.

```graphql
{
    add(a: 2, b: 40)
}
```

```graphql
subscription {
    count
}
```

## Queries and mutations
This crate provides an extractor and response for axum to work with juniper.

```rust,ignore
use juniper_axum::response::JuniperResponse;

let app: Router<Body> = Router::new()
    .route("/graphql", post(graphql))
    .layer(Extension(schema))
    .layer(Extension(context));

async fn graphql(
    JuniperRequest(request): JuniperRequest,
    Extension(schema): Extension<Arc<Schema>>,
    Extension(context): Extension<Arc<Context>>
) -> JuniperResponse {
    JuniperResponse(request.execute(&schema, &context).await)
}
```

## Subscriptions
This crate provides a helper function to easily work with graphql subscriptions over a websocket.
```rust,ignore
use juniper_axum::subscription::handle_graphql_socket;

let app: Router = Router::new()
    .route("/subscriptions", get(juniper_subscriptions))
    .layer(Extension(schema))
    .layer(Extension(context));

pub async fn juniper_subscriptions(
    Extension(schema): Extension<Arc<Schema>>,
    Extension(context): Extension<Context>,
    ws: WebSocketUpgrade,
) -> Response {
    ws.protocols(["graphql-ws", "graphql-transport-ws"])
        .max_frame_size(1024)
        .max_message_size(1024)
        .max_send_queue(100)
        .on_upgrade(|socket| handle_graphql_socket(socket, schema, context))
}
```



## License

This project is licensed under [BSD 2-Clause License](https://github.com/graphql-rust/juniper/blob/master/juniper_axum/LICENSE).




[`juniper`]: https://docs.rs/juniper
[`juniper_axum`]: https://docs.rs/juniper_axum
[`axum`]: https://docs.rs/axum
[GraphQL]: http://graphql.org
[Juniper Book]: https://graphql-rust.github.io
[Rust]: https://www.rust-lang.org

[1]: https://github.com/graphql-rust/juniper/blob/master/juniper_warp/examples/warp_server.rs
