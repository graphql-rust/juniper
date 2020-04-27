# Subscriptions
### How to achieve realtime data with GraphQL subscriptions

GraphQL subscriptions are a way to push data from the server to clients requesting real-time messages 
from the server. Subscriptions are similar to queries in that they specify a set of fields to be delivered to the client,
but instead of immediately returning a single answer, a result is sent every time a particular event happens on the 
server. 

In order to execute subscriptions you need a coordinator (that spawns connections) 
and a GraphQL object that can be resolved into a stream--elements of which will then 
be returned to the end user. The [juniper_subscriptions][juniper_subscriptions] crate 
provides a default connection implementation. Currently subscriptions are only supported on the `master` branch. Add the following to your `Cargo.toml`:
```toml
[dependencies]
juniper = { git = "https://github.com/graphql-rust/juniper", branch = "master" }
juniper_subscriptions = { git = "https://github.com/graphql-rust/juniper", branch = "master" }
```

### Schema Definition

The Subscription is just a GraphQL object, similar to the Query root and Mutations object that you defined for the 
operations in your [Schema][Schema], the difference is that all the operations defined there should be async and the return of it
should be a [Stream][Stream].

This example shows a subscription operation that returns two events, the strings `Hello` and `World!`
sequentially: 

```rust
# use juniper::http::GraphQLRequest;
# use juniper::{DefaultScalarValue, FieldError, SubscriptionCoordinator};
# use juniper_subscriptions::Coordinator;
# use futures::{Stream, StreamExt};
# use std::pin::Pin;
# #[derive(Clone)]
# pub struct Database;
# impl juniper::Context for Database {}
# impl Database {
#    fn new() -> Self {
#        Self {}
#    }
# }
# pub struct Query;
# #[juniper::graphql_object(Context = Database)]
# impl Query {
#    fn hello_world() -> &str {
#        "Hello World!"
#    }
# }
pub struct Subscription;

type StringStream = Pin<Box<dyn Stream<Item = Result<String, FieldError>> + Send>>;

#[juniper::graphql_subscription(Context = Database)]
impl Subscription {
    async fn hello_world() -> StringStream {
        let stream = tokio::stream::iter(vec![
            Ok(String::from("Hello")),
            Ok(String::from("World!"))
        ]);
        Box::pin(stream)
    }
}
# fn main () {}
```         



### Coordinator

Subscriptions require a bit more resources than regular queries, since they can provide a great vector 
for DOS attacks and can bring down a server easily if not handled right. [SubscriptionCoordinator][SubscriptionCoordinator] trait provides the coordination logic. 
It contains the schema and can keep track of opened connections, handle subscription 
start and maintains a global subscription id. Once connection is established, subscription 
coordinator spawns a [SubscriptionConnection][SubscriptionConnection], which handles a 
single connection, provides resolver logic for a client stream and can provide re-connection 
and shutdown logic.


The [Coordinator][Coordinator] struct is a simple implementation of the trait [SubscriptionCoordinator][SubscriptionCoordinator]
that is responsible for handling the execution of subscription operation into your schema. The execution of the `subscribe` 
operation returns a [Future][Future] with a Item value of a Result<[Connection][Connection], [GraphQLError][GraphQLError]>,
where the connection is the Stream of values returned by the operation and the GraphQLError is the error that occurred in the
resolution of this connection, which means that the subscription failed.

```rust
# use juniper::http::GraphQLRequest;
# use juniper::{DefaultScalarValue, EmptyMutation, FieldError, RootNode, SubscriptionCoordinator};
# use juniper_subscriptions::Coordinator;
# use futures::{Stream, StreamExt};
# use std::pin::Pin;
# use tokio::runtime::Runtime;
# use tokio::task;
# 
# #[derive(Clone)]
# pub struct Database;
# 
# impl juniper::Context for Database {}
# 
# impl Database {
#     fn new() -> Self {
#         Self {}
#     }
# }
# 
# pub struct Query;
# 
# #[juniper::graphql_object(Context = Database)]
# impl Query {
#     fn hello_world() -> &str {
#         "Hello World!"
#     }
# }
# 
# pub struct Subscription;
# 
# type StringStream = Pin<Box<dyn Stream<Item = Result<String, FieldError>> + Send>>;
# 
# #[juniper::graphql_subscription(Context = Database)]
# impl Subscription {
#     async fn hello_world() -> StringStream {
#         let stream =
#             tokio::stream::iter(vec![Ok(String::from("Hello")), Ok(String::from("World!"))]);
#         Box::pin(stream)
#     }
# }
type Schema = RootNode<'static, Query, EmptyMutation<Database>, Subscription>;

fn schema() -> Schema {
    Schema::new(Query {}, EmptyMutation::new(), Subscription {})
}

async fn run_subscription() {
    let schema = schema();
    let coordinator = Coordinator::new(schema);
    let req: GraphQLRequest<DefaultScalarValue> = serde_json::from_str(
        r#"
        {
            "query": "subscription { helloWorld }"
        }
    "#,
    )
        .unwrap();
    let ctx = Database::new();
    let mut conn = coordinator.subscribe(&req, &ctx).await.unwrap();
    while let Some(result) = conn.next().await {
        println!("{}", serde_json::to_string(&result).unwrap());
    }
}

# fn main() { }
```     

### Web Integration and Examples

Currently there is an example of subscriptions with [warp][warp], but it still in an alpha state.
GraphQL over [WS][WS] is not fully supported yet and is non-standard.

- [Warp Subscription Example](https://github.com/graphql-rust/juniper/tree/master/examples/warp_subscriptions)
- [Small Example](https://github.com/graphql-rust/juniper/tree/master/examples/basic_subscriptions)




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
