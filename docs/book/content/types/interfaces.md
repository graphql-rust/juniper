# Interfaces

GraphQL interfaces map well to interfaces known from common object-oriented
languages such as Java or C#, but Rust has unfortunately not a concept that maps
perfectly to them. Because of this, defining interfaces in Juniper can require a
little bit of boilerplate code, but on the other hand gives you full control
over which type is backing your interface.

To highlight a couple of different ways you can implement interfaces in Rust,
let's have a look at the same end-result from a few different implementations:

## Traits

Traits are maybe the most obvious concept you want to use when building
interfaces. But because GraphQL supports downcasting while Rust doesn't, you'll
have to manually specify how to convert a trait into a concrete type. This can
be done in a couple of different ways:

### Downcasting via accessor methods

```rust,ignore
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
    fn id(&self) -> &str;

    // Downcast methods, each concrete class will need to implement one of these
    fn as_human(&self) -> Option<&Human> { None }
    fn as_droid(&self) -> Option<&Droid> { None }
}

impl Character for Human {
    fn id(&self) -> &str { self.id.as_str() }
    fn as_human(&self) -> Option<&Human> { Some(&self) }
}

impl Character for Droid {
    fn id(&self) -> &str { self.id.as_str() }
    fn as_droid(&self) -> Option<&Droid> { Some(&self) }
}

juniper::graphql_interface!(<'a> &'a dyn Character: () as "Character" where Scalar = <S> |&self| {
    field id() -> &str { self.id() }

    instance_resolvers: |_| {
        // The left hand side indicates the concrete type T, the right hand
        // side should be an expression returning Option<T>
        &Human => self.as_human(),
        &Droid => self.as_droid(),
    }
});

# fn main() {}
```

The `instance_resolvers` declaration lists all the implementors of the given
interface and how to resolve them.

As you can see, you lose a bit of the point with using traits: you need to list
all the concrete types in the trait itself, and there's a bit of repetition
going on.

### Using an extra database lookup

If you can afford an extra database lookup when the concrete class is requested,
you can do away with the downcast methods and use the context instead. Here,
we'll use two hashmaps, but this could be two tables and some SQL calls instead:

```rust,ignore
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

juniper::graphql_interface!(<'a> &'a dyn Character: Database as "Character" where Scalar = <S> |&self| {
    field id() -> &str { self.id() }

    instance_resolvers: |&context| {
        &Human => context.humans.get(self.id()),
        &Droid => context.droids.get(self.id()),
    }
});

# fn main() {}
```

This removes the need of downcast methods, but still requires some repetition.

## Placeholder objects

Continuing on from the last example, the trait itself seems a bit unneccesary.
Maybe it can just be a struct containing the ID?

```rust,ignore
# use std::collections::HashMap;
#[derive(juniper::GraphQLObject)]
#[graphql(Context = "Database")]
struct Human {
    id: String,
    home_planet: String,
}

#[derive(juniper::GraphQLObject)]
#[graphql(Context = "Database")]
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

juniper::graphql_interface!(Character: Database where Scalar = <S> |&self| {
    field id() -> &str { self.id.as_str() }

    instance_resolvers: |&context| {
        &Human => context.humans.get(&self.id),
        &Droid => context.droids.get(&self.id),
    }
});

# fn main() {}
```

This reduces repetition some more, but might be impractical if the interface's
surface area is large. 

## Enums

Using enums and pattern matching lies half-way between using traits and using
placeholder objects. We don't need the extra database call in this case, so
we'll remove it.

```rust,ignore
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

juniper::graphql_interface!(Character: () where Scalar = <S> |&self| {
    field id() -> &str {
        match *self {
            Character::Human(Human { ref id, .. }) |
            Character::Droid(Droid { ref id, .. }) => id,
        }
    }

    instance_resolvers: |_| {
        &Human => match *self { Character::Human(ref h) => Some(h), _ => None },
        &Droid => match *self { Character::Droid(ref d) => Some(d), _ => None },
    }
});

# fn main() {}
```
