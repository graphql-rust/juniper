Unions
======

From the server's point of view, [GraphQL unions][1] are somewhat similar to [interfaces][5] - the main difference is that they don't contain fields on their own.

The most obvious and straightforward way to represent a [GraphQL union][1] in Rust is enum. However, we also can do so either with trait or a regular struct. That's why, for implementing [GraphQL unions][1] Juniper provides:
- `#[derive(GraphQLUnion)]` macro for enums and structs.
- `#[graphql_union]` for traits.




## Enums

Most of the time, we just need a trivial and straightforward Rust enum to represent a [GraphQL union][1].

```rust
# extern crate juniper;
# extern crate derive_more;
use derive_more::From;
use juniper::{GraphQLObject, GraphQLUnion};

#[derive(GraphQLObject)]
struct Human {
    id: String,
    home_planet: String,
}

#[derive(GraphQLObject)]
struct Droid {
    id: String,
    primary_function: String,
}

#[derive(From, GraphQLUnion)]
enum Character {
    Human(Human),
    Droid(Droid),
}
#
# fn main() {}
```


### Ignoring enum variants

In some rare situations we may want to omit exposing an enum variant in the GraphQL schema.

As an example, let's consider the situation where we need to bind some type parameter `T` for doing interesting type-level stuff in our resolvers. To achieve this we need to have `PhantomData<T>`, but we don't want it exposed in the GraphQL schema.

> __WARNING__:  
> It's the _library user's responsibility_ to ensure that ignored enum variant is _never_ returned from resolvers, otherwise resolving the GraphQL query will __panic at runtime__.

```rust
# extern crate juniper;
# extern crate derive_more;
# use std::marker::PhantomData;
use derive_more::From;
use juniper::{GraphQLObject, GraphQLUnion};

#[derive(GraphQLObject)]
struct Human {
    id: String,
    home_planet: String,
}

#[derive(GraphQLObject)]
struct Droid {
    id: String,
    primary_function: String,
}

#[derive(From, GraphQLUnion)]
enum Character<S> {
    Human(Human),
    Droid(Droid),
    #[from(ignore)]
    #[graphql(ignore)]  // or `#[graphql(skip)]`, your choice
    _State(PhantomData<S>),
}
#
# fn main() {}
```


### External resolver functions

If some custom logic is needed to resolve a [GraphQL union][1] variant, you may specify an external function to do so:

```rust
# #![allow(dead_code)]
# extern crate juniper;
use juniper::{GraphQLObject, GraphQLUnion};

#[derive(GraphQLObject)]
#[graphql(Context = CustomContext)]
struct Human {
    id: String,
    home_planet: String,
}

#[derive(GraphQLObject)]
#[graphql(Context = CustomContext)]
struct Droid {
    id: String,
    primary_function: String,
}

pub struct CustomContext {
    droid: Droid,
}
impl juniper::Context for CustomContext {}

#[derive(GraphQLUnion)]
#[graphql(Context = CustomContext)]
enum Character {
    Human(Human),
    #[graphql(with = Character::droid_from_context)]
    Droid(Droid),
}

impl Character {
    // NOTICE: The function signature must contain `&self` and `&Context`,
    //         and return `Option<&VariantType>`.
    fn droid_from_context<'c>(&self, ctx: &'c CustomContext) -> Option<&'c Droid> {
        Some(&ctx.droid)
    }
}
#
# fn main() {}
```

With an external resolver function we can even declare a new [GraphQL union][1] variant where the Rust type is absent in the initial enum definition. The attribute syntax `#[graphql(on VariantType = resolver_fn)]` follows the [GraphQL syntax for dispatching union variants](https://spec.graphql.org/June2018/#example-f8163).

