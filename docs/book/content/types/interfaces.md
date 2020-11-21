Interfaces
==========

[GraphQL interfaces][1] map well to interfaces known from common object-oriented languages such as Java or C#, but Rust, unfortunately, has no concept that maps perfectly to them. The nearest analogue of [GraphQL interfaces][1] are Rust traits, and the main difference is that in GraphQL an [interface type][1] serves both as an _abstraction_ and a _boxed value (downcastable to concrete implementers)_, while in Rust, a trait is an _abstraction only_ and _to represent such a boxed value a separate type is required_, like enum or trait object, because Rust trait doesn't represent a type itself, and so can have no values. This difference imposes some unintuitive and non-obvious corner cases when we try to express [GraphQL interfaces][1] in Rust, but on the other hand gives you full control over which type is backing your interface, and how it's resolved.

For implementing [GraphQL interfaces][1] Juniper provides the `#[graphql_interface]` macro.




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

#[graphql_interface(for = [Human, Droid])] // enumerating all implementers is mandatory 
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

#[derive(GraphQLObject)]
#[graphql(impl = CharacterValue)]
struct Droid {
    id: String,
}
#[graphql_interface]
impl Character for Droid {
    fn id(&self) -> &str {
        &self.id
    }
}

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

If, for some reason, we would like to use [trait objects][2] for representing [interface][1] values incorporating dynamic dispatch, then it should be specified explicitly in the trait definition.

Downcasting [trait objects][2] in Rust is not that trivial, that's why macro transforms the trait definition slightly, imposing some additional type parameters under-the-hood.

> __NOTICE__:  
> A __trait has to be [object safe](https://doc.rust-lang.org/stable/reference/items/traits.html#object-safety)__, because schema resolvers will need to return a [trait object][2] to specify a [GraphQL interface][1] behind it.

```rust
# extern crate juniper;
# extern crate tokio;
use juniper::{graphql_interface, GraphQLObject};

// `dyn` argument accepts the name of type alias for the required trait object,
// and macro generates this alias automatically.
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

#[derive(GraphQLObject)]
#[graphql(impl = DynCharacter<__S>)]
struct Droid {
    id: String,
}
#[graphql_interface]
impl Character for Droid {
    async fn id(&self) -> &str {
        &self.id
    }
}

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

    #[graphql(ignore)] // or `#[graphql(skip)]`, your choice
    fn ignored(&self) -> u32 { 0 }
}

#[derive(GraphQLObject)]
#[graphql(impl = CharacterValue)]
struct Human {
    id: String,
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


### Fields, arguments and interface customization

Similarly to [GraphQL objects][5] Juniper allows to fully customize [interface][1] fields and their arguments.

```rust
# #![allow(deprecated)]
# extern crate juniper;
use juniper::graphql_interface;

// Renames the interface in GraphQL schema.
#[graphql_interface(name = "MyCharacter")] 
// Describes the interface in GraphQL schema.
#[graphql_interface(description = "My own character.")]
// Usual Rust docs are supported too as GraphQL interface description, 
// but `description` attribute argument takes precedence over them, if specified.
/// This doc is absent in GraphQL schema.  
trait Character {
    // Renames the field in GraphQL schema.
    #[graphql(name = "myId")]
    // Deprecates the field in GraphQL schema.
    // Usual Rust `#[deprecated]` attribute is supported too as field deprecation,
    // but `deprecated` attribute argument takes precedence over it, if specified.
    #[graphql(deprecated = "Do not use it.")]
    // Describes the field in GraphQL schema.
    #[graphql(description = "ID of my own character.")]
    // Usual Rust docs are supported too as field description, 
    // but `description` attribute argument takes precedence over them, if specified.
    /// This description is absent in GraphQL schema.  
    fn id(
        &self,
        // Renames the argument in GraphQL schema.
        #[graphql(name = "myNum")]
        // Describes the argument in GraphQL schema.
        #[graphql(description = "ID number of my own character.")]
        // Specifies the default value for the argument.
        // The concrete value may be omitted, and the `Default::default` one 
        // will be used in such case.
        #[graphql(default = 5)]
        num: i32,
    ) -> &str;
}
#
# fn main() {}
```


### Custom context

If a [`Context`][6] is required in a trait method to resolve a [GraphQL interface][1] field, specify it as an argument.

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
    fn name(&self, #[graphql(context)] db: &Database) -> Option<&str>;
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

If an [`Executor`][4] is required in a trait method to resolve a [GraphQL interface][1] field, specify it as an argument.

This requires to explicitly parametrize over [`ScalarValue`][3], as [`Executor`][4] does so. 

```rust
# extern crate juniper;
use juniper::{graphql_interface, Executor, GraphQLObject, LookAheadMethods as _, ScalarValue};

