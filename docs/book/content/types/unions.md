# Unions

From a server's point of view, GraphQL unions are similar to interfaces: the
only exception is that they don't contain fields on their own.

In Juniper, the `graphql_union!` has identical syntax to the
[interface macro](interfaces.md), but does not support defining
fields. Therefore, the same considerations about using traits,
placeholder types, or enums still apply to unions. For simple
situations, Juniper provides `#[derive(GraphQLUnion)]` for enums.

If we look at the same examples as in the interfaces chapter, we see the
similarities and the tradeoffs:

## Traits

### Downcasting via accessor methods

```rust
#[derive(juniper::GraphQLObject)]
struct Human {
    id: String,
    home_planet: String,
}

#[derive(juniper::GraphQLObject)]
struct Droid {
    id: String,
    primary_function: String,
}

trait Character {
    // Downcast methods, each concrete class will need to implement one of these
    fn as_human(&self) -> Option<&Human> { None }
    fn as_droid(&self) -> Option<&Droid> { None }
}

impl Character for Human {
    fn as_human(&self) -> Option<&Human> { Some(&self) }
}

impl Character for Droid {
    fn as_droid(&self) -> Option<&Droid> { Some(&self) }
}

#[juniper::graphql_union]
impl<'a> GraphQLUnion for &'a dyn Character {
    fn resolve(&self) {
        match self {
            Human => self.as_human(),
            Droid => self.as_droid(),
        }
    }
}

# fn main() {}
```

### Using an extra database lookup

FIXME: This example does not compile at the moment

```rust
# use std::collections::HashMap;
#[derive(juniper::GraphQLObject)]
#[graphql(Context = Database)]
struct Human {
    id: String,
    home_planet: String,
}

#[derive(juniper::GraphQLObject)]
#[graphql(Context = Database)]
struct Droid {
    id: String,
    primary_function: String,
}

struct Database {
    humans: HashMap<String, Human>,
    droids: HashMap<String, Droid>,
}

impl juniper::Context for Database {}

trait Character {
    fn id(&self) -> &str;
}

impl Character for Human {
    fn id(&self) -> &str { self.id.as_str() }
}

impl Character for Droid {
    fn id(&self) -> &str { self.id.as_str() }
}


#[juniper::graphql_union(
    Context = Database
)]
impl<'a> GraphQLUnion for &'a dyn Character {
    fn resolve(&self, context: &Database) {
        match self {
            Human => context.humans.get(self.id()),
            Droid => context.droids.get(self.id()),
        }
    }
}

# fn main() {}
```

## Placeholder objects

```rust
# use std::collections::HashMap;
#[derive(juniper::GraphQLObject)]
#[graphql(Context = Database)]
struct Human {
    id: String,
    home_planet: String,
}

#[derive(juniper::GraphQLObject)]
#[graphql(Context = Database)]
struct Droid {
    id: String,
    primary_function: String,
}

struct Database {
    humans: HashMap<String, Human>,
    droids: HashMap<String, Droid>,
}

impl juniper::Context for Database {}

struct Character {
    id: String,
}

#[juniper::graphql_union(
    Context = Database,
)]
impl GraphQLUnion for Character {
    fn resolve(&self, context: &Database) {
        match self {
            Human => { context.humans.get(&self.id) },
            Droid => { context.droids.get(&self.id) },
        }
    }
}

# fn main() {}
```

## Enums (Impl)

```rust
#[derive(juniper::GraphQLObject)]
struct Human {
    id: String,
    home_planet: String,
}

#[derive(juniper::GraphQLObject)]
struct Droid {
    id: String,
    primary_function: String,
}

# #[allow(dead_code)]
enum Character {
    Human(Human),
    Droid(Droid),
}

#[juniper::graphql_union]
impl Character {
    fn resolve(&self) {
        match self {
            Human => { match *self { Character::Human(ref h) => Some(h), _ => None } },
            Droid => { match *self { Character::Droid(ref d) => Some(d), _ => None } },
        }
    }
}

# fn main() {}
```

## Enums (Derive)

This example is similar to `Enums (Impl)`. To successfully use the
derive macro, ensure that each variant of the enum has a different
type. Since each variant is different, the device macro provides
`std::convert::Into<T>` converter for each variant.

```rust
#[derive(juniper::GraphQLObject)]
struct Human {
    id: String,
    home_planet: String,
}

#[derive(juniper::GraphQLObject)]
struct Droid {
    id: String,
    primary_function: String,
}

#[derive(juniper::GraphQLUnion)]
enum Character {
    Human(Human),
    Droid(Droid),
}

# fn main() {}
```
