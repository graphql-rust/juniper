# Schemas

A schema consists of two types: a query object and a mutation object (Juniper
does not support subscriptions yet). These two define the root query fields
and mutations of the schema, respectively.

Both query and mutation objects are regular GraphQL objects, defined like any
other object in Juniper. The mutation object, however, is optional since schemas
can be read-only.

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
