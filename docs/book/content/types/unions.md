# Unions

From a server's point of view, GraphQL unions are similar to interfaces: the
only exception is that they don't contain fields on their own.

In Juniper, the `graphql_union!` has identical syntax to the [interface
macro](interfaces.md), but does not support defining fields. Therefore, the same
considerations about using traits, placeholder types, or enums still apply to
unions.

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

juniper::graphql_union!(<'a> &'a Character: () as "Character" where Scalar = <S> |&self| { 
    instance_resolvers: |_| {
        // The left hand side indicates the concrete type T, the right hand
        // side should be an expression returning Option<T>
        &Human => self.as_human(),
        &Droid => self.as_droid(),
    }
});

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

juniper::graphql_union!(<'a> &'a Character: Database as "Character" where Scalar = <S> |&self| {
    instance_resolvers: |&context| {
        &Human => context.humans.get(self.id()),
        &Droid => context.droids.get(self.id()),
    }
});

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

juniper::graphql_union!(Character: Database where Scalar = <S> |&self| {
    instance_resolvers: |&context| {
        &Human => context.humans.get(&self.id),
        &Droid => context.droids.get(&self.id),
    }
});

# fn main() {}
```

## Enums

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

juniper::graphql_union!(Character: () where Scalar = <S> |&self| {
    instance_resolvers: |_| {
        &Human => match *self { Character::Human(ref h) => Some(h), _ => None },
        &Droid => match *self { Character::Droid(ref d) => Some(d), _ => None },
    }
});

# fn main() {}
```
