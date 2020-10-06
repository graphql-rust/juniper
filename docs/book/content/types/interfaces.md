Interfaces
==========

[GraphQL interfaces][1] map well to interfaces known from common object-oriented languages such as Java or C#, but Rust, unfortunately, has no concept that maps perfectly to them. The nearest analogue of [GraphQL interfaces][1] are Rust traits, and the main difference is that in GraphQL [interface type][1] serves both as an _abstraction_ and a _boxed value (downcastable to concrete implementers)_, while in Rust, a trait is an _abstraction only_ and _to represent such a boxed value a separate type is required_, like enum or trait object, because Rust trait does not represent a type itself, and so can have no values. This difference imposes some unintuitive and non-obvious corner cases when we try to express [GraphQL interfaces][1] in Rust, but on the other hand gives you full control over which type is backing your interface, and how it's resolved.

For implementing [GraphQL interfaces][1] Juniper provides `#[graphql_interface]` macro.




## Traits

Defining a trait is mandatory for defining a [GraphQL interface][1], because this is the _obvious_ way we describe an _abstraction_ in Rust. All [interface][1] fields are defined as computed ones via trait methods.

```rust
# extern crate juniper;
use juniper::graphql_interface;

#[graphql_interface]
trait Character {
    fn id(&self) -> &str;
}
#
# fn main() {}
```

However, to return values of such [interface][1], we should provide its implementers and the Rust type representing a _boxed value of this trait_. The last one can be represented in two flavors: enum and [trait object][2].


### Enum values (default)

By default, Juniper generates an enum representing the values of the defined [GraphQL interface][1], and names it straightforwardly, `{Interface}Value`.

```rust
# extern crate juniper;
use juniper::{graphql_interface, GraphQLObject};

#[graphql_interface(for = Human)] // enumerating all implementers is mandatory 
trait Character {
    fn id(&self) -> &str;
}

#[derive(GraphQLObject)]
#[graphql(impl = CharacterValue)] // notice enum name, NOT trait name
struct Human {
    id: String,
}
#[graphql_interface] // implementing requires macro attribute too, (°ｏ°)!
impl Character for Human {
    fn id(&self) -> &str {
        &self.id
    }
}
#
# fn main() {
let human = Human { id: "human-32".to_owned() };
// Values type for interface has `From` implementations for all its implementers,
// so we don't need to bother with enum variant names.
let character: CharacterValue = human.into();
assert_eq!(character.id(), "human-32");
# }
```

Also, enum name can be specified explicitly, if desired.

```rust
# extern crate juniper;
use juniper::{graphql_interface, GraphQLObject};

#[graphql_interface(enum = CharaterInterface, for = Human)] 
trait Character {
    fn id(&self) -> &str;
}

#[derive(GraphQLObject)]
#[graphql(impl = CharaterInterface)]
struct Human {
    id: String,
    home_planet: String,
}
#[graphql_interface]
impl Character for Human {
    fn id(&self) -> &str {
        &self.id
    }
}
#
# fn main() {}
```


### Trait object values

If, for some reason, we would like to use [trait objects][2] for representing [interface][1] values incorporating dynamic dispatch, that should be specified explicitly in the trait definition.

Downcasting [trait objects][2] in Rust is not that trivial, that's why macro transforms the trait definition slightly, imposing some additional type parameters under-the-hood.

> __NOTICE__:  
> A __trait has to be [object safe](https://doc.rust-lang.org/stable/reference/items/traits.html#object-safety)__, because schema resolvers will need to return a [trait object][2] to specify a [GraphQL interface][1] behind it.

```rust
# extern crate juniper;
# extern crate tokio;
use juniper::{graphql_interface, GraphQLObject};

// `dyn` argument accepts the name of type alias for the required trait object,
// and macro generates this alias automatically
#[graphql_interface(dyn = DynCharacter, for = Human)] 
trait Character {
    async fn id(&self) -> &str; // async fields are supported natively
}

#[derive(GraphQLObject)]
#[graphql(impl = DynCharacter<__S>)] // macro adds `ScalarValue` type parameter to trait,
struct Human {                       // so it may be specified explicitly when required 
    id: String,
}
#[graphql_interface(dyn)] // implementing requires to know about dynamic dispatch too
impl Character for Human {
    async fn id(&self) -> &str {
        &self.id
    }
}
#
# #[tokio::main]
# async fn main() {
let human = Human { id: "human-32".to_owned() };
let character: Box<DynCharacter> = Box::new(human);
assert_eq!(character.id().await, "human-32");
# }
``` 


### Ignoring trait methods

We may want to omit some trait methods to be assumed as [GraphQL interface][1] fields and ignore them.

```rust
# extern crate juniper;
use juniper::{graphql_interface, GraphQLObject};

#[graphql_interface(for = Human)]  
trait Character {
    fn id(&self) -> &str;

    #[graphql_interface(ignore)] // or `#[graphql_interface(skip)]`, your choice
    fn ignored(&self) -> u32 { 0 }
}

