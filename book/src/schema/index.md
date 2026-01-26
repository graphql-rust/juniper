Schema
======

**[Juniper] follows a [code-first] approach to define a [GraphQL] schema.**

> **TIP**: For a [schema-first] approach, consider using a [`juniper-from-schema`] crate for generating a [`juniper`]-based code from a [schema] file.

[GraphQL schema][0] consists of three [object types][4]: a [query root][1], a [mutation root][2], and a [subscription root][3].

> The **query** root operation type must be provided and must be an [Object][4] type.
> 
> The **mutation** root operation type is optional; if it is not provided, the service does not support mutations. If it is provided, it must be an [Object][4] type.
> 
> Similarly, the **subscription** root operation type is also optional; if it is not provided, the service does not support subscriptions. If it is provided, it must be an [Object][4] type.
> 
> The **query**, **mutation**, and **subscription** root types must all be different types if provided.

In [Juniper], the [`RootNode`] type represents a [schema][0]. When the [schema][0] is first created, [Juniper] will traverse the entire object graph and register all types it can find. This means that if we [define a GraphQL object](../types/objects/index.md) somewhere but never use or reference it, it won't be exposed in a [GraphQL schema][0].

Both [query][1] and [mutation][2] objects are regular [GraphQL objects][4], defined like [any other object in Juniper](../types/objects/index.md). The [mutation][2] and [subscription][3] objects, however, are optional, since [schemas][0] can be read-only and do not require [subscriptions][3].

> **TIP**: If [mutation][2]/[subscription][3] functionality is not needed, consider using the predefined [`EmptyMutation`]/[`EmptySubscription`] types for stubbing them in a [`RootNode`].

```rust
# extern crate juniper;
# use juniper::{
#     EmptySubscription, FieldResult, GraphQLObject, RootNode, graphql_object,
# };
#
#[derive(GraphQLObject)] 
struct User { 
    name: String,
}

struct Query;

#[graphql_object]
impl Query {
    fn user_with_username(username: String) -> FieldResult<Option<User>> {
        // Look up user in database...
#       unimplemented!()
    }
}

struct Mutation;

#[graphql_object]
impl Mutation {
    fn sign_up_user(name: String, email: String) -> FieldResult<User> {
        // Validate inputs and save user in database...
#       unimplemented!()
    }
}

type Schema = RootNode<Query, Mutation, EmptySubscription>;
#
# fn main() {}
```

> **NOTE**: It's considered a [good practice][5] to name [query][1], [mutation][2], and [subscription][3] root types as `Query`, `Mutation`, and `Subscription` respectively.

The usage of [subscriptions][3] is a little different from the [mutation][2] and [query][1] [objects][4], so they are discussed in the [separate chapter](subscriptions.md).




## Export

Many tools in [GraphQL] ecosystem require a [schema] definition to operate on. With [Juniper] we can export our [GraphQL schema][0] defined in [Rust] code either represented in the [GraphQL schema language][6] or in [JSON].


### SDL (schema definition language)

To generate an [SDL (schema definition language)][6] representation of a [GraphQL schema][0] defined in [Rust] code, the [`as_sdl()` method][20] should be used for the direct extraction (requires enabling the `schema-language` [Juniper] feature):
```rust
# extern crate juniper;
# use juniper::{
#     graphql_object, EmptyMutation, EmptySubscription, FieldResult, RootNode,
# };
#
struct Query;

#[graphql_object]
impl Query {
    fn hello(&self) -> FieldResult<&str> {
        Ok("hello world")
    }
}

fn main() {
    // Define our schema in Rust.
    let schema = RootNode::new(
        Query,
        EmptyMutation::<()>::new(),
        EmptySubscription::<()>::new(),
    );

    // Convert the Rust schema into the GraphQL SDL schema.
    let result = schema.as_sdl();

    let expected = "\
schema {
  query: Query
}

type Query {
  hello: String!
}
";
#   #[cfg(not(target_os = "windows"))]
    assert_eq!(result, expected);
}
```


### JSON

To export a [GraphQL schema][0] defined in [Rust] code as [JSON] (often referred to as `schema.json`), the specially crafted [introspection query][21] should be issued. [Juniper] provides a [convenience `introspect()` function][22] to [introspect](introspection.md) the entire [schema][0], which result can be serialized into [JSON]:
```rust
# extern crate juniper;
# extern crate serde_json;
# use juniper::{
#     graphql_object, EmptyMutation, EmptySubscription, GraphQLObject,
#     IntrospectionFormat, RootNode,
# };
#
#[derive(GraphQLObject)]
struct Example {
    id: String,
}

struct Query;

#[graphql_object]
impl Query {
   fn example(id: String) -> Example {
       unimplemented!()
   }
}

type Schema = RootNode<Query, EmptyMutation, EmptySubscription>;

fn main() {
    // Run the built-in introspection query.
    let (res, _errors) = juniper::introspect(
        &Schema::new(Query, EmptyMutation::new(), EmptySubscription::new()),
        &(),
        IntrospectionFormat::default(),
    ).unwrap();

    // Serialize the introspection result into JSON.
    let json_result = serde_json::to_string_pretty(&res);
    assert!(json_result.is_ok());
}
```

> **TIP**: We still can convert the generated [JSON] into a [GraphQL schema language][6] representation by using tools like [`graphql-json-to-sdl` command line utility][30].




[`EmptyMutation`]: https://docs.rs/juniper/0.17.1/juniper/struct.EmptyMutation.html
[`EmptySubscription`]: https://docs.rs/juniper/0.17.1/juniper/struct.EmptySubscription.html
[`juniper`]: https://docs.rs/juniper
[`juniper-from-schema`]: https://docs.rs/juniper-from-schema
[`RootNode`]: https://docs.rs/juniper/0.17.1/juniper/struct.RootNode.html
[code-first]: https://www.apollographql.com/blog/backend/architecture/schema-first-vs-code-only-graphql#code-only
[schema-first]: https://www.apollographql.com/blog/backend/architecture/schema-first-vs-code-only-graphql#schema-first
[GraphQL]: https://graphql.org
[JSON]: https://www.json.org
[Juniper]: https://docs.rs/juniper
[Rust]: https://www.rust-lang.org
[schema]: https://graphql.org/learn/schema

[0]: https://spec.graphql.org/October2021#sec-Schema
[1]: https://spec.graphql.org/October2021#sel-FAHTRFCAACChCtpG
[2]: https://spec.graphql.org/October2021#sel-FAHTRHCAACCuE9yD
[3]: https://spec.graphql.org/October2021#sel-FAHTRJCAACC3EhsX
[4]: https://spec.graphql.org/October2021#sec-Objects
[5]: https://spec.graphql.org/October2021#sec-Root-Operation-Types.Default-Root-Operation-Type-Names
[6]: https://graphql.org/learn/schema#type-language
[20]: https://docs.rs/juniper/0.17.1/juniper/struct.RootNode.html#method.as_sdl
[21]: https://docs.rs/crate/juniper/latest/source/src/introspection/query.graphql
[22]: https://docs.rs/juniper/0.17.1/juniper/fn.introspect.html
[30]: https://npmjs.com/package/graphql-json-to-sdl