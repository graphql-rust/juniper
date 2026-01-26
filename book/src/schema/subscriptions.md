Subscriptions
=============

[GraphQL subscriptions][9] are a way to push data from a server to clients requesting real-time messages from a server. [Subscriptions][9] are similar to [queries][7] in that they specify a set of fields to be delivered to a client, but instead of immediately returning a single answer a result is sent every time a particular event happens on a server.

In order to execute [subscriptions][9] in [Juniper], we need a coordinator (spawning long-lived connections) and a [GraphQL object][4] with [fields][5] resolving into a [`Stream`] of elements which will then be returned to a client. The [`juniper_subscriptions` crate][30] provides a default implementation of these abstractions.

The [subscription root][3] is just a [GraphQL object][4], similar to the [query root][1] and [mutations root][2] that we define for operations in our [GraphQL schema][0]. For [subscriptions][9] all fields should be `async` and return a [`Stream`] of some [GraphQL type][6] values, rather than direct values.

```rust
# extern crate futures;
# extern crate juniper;
# use std::pin::Pin;
# use futures::Stream;
# use juniper::{FieldError, graphql_object, graphql_subscription};
#
# #[derive(Clone)]
# pub struct Database;
#
# impl juniper::Context for Database {}
#
# pub struct Query;
#
# #[graphql_object]
# #[graphql(context = Database)]
# impl Query {
#    fn hello_world() -> &'static str {
#        "Hello World!"
#    }
# }
#
type StringStream = Pin<Box<dyn Stream<Item = Result<String, FieldError>> + Send>>;

pub struct Subscription;

#[graphql_subscription]
#[graphql(context = Database)]
impl Subscription {
    // This subscription operation emits two values sequentially:
    // the `String`s "Hello" and "World!".
    async fn hello_world() -> StringStream {
        let stream = futures::stream::iter([
            Ok(String::from("Hello")),
            Ok(String::from("World!")),
        ]);
        Box::pin(stream)
    }
}
#
# fn main () {}
```




## Coordinator

[GraphQL subscriptions][9] require a bit more resources than regular [queries][7] and provide a great vector for [DoS attacks][20]. This can can bring down a server easily if not handled correctly. The [`SubscriptionCoordinator` trait][`SubscriptionCoordinator`] provides coordination logic to enable functionality like [DoS attacks][20] mitigation and resource limits.

The [`SubscriptionCoordinator`] contains the [schema][0] and can keep track of opened connections, handle [subscription][9] start and end, and maintain a global ID for each [subscription][9]. Each time a connection is established, the [`SubscriptionCoordinator`] spawns a [32], which handles a single connection, providing resolver logic for a client stream as well as reconnection and shutdown logic.

While we can implement [`SubscriptionCoordinator`] ourselves, [Juniper] contains a simple and generic implementation called [`Coordinator`]. The `subscribe` method returns a [`Future`] resolving into a `Result<Connection, GraphQLError>`, where [`Connection`] is a [`Stream`] of [values][10] returned by the operation, and a [`GraphQLError`] is the error when the [subscription operation][9] fails.

```rust
# extern crate futures;
# extern crate juniper;
# extern crate juniper_subscriptions;
# extern crate serde_json;
# use std::pin::Pin;
# use futures::{Stream, StreamExt as _};
# use juniper::{
#     http::GraphQLRequest,
#     graphql_object, graphql_subscription, 
#     DefaultScalarValue, EmptyMutation, FieldError, 
#     RootNode, SubscriptionCoordinator,
# };
# use juniper_subscriptions::Coordinator;
# 
# #[derive(Clone)]
# pub struct Database;
# 
# impl juniper::Context for Database {}
# 
# impl Database {
#     fn new() -> Self {
#         Self
#     }
# }
# 
# pub struct Query;
# 
# #[graphql_object]
# #[graphql(context = Database)]
# impl Query {
#     fn hello_world() -> &'static str {
#         "Hello World!"
#     }
# }
#
# type StringStream = Pin<Box<dyn Stream<Item = Result<String, FieldError>> + Send>>;
#
# pub struct Subscription;
# 
# #[graphql_subscription]
# #[graphql(context = Database)]
# impl Subscription {
#     async fn hello_world() -> StringStream {
#         let stream = futures::stream::iter([
#             Ok(String::from("Hello")), 
#             Ok(String::from("World!")),
#         ]);
#         Box::pin(stream)
#     }
# }
#
type Schema = RootNode<Query, EmptyMutation<Database>, Subscription>;

fn schema() -> Schema {
    Schema::new(Query, EmptyMutation::new(), Subscription)
}

async fn run_subscription() {
    let schema = schema();
    let coordinator = Coordinator::new(schema);
    let db = Database::new();

    let req: GraphQLRequest<DefaultScalarValue> = serde_json::from_str(
        r#"{
            "query": "subscription { helloWorld }"
        }"#,
    ).unwrap();
    
    let mut conn = coordinator.subscribe(&req, &db).await.unwrap();
    while let Some(result) = conn.next().await {
        println!("{}", serde_json::to_string(&result).unwrap());
    }
}
#
# fn main() {}
```




## WebSocket

For information about serving [GraphQL subscriptions][9] over [WebSocket], see the ["Serving" chapter](../serve/index.md#websocket).




[`Coordinator`]: https://docs.rs/juniper_subscriptions/0.18.0/juniper_subscriptions/struct.Coordinator.html
[`Connection`]: https://docs.rs/juniper_subscriptions/0.18.0/juniper_subscriptions/struct.Connection.html
[`Future`]: https://doc.rust-lang.org/stable/std/future/trait.Future.html
[`GraphQLError`]: https://docs.rs/juniper/0.17.1/juniper/enum.GraphQLError.html
[`Stream`]: https://docs.rs/futures/latest/futures/stream/trait.Stream.html
[`SubscriptionCoordinator`]:  https://docs.rs/juniper/0.17.1/juniper/trait.SubscriptionCoordinator.html
[`SubscriptionConnection`]: https://docs.rs/juniper/0.17.1/juniper/trait.SubscriptionConnection.html
[GraphQL]: https://graphql.org
[Juniper]: https://docs.rs/juniper
[Rust]: https://www.rust-lang.org
[WebSocket]: https://en.wikipedia.org/wiki/WebSocket

[0]: https://spec.graphql.org/October2021#sec-Schema
[1]: https://spec.graphql.org/October2021#sel-FAHTRFCAACChCtpG
[2]: https://spec.graphql.org/October2021#sel-FAHTRHCAACCuE9yD
[3]: https://spec.graphql.org/October2021#sel-FAHTRJCAACC3EhsX
[4]: https://spec.graphql.org/October2021#sec-Objects
[5]: https://spec.graphql.org/October2021#sec-Language.Fields
[6]: https://spec.graphql.org/October2021#sec-Types
[7]: https://spec.graphql.org/October2021#sec-Query
[8]: https://spec.graphql.org/October2021#sec-Mutation
[9]: https://spec.graphql.org/October2021#sec-Subscription
[10]: https://spec.graphql.org/October2021#sec-Values
[20]: https://en.wikipedia.org/wiki/Denial-of-service_attack
[30]: https://docs.rs/juniper_subscriptions
