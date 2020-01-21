# Introspection

GraphQL defines a special built-in top-level field called `__schema`. Querying
for this field allows one to [introspect the schema](https://graphql.org/learn/introspection/)
at runtime to see what queries and mutations the GraphQL server supports.

Because introspection queries are just regular GraphQL queries, Juniper supports
them natively. For example, to get all the names of the types supported one
could execute the following query against Juniper:

```graphql
{
  __schema {
    types {
      name
    }
  }
}
```

## Schema introspection output as JSON

Many client libraries and tools in the GraphQL ecosystem require a complete
representation of the server schema. Often this representation is in JSON and
referred to as `schema.json`. A complete representation of the schema can be
produced by issuing a specially crafted introspection query.

Juniper provides a convenience function to introspect the entire schema. The
result can then be converted to JSON for use with tools and libraries such as
[graphql-client](https://github.com/graphql-rust/graphql-client):

```rust
use juniper::{EmptyMutation, FieldResult, IntrospectionFormat};

// Define our schema.

#[derive(juniper::GraphQLObject)]
struct Example {
  id: String,
}

struct Context;
impl juniper::Context for Context {}

struct Query;

#[juniper::graphql_object(
  Context = Context,
)]
impl Query {
   fn example(id: String) -> FieldResult<Example> {
       unimplemented!()
   }
}

type Schema = juniper::RootNode<'static, Query, EmptyMutation<Context>>;

fn main() {
    // Create a context object.
    let ctx = Context{};

    // Run the built-in introspection query.
    let (res, _errors) = juniper::introspect(
        &Schema::new(Query, EmptyMutation::new()),
        &ctx,
        IntrospectionFormat::default(),
    ).unwrap();

    // Convert introspection result to json.
    let json_result = serde_json::to_string_pretty(&res);
    assert!(json_result.is_ok());
}
```