#[graphql_interface(for = Human, Scalar = S)] // notice specifying `ScalarValue` as existing type parameter
trait Character<S: ScalarValue> {             
    // If a field argument is named `executor`, it's automatically assumed
    // as an executor argument.
    async fn id<'a>(&self, executor: &'a Executor<'_, '_, (), S>) -> &'a str
    where
        S: Send + Sync; // required by `#[async_trait]` transformation ¯\_(ツ)_/¯

    // Otherwise, you may mark it explicitly as an executor argument.
    async fn name<'b>(
        &'b self,
        #[graphql(executor)] another: &Executor<'_, '_, (), S>,
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
#[graphql_interface(scalar = S)]
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


### Downcasting

By default, the [GraphQL interface][1] value is downcast to one of its implementer types via matching the enum variant or downcasting the trait object (if `dyn` macro argument is used).

However, if some custom logic is needed to downcast a [GraphQL interface][1] implementer, you may specify either an external function or a trait method to do so.

```rust
# extern crate juniper;
# use std::collections::HashMap;
use juniper::{graphql_interface, GraphQLObject};

struct Database {
    droids: HashMap<String, Droid>,
}
impl juniper::Context for Database {}

#[graphql_interface(for = [Human, Droid], context = Database)]
#[graphql_interface(on Droid = get_droid)] // enables downcasting `Droid` via `get_droid()` function
trait Character {
    fn id(&self) -> &str;

    #[graphql(downcast)] // makes method a downcast to `Human`, not a field 
    // NOTICE: The method signature may optionally contain `&Database` context argument.
    fn as_human(&self) -> Option<&Human> {
        None
    }
}

#[derive(GraphQLObject)]
#[graphql(impl = CharacterValue, Context = Database)]
struct Human {
    id: String,
}
#[graphql_interface]
impl Character for Human {
    fn id(&self) -> &str {
        &self.id
    }

    fn as_human(&self) -> Option<&Self> {
        Some(self)
    }   
}

#[derive(GraphQLObject)]
#[graphql(impl = CharacterValue, Context = Database)]
struct Droid {
    id: String,
}
#[graphql_interface]
impl Character for Droid {
    fn id(&self) -> &str {
        &self.id
    }
}

// External downcast function doesn't have to be a method of a type.
// It's only a matter of the function signature to match the requirements.
fn get_droid<'db>(ch: &CharacterValue, db: &'db Database) -> Option<&'db Droid> {
    db.droids.get(ch.id())
}
#
# fn main() {}
```

The attribute syntax `#[graphql_interface(on ImplementerType = resolver_fn)]` follows the [GraphQL syntax for downcasting interface implementer](https://spec.graphql.org/June2018/#example-5cc55).




## `ScalarValue` considerations

By default, `#[graphql_interface]` macro generates code, which is generic over a [`ScalarValue`][3] type. This may introduce a problem when at least one of [GraphQL interface][1] implementers is restricted to a concrete [`ScalarValue`][3] type in its implementation. To resolve such problem, a concrete [`ScalarValue`][3] type should be specified.

```rust
# extern crate juniper;
use juniper::{graphql_interface, DefaultScalarValue, GraphQLObject};

#[graphql_interface(for = [Human, Droid])]
#[graphql_interface(scalar = DefaultScalarValue)] // removing this line will fail compilation
trait Character {
    fn id(&self) -> &str;
}

#[derive(GraphQLObject)]
#[graphql(impl = CharacterValue, Scalar = DefaultScalarValue)]
struct Human {
    id: String,
    home_planet: String,
}
#[graphql_interface(scalar = DefaultScalarValue)]
impl Character for Human {
    fn id(&self) -> &str {
        &self.id
    }   
}

#[derive(GraphQLObject)]
#[graphql(impl = CharacterValue, Scalar = DefaultScalarValue)]
struct Droid {
    id: String,
    primary_function: String,
}
#[graphql_interface(scalar = DefaultScalarValue)]
impl Character for Droid {
    fn id(&self) -> &str {
        &self.id
    }   
}
#
# fn main() {}
```





[1]: https://spec.graphql.org/June2018/#sec-Interfaces
[2]: https://doc.rust-lang.org/reference/types/trait-object.html
[3]: https://docs.rs/juniper/latest/juniper/trait.ScalarValue.html
[4]: https://docs.rs/juniper/latest/juniper/struct.Executor.html
[5]: https://spec.graphql.org/June2018/#sec-Objects
[6]: https://docs.rs/juniper/0.14.2/juniper/trait.Context.html
