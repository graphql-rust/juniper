# Integrating with Iron

[Iron] is a library that's been around for a while in the Rust sphere but lately
hasn't seen much of development. Nevertheless, it's still a solid library with a
familiar request/response/middleware architecture that works on Rust's stable
channel.

Juniper's Iron integration is contained in the `juniper_iron` crate:

!FILENAME Cargo.toml

```toml
[dependencies]
juniper = "0.10"
juniper_iron = "0.2.0"
```

Included in the source is a [small
example](https://github.com/graphql-rust/juniper_iron/blob/master/examples/iron_server.rs)
which sets up a basic GraphQL and [GraphiQL] handler.

## Basic integration

Let's start with a minimal schema and just get a GraphQL endpoint up and
running. We use [mount] to attach the GraphQL handler at `/graphql`.

The `context_factory` function will be executed on every request and can be used
to set up database connections, read session token information from cookies, and
set up other global data that the schema might require.

In this example, we won't use any global data so we just return an empty value.

```rust,ignore
extern crate juniper;
extern crate juniper_iron;
extern crate iron;
extern crate mount;

use mount::Mount;
use iron::prelude::*;
use juniper::EmptyMutation;
use juniper_iron::GraphQLHandler;

fn context_factory(_: &mut Request) -> IronResult<()> {
    Ok(())
}

struct Root;

#[juniper::graphql_object]
impl Root {
    fn foo() -> String {
        "Bar".to_owned()
    }
}

# #[allow(unreachable_code, unused_variables)]
fn main() {
    let mut mount = Mount::new();

    let graphql_endpoint = GraphQLHandler::new(
        context_factory,
        Root,
        EmptyMutation::<()>::new(),
    );

    mount.mount("/graphql", graphql_endpoint);

    let chain = Chain::new(mount);

#   return;
    Iron::new(chain).http("0.0.0.0:8080").unwrap();
}
```

## Accessing data from the request

If you want to access e.g. the source IP address of the request from a field
resolver, you need to pass this data using Juniper's [context feature](../types/objects/using_contexts.md).

```rust,ignore
# extern crate juniper;
# extern crate juniper_iron;
# extern crate iron;
# use iron::prelude::*;
use std::net::SocketAddr;

struct Context {
    remote_addr: SocketAddr,
}

impl juniper::Context for Context {}

fn context_factory(req: &mut Request) -> IronResult<Context> {
    Ok(Context {
        remote_addr: req.remote_addr
    })
}

struct Root;

#[juniper::graphql_object(
    Context = Context,
)]
impl Root {
    field my_addr(context: &Context) -> String {
        format!("Hello, you're coming from {}", context.remote_addr)
    }
}

# fn main() {
#     let _graphql_endpoint = juniper_iron::GraphQLHandler::new(
#         context_factory,
#         Root,
#         juniper::EmptyMutation::<Context>::new(),
#     );
# }
```

[iron]: http://ironframework.io
[graphiql]: https://github.com/graphql/graphiql
[mount]: https://github.com/iron/mount