```rust
# #![allow(dead_code)]
# extern crate juniper;
use juniper::{GraphQLObject, GraphQLUnion};

#[derive(GraphQLObject)]
#[graphql(Context = CustomContext)]
struct Human {
    id: String,
    home_planet: String,
}

#[derive(GraphQLObject)]
#[graphql(Context = CustomContext)]
struct Droid {
    id: String,
    primary_function: String,
}

#[derive(GraphQLObject)]
#[graphql(Context = CustomContext)]
struct Ewok {
    id: String,
    is_funny: bool,
}

pub struct CustomContext {
    ewok: Ewok,
}
impl juniper::Context for CustomContext {}

#[derive(GraphQLUnion)]
#[graphql(Context = CustomContext)]
#[graphql(on Ewok = Character::ewok_from_context)]
enum Character {
    Human(Human),
    Droid(Droid),
    #[graphql(ignore)]  // or `#[graphql(skip)]`, your choice
    Ewok,
}

impl Character {
    fn ewok_from_context<'c>(&self, ctx: &'c CustomContext) -> Option<&'c Ewok> {
        if let Self::Ewok = self {
            Some(&ctx.ewok)
        } else {
            None
        }       
    }
}
#
# fn main() {}
```




## Structs

Using Rust structs as [GraphQL unions][1] is very similar to using enums, with the nuance that specifying an external resolver function is the only way to declare a [GraphQL union][1] variant.

```rust
# extern crate juniper;
# use std::collections::HashMap;
use juniper::{GraphQLObject, GraphQLUnion};

#[derive(GraphQLObject)]
#[graphql(Context = Database)]
struct Human {
    id: String,
    home_planet: String,
}

#[derive(GraphQLObject)]
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

#[derive(GraphQLUnion)]
#[graphql(
    Context = Database,
    on Human = Character::get_human,
    on Droid = Character::get_droid,
)]
struct Character {
    id: String,
}

impl Character {
    fn get_human<'db>(&self, ctx: &'db Database) -> Option<&'db Human>{
        ctx.humans.get(&self.id)
    }

    fn get_droid<'db>(&self, ctx: &'db Database) -> Option<&'db Droid>{
        ctx.droids.get(&self.id)
    }
}
#
# fn main() {}
```




## Traits

To use a Rust trait definition as a [GraphQL union][1] you need to use the `#[graphql_union]` macro. [Rust doesn't allow derive macros on traits](https://doc.rust-lang.org/stable/reference/procedural-macros.html#derive-macros), so using `#[derive(GraphQLUnion)]` on traits doesn't work.

> __NOTICE__:  
> A __trait has to be [object safe](https://doc.rust-lang.org/stable/reference/items/traits.html#object-safety)__, because schema resolvers will need to return a [trait object](https://doc.rust-lang.org/stable/reference/types/trait-object.html) to specify a [GraphQL union][1] behind it.

```rust
# extern crate juniper;
use juniper::{graphql_union, GraphQLObject};

#[derive(GraphQLObject)]
struct Human {
    id: String,
    home_planet: String,
}

#[derive(GraphQLObject)]
struct Droid {
    id: String,
    primary_function: String,
}

#[graphql_union]
trait Character {
    // NOTICE: The method signature must contain `&self` and return `Option<&VariantType>`.
    fn as_human(&self) -> Option<&Human> { None }
    fn as_droid(&self) -> Option<&Droid> { None }
}

impl Character for Human {
    fn as_human(&self) -> Option<&Human> { Some(&self) }
}

impl Character for Droid {
    fn as_droid(&self) -> Option<&Droid> { Some(&self) }
}
#
# fn main() {}
```


### Custom context

If a [`Context`][6] is required in a trait method to resolve a [GraphQL union][1] variant, specify it as an argument.

```rust
# #![allow(unused_variables)]
# extern crate juniper;
# use std::collections::HashMap;
use juniper::{graphql_union, GraphQLObject};

#[derive(GraphQLObject)]
#[graphql(Context = Database)]
struct Human {
    id: String,
    home_planet: String,
}

#[derive(GraphQLObject)]
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

#[graphql_union(context = Database)]
trait Character {
    // NOTICE: The method signature may optionally contain `&Context`.
    fn as_human<'db>(&self, ctx: &'db Database) -> Option<&'db Human> { None }
    fn as_droid<'db>(&self, ctx: &'db Database) -> Option<&'db Droid> { None }
}

impl Character for Human {
    fn as_human<'db>(&self, ctx: &'db Database) -> Option<&'db Human> {
        ctx.humans.get(&self.id)
    }
}

impl Character for Droid {
    fn as_droid<'db>(&self, ctx: &'db Database) -> Option<&'db Droid> {
        ctx.droids.get(&self.id)
    }
}
#
# fn main() {}
```


### Ignoring trait methods

As with enums, we may want to omit some trait methods to be assumed as [GraphQL union][1] variants and ignore them.

```rust
# extern crate juniper;
use juniper::{graphql_union, GraphQLObject};

#[derive(GraphQLObject)]
struct Human {
    id: String,
    home_planet: String,
}

#[derive(GraphQLObject)]
struct Droid {
    id: String,
    primary_function: String,
}

#[graphql_union]
trait Character {
    fn as_human(&self) -> Option<&Human> { None }
    fn as_droid(&self) -> Option<&Droid> { None }
    #[graphql(ignore)]  // or `#[graphql(skip)]`, your choice
    fn id(&self) -> &str;
}