#[derive(GraphQLObject)]
#[graphql(impl = CharacterValue)]
struct Human {
    id: String,
}
#[graphql_interface] // implementing requires macro attribute too, (°ｏ°)!
impl Character for Human {
    fn id(&self) -> &str {
        &self.id
    }
}
#
# fn main() {}
```


### Custom context

If a context is required in a trait method to resolve a [GraphQL interface][1] field, specify it as an argument.

```rust
# extern crate juniper;
# use std::collections::HashMap;
use juniper::{graphql_interface, GraphQLObject};

struct Database {
    humans: HashMap<String, Human>,
}
impl juniper::Context for Database {}

#[graphql_interface(for = Human)] // look, ma, context type is inferred! ＼(^o^)／
trait Character {                 // while still can be specified via `Context = ...` attribute argument
    // If a field argument is named `context` or `ctx`, it's automatically assumed
    // as a context argument.
    fn id(&self, context: &Database) -> Option<&str>;

    // Otherwise, you may mark it explicitly as a context argument.
    fn name(&self, #[graphql_interface(context)] db: &Database) -> Option<&str>;
}

#[derive(GraphQLObject)]
#[graphql(impl = CharacterValue, Context = Database)]
struct Human {
    id: String,
    name: String,
}
#[graphql_interface]
impl Character for Human {
    fn id(&self, db: &Database) -> Option<&str> {
        if db.humans.contains_key(&self.id) {
            Some(&self.id)
        } else {
            None
        }
    }

    fn name(&self, db: &Database) -> Option<&str> {
        if db.humans.contains_key(&self.id) {
            Some(&self.name)
        } else {
            None
        }
    }
}
#
# fn main() {}
```


### Using executor and explicit generic scalar

If an executor is required in a trait method to resolve a [GraphQL interface][1] field, specify it as an argument.

This requires to explicitly parametrize over [`ScalarValue`][3], as [`Executor`][4] does so. 

```rust
# extern crate juniper;
use juniper::{graphql_interface, Executor, GraphQLObject, LookAheadMethods as _, ScalarValue};

#[graphql_interface(for = Human, Scalar = S)] // notice specifying scalar as existing type parameter
trait Character<S: ScalarValue> {             
    // If a field argument is named `executor`, it's automatically assumed
    // as an executor argument.
    async fn id<'a>(&self, executor: &'a Executor<'_, '_, (), S>) -> &'a str
    where
        S: Send + Sync; // required by `#[async_trait]` transformation ¯\_(ツ)_/¯
    

    // Otherwise, you may mark it explicitly as an executor argument.
    async fn name<'b>(
        &'b self,
        #[graphql_interface(executor)] another: &Executor<'_, '_, (), S>,
    ) -> &'b str
    where
        S: Send + Sync;
}

#[derive(GraphQLObject)]
#[graphql(impl = CharacterValue<__S>)]
struct Human {
    id: String,
    name: String,
}
#[graphql_interface(Scalar = S)]
impl<S: ScalarValue> Character<S> for Human {
    async fn id<'a>(&self, executor: &'a Executor<'_, '_, (), S>) -> &'a str
    where
        S: Send + Sync,
    {
        executor.look_ahead().field_name()
    }

    async fn name<'b>(&'b self, _: &Executor<'_, '_, (), S>) -> &'b str
    where
        S: Send + Sync,
    {
        &self.name
    }
}
#
# fn main() {}
```






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

The `instance_resolvers` declaration lists all the implementers of the given
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





[1]: https://spec.graphql.org/June2018/#sec-Interfaces
[2]: https://doc.rust-lang.org/reference/types/trait-object.html
[3]: https://docs.rs/juniper/latest/juniper/trait.ScalarValue.html
[4]: https://docs.rs/juniper/latest/juniper/struct.Executor.html