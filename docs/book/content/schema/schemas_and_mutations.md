# Schemas

Juniper follows a [code-first approach][schema_approach] to defining GraphQL schemas. If you would like to use a [schema-first approach][schema_approach] instead, consider [juniper-from-schema][] for generating code from a schema file.

A schema consists of three types: a query object, a mutation object, and a subscription object.
These three define the root query fields, mutations and subscriptions of the schema, respectively.

The usage of subscriptions is a little different from the mutation and query objects, so there is a specific [section][section] that discusses them.

Both query and mutation objects are regular GraphQL objects, defined like any
other object in Juniper. The mutation and subscription objects, however, are optional since schemas
can be read-only and do not require subscriptions. If mutation/subscription functionality is not needed, consider using [EmptyMutation][EmptyMutation]/[EmptySubscription][EmptySubscription].

In Juniper, the `RootNode` type represents a schema. When the schema is first created,
Juniper will traverse the entire object graph
and register all types it can find. This means that if you define a GraphQL
object somewhere but never reference it, it will not be exposed in a schema.

## The query root

The query root is just a GraphQL object. You define it like any other GraphQL
object in Juniper, most commonly using the `graphql_object` proc macro:

```rust
# #![allow(unused_variables)]
# extern crate juniper;
# use juniper::{graphql_object, FieldResult, GraphQLObject};
# #[derive(GraphQLObject)] struct User { name: String }
struct Root;

#[graphql_object]
impl Root {
    fn userWithUsername(username: String) -> FieldResult<Option<User>> {
        // Look up user in database...
#       unimplemented!()
    }
}
#
# fn main() { }
```

## Mutations

Mutations are _also_ just GraphQL objects. Each mutation is a single field
that performs some mutating side-effect such as updating a database.

```rust
# #![allow(unused_variables)]
# extern crate juniper;
# use juniper::{graphql_object, FieldResult, GraphQLObject};
# #[derive(GraphQLObject)] struct User { name: String }
struct Mutations;

#[graphql_object]
impl Mutations {
    fn signUpUser(name: String, email: String) -> FieldResult<User> {
        // Validate inputs and save user in database...
#       unimplemented!()
    }
}
#
# fn main() { }
```

# Converting a Rust schema to the [GraphQL Schema Language][schema_language]

Many tools in the GraphQL ecosystem require the schema to be defined in the [GraphQL Schema Language][schema_language]. You can generate a [GraphQL Schema Language][schema_language] representation of your schema defined in Rust using the `schema-language` feature (on by default):

```rust
# extern crate juniper;
use juniper::{
    graphql_object, EmptyMutation, EmptySubscription, FieldResult, RootNode,
};

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

    // Convert the Rust schema into the GraphQL Schema Language.
    let result = schema.as_schema_language();

    let expected = "\
type Query {
  hello: String!
}

schema {
  query: Query
}
";
    assert_eq!(result, expected);
}
```

Note the `schema-language` feature may be turned off if you do not need this functionality to reduce dependencies and speed up
compile times.


[schema_language]: https://graphql.org/learn/schema/#type-language
[juniper-from-schema]: https://github.com/davidpdrsn/juniper-from-schema
[schema_approach]: https://blog.logrocket.com/code-first-vs-schema-first-development-graphql/
[section]: ../advanced/subscriptions.md
[EmptyMutation]: https://docs.rs/juniper/0.14.2/juniper/struct.EmptyMutation.html
<!--TODO: Fix This URL when the EmptySubscription become available in the Documentation  -->
[EmptySubscription]: https://docs.rs/juniper/0.14.2/juniper/struct.EmptySubscription.html