impl Character for Human {
    fn as_human(&self) -> Option<&Human> { Some(&self) }
    fn id(&self) -> &str { self.id.as_str() }
}

impl Character for Droid {
    fn as_droid(&self) -> Option<&Droid> { Some(&self) }
    fn id(&self) -> &str { self.id.as_str() }
}
#
# fn main() {}
```


### External resolver functions

Similarly to enums and structs, it's not mandatory to use trait methods as [GraphQL union][1] variant resolvers. Instead, custom functions may be specified:

```rust
# extern crate juniper;
# use std::collections::HashMap;
use juniper::{graphql_union, GraphQLObject};

#[derive(GraphQLObject)]
#[graphql(Context = Database)]
struct Human {
    id: String,
    home_planet: String,
}

#[derive(GraphQLObject)]
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

#[graphql_union(context = Database)]
#[graphql_union(
    on Human = DynCharacter::get_human,
    on Droid = get_droid,
)]
trait Character {
    #[graphql(ignore)]  // or `#[graphql(skip)]`, your choice
    fn id(&self) -> &str;
}

impl Character for Human {
    fn id(&self) -> &str { self.id.as_str() }
}

impl Character for Droid {
    fn id(&self) -> &str { self.id.as_str() }
}

// The trait object is always `Send` and `Sync`.
type DynCharacter<'a> = dyn Character + Send + Sync + 'a;

impl<'a> DynCharacter<'a> {
    fn get_human<'db>(&self, ctx: &'db Database) -> Option<&'db Human> {
        ctx.humans.get(self.id())
    }
}

// External resolver function doesn't have to be a method of a type.
// It's only a matter of the function signature to match the requirements.
fn get_droid<'db>(ch: &DynCharacter<'_>, ctx: &'db Database) -> Option<&'db Droid> {
    ctx.droids.get(ch.id())
}
#
# fn main() {}
```




## `ScalarValue` considerations

By default, `#[derive(GraphQLUnion)]` and `#[graphql_union]` macros generate code, which is generic over a [`ScalarValue`][2] type. This may introduce a problem when at least one of [GraphQL union][1] variants is restricted to a concrete [`ScalarValue`][2] type in its implementation. To resolve such problem, a concrete [`ScalarValue`][2] type should be specified:

```rust
# #![allow(dead_code)]
# extern crate juniper;
use juniper::{DefaultScalarValue, GraphQLObject, GraphQLUnion};

#[derive(GraphQLObject)]
#[graphql(Scalar = DefaultScalarValue)]
struct Human {
    id: String,
    home_planet: String,
}

#[derive(GraphQLObject)]
struct Droid {
    id: String,
    primary_function: String,
}

#[derive(GraphQLUnion)]
#[graphql(Scalar = DefaultScalarValue)]  // removing this line will fail compilation
enum Character {
    Human(Human),
    Droid(Droid),
}
#
# fn main() {}
```





[1]: https://spec.graphql.org/June2018/#sec-Unions
[2]: https://docs.rs/juniper/latest/juniper/trait.ScalarValue.html
[5]: https://spec.graphql.org/June2018/#sec-Interfaces
[6]: https://docs.rs/juniper/0.14.2/juniper/trait.Context.html
