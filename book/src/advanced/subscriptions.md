# Subscriptions
### How to achieve realtime data with GraphQL subscriptions

GraphQL subscriptions are a way to push data from the server to clients requesting real-time messages 
from the server. Subscriptions are similar to queries in that they specify a set of fields to be delivered to the client,
but instead of immediately returning a single answer a result is sent every time a particular event happens on the 
server. 

In order to execute subscriptions you need a coordinator (that spawns connections) 
and a GraphQL object that can be resolved into a stream--elements of which will then 
be returned to the end user. The [`juniper_subscriptions`][juniper_subscriptions] crate 
provides a default connection implementation. Currently subscriptions are only supported on the `master` branch. Add the following to your `Cargo.toml`:
```toml
[dependencies]
juniper = "0.16.0"
juniper_subscriptions = "0.17.0"
```

### Schema Definition

The `Subscription` is just a GraphQL object, similar to the query root and mutations object that you defined for the 
operations in your [Schema][Schema]. For subscriptions all fields/operations should be async and should return a [Stream][Stream].

This example shows a subscription operation that returns two events, the strings `Hello` and `World!`
sequentially: 

```rust
# extern crate futures;
# extern crate juniper;
# use std::pin::Pin;
# use futures::Stream;
# use juniper::{graphql_object, graphql_subscription, FieldError};
#
# #[derive(Clone)]
# pub struct Database;
# impl juniper::Context for Database {}

# pub struct Query;
# #[graphql_object(context = Database)]
# impl Query {
#    fn hello_world() -> &'static str {
#        "Hello World!"
#    }
# }
pub struct Subscription;

type StringStream = Pin<Box<dyn Stream<Item = Result<String, FieldError>> + Send>>;

#[graphql_subscription(context = Database)]
impl Subscription {
    async fn hello_world() -> StringStream {
        let stream = futures::stream::iter(vec![
            Ok(String::from("Hello")),
            Ok(String::from("World!"))
        ]);
        Box::pin(stream)
    }
}
#
# fn main () {}
```



### Coordinator

Subscriptions require a bit more resources than regular queries and provide a great vector for DOS attacks. This can can bring down a server easily if not handled correctly. The [`SubscriptionCoordinator`][SubscriptionCoordinator] trait provides coordination logic to enable functionality like DOS attack mitigation and resource limits.

The [`SubscriptionCoordinator`][SubscriptionCoordinator] contains the schema and can keep track of opened connections, handle subscription 
start and end, and maintain a global subscription id for each subscription. Each time a connection is established,  
the [`SubscriptionCoordinator`][SubscriptionCoordinator] spawns a [`SubscriptionConnection`][SubscriptionConnection]. The [`SubscriptionConnection`][SubscriptionConnection] handles a single connection, providing resolver logic for a client stream as well as reconnection 
and shutdown logic.


While you can implement [`SubscriptionCoordinator`][SubscriptionCoordinator] yourself, Juniper contains a simple and generic implementation called [`Coordinator`][Coordinator].  The `subscribe` 
operation returns a [`Future`][Future] with an `Item` value of a `Result<Connection, GraphQLError>`,
where [`Connection`][Connection] is a `Stream` of values returned by the operation and [`GraphQLError`][GraphQLError] is the error when the subscription fails.

```rust
# #![allow(dead_code)]
# extern crate futures;
# extern crate juniper;
# extern crate juniper_subscriptions;
# extern crate serde_json;
# use juniper::{
#     http::GraphQLRequest,
#     graphql_object, graphql_subscription, 
#     DefaultScalarValue, EmptyMutation, FieldError, 
#     RootNode, SubscriptionCoordinator,
# };
# use juniper_subscriptions::Coordinator;
# use futures::{Stream, StreamExt};
# use std::pin::Pin;
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
# #[graphql_object(context = Database)]
# impl Query {
#     fn hello_world() -> &'static str {
#         "Hello World!"
#     }
# }
# 
# pub struct Subscription;
# 
# type StringStream = Pin<Box<dyn Stream<Item = Result<String, FieldError>> + Send>>;
# 
# #[graphql_subscription(context = Database)]
# impl Subscription {
#     async fn hello_world() -> StringStream {
#         let stream =
#             futures::stream::iter(vec![Ok(String::from("Hello")), Ok(String::from("World!"))]);
#         Box::pin(stream)
#     }
# }
type Schema = RootNode<'static, Query, EmptyMutation<Database>, Subscription>;

fn schema() -> Schema {
    Schema::new(Query, EmptyMutation::new(), Subscription)
}

async fn run_subscription() {
    let schema = schema();
    let coordinator = Coordinator::new(schema);
    let req: GraphQLRequest<DefaultScalarValue> = serde_json::from_str(
        r#"{
            "query": "subscription { helloWorld }"
        }"#,
    )
        .unwrap();
    let ctx = Database::new();
    let mut conn = coordinator.subscribe(&req, &ctx).await.unwrap();
    while let Some(result) = conn.next().await {
        println!("{}", serde_json::to_string(&result).unwrap());
    }
}
#
# fn main() { }
```     

### Web Integration and Examples

Currently there is an example of subscriptions with [warp][warp], but it still in an alpha state.
GraphQL over [WS][WS] is not fully supported yet and is non-standard.

- [Warp Subscription Example](https://github.com/graphql-rust/juniper/tree/master/juniper_warp/examples/subscription.rs)
- [Small Example](https://github.com/graphql-rust/juniper/tree/master/juniper_subscriptions/examples/basic.rs)




[juniper_subscriptions]: https://github.com/graphql-rust/juniper/tree/master/juniper_subscriptions
[Stream]: https://docs.rs/futures/0.3.4/futures/stream/trait.Stream.html
 <!-- TODO: Fix these links when the documentation for the `juniper_subscriptions` are defined in the docs. --->
[Coordinator]: https://docs.rs/juniper_subscriptions/0.15.0/struct.Coordinator.html
[SubscriptionCoordinator]: https://docs.rs/juniper_subscriptions/0.15.0/trait.SubscriptionCoordinator.html
[Connection]: https://docs.rs/juniper_subscriptions/0.15.0/struct.Connection.html
[SubscriptionConnection]: https://docs.rs/juniper_subscriptions/0.15.0/trait.SubscriptionConnection.html
<!--- --->
[Future]: https://docs.rs/futures/0.3.4/futures/future/trait.Future.html
[warp]: https://github.com/graphql-rust/juniper/tree/master/juniper_warp
[WS]: https://github.com/apollographql/subscriptions-transport-ws/blob/master/PROTOCOL.md
[GraphQLError]: https://docs.rs/juniper/0.14.2/juniper/enum.GraphQLError.html
[Schema]: ../schema/schemas_and_mutations.md
