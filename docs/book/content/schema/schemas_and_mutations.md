# Schemas

A schema consists of three types: a query object, a mutation object, and a subscription object.
These three define the root query fields, mutations and subscriptions of the schema, respectively.

The usage of subscriptions is a little different from the mutation and query objects, so there is a specific [section][section] that discusses them.

Both query and mutation objects are regular GraphQL objects, defined like any
other object in Juniper. The mutation and subscription object, however, is optional since schemas
can be read-only and without subscriptions as well. If mutations/subscriptions functionality is not needed, consider using [EmptyMutation][EmptyMutation]/[EmptySubscription][EmptySubscription].

In Juniper, the `RootNode` type represents a schema. You usually don't have to
create this object yourself: see the framework integrations for [Iron](../servers/iron.md)
and [Rocket](../servers/rocket.md) how schemas are created together with the handlers
themselves.

When the schema is first created, Juniper will traverse the entire object graph
and register all types it can find. This means that if you define a GraphQL
object somewhere but never references it, it will not be exposed in a schema.

## The query root

The query root is just a GraphQL object. You define it like any other GraphQL
object in Juniper, most commonly using the `object` proc macro:

```rust
# use juniper::FieldResult;
# #[derive(juniper::GraphQLObject)] struct User { name: String }
struct Root;

#[juniper::graphql_object]
impl Root {
    fn userWithUsername(username: String) -> FieldResult<Option<User>> {
        // Look up user in database...
# unimplemented!()
    }
}

# fn main() { }
```

## Mutations

Mutations are _also_ just GraphQL objects. Each mutation is a single field that
usually performs some mutating side-effect, such as updating a database.

```rust
# use juniper::FieldResult;
# #[derive(juniper::GraphQLObject)] struct User { name: String }
struct Mutations;

#[juniper::graphql_object]
impl Mutations {
    fn signUpUser(name: String, email: String) -> FieldResult<User> {
        // Validate inputs and save user in database...
# unimplemented!()
    }
}

# fn main() { }
```

[section]: ../advanced/subscriptions.md
[EmptyMutation]: https://docs.rs/juniper/0.14.2/juniper/struct.EmptyMutation.html
<!--TODO: Fix This URL when the EmptySubscription become available in the Documentation  -->
[EmptySubscription]: https://docs.rs/juniper/0.14.2/juniper/struct.EmptySubscription.html
