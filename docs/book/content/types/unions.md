Unions
======

From a server's point of view, [GraphQL unions][1] are similar to interfaces: the only exception is that they don't contain fields on their own.

For implementing [GraphQL union][1] Juniper provides:
- `#[derive(GraphQLUnion)]` macro for enums and structs;
- `#[graphql_union]` for traits.




## Enums

Most of the time, we need just a trivial and straightforward Rust enum to represent a [GraphQL union][1].

```rust
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

In some rare situations we may want to omit exposing enum variant in GraphQL schema.

As an example, let's consider the situation when we need to bind some type parameter for doing interesting type-level stuff in our resolvers. To achieve that, we need to carry the one with `PhantomData`, but we don't want the latest being exposed in GraphQL schema.

> __WARNING__:  
> It's _library user responsibility_ to ensure that ignored enum variant is _never_ returned from resolvers, otherwise resolving GraphQL query will __panic in runtime__.

```rust
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
    #[graphql(ignore)]  // or `#[graphql(skip)]`, on your choice
    _State(PhatomData<S>),
}
#
# fn main() {}
```


### Custom resolvers

If some custom logic should be involved to resolve a [GraphQL union][1] variant, we may specify the function responsible for that.

```rust
# #![allow(dead_code)]
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
    // NOTICE: The function signature is mandatory to accept `&self`, `&Context` 
    //         and return `Option<&VariantType>`.
    fn droid_from_context<'c>(&self, ctx: &'c CustomContext) -> Option<&'c Droid> {
        Some(&ctx.droid)
    }
}
#
# fn main() {}
```

With a custom resolver we can even declare a new [GraphQL union][1] variant, which Rust type is absent in the initial enum definition (the attribute syntax `#[graphql(on VariantType = resolver_fn)]` follows the [GraphQL syntax for dispatching union variant](https://spec.graphql.org/June2018/#example-f8163)).

```rust
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
}

impl Character {
    fn ewok_from_context<'c>(&self, ctx: &'c CustomContext) -> Option<&'c Ewok> {
        Some(&ctx.ewok)
    }
}
#
# fn main() {}
```




## Structs

Using Rust structs as [GraphQL unions][1] is very similar to using enums, with the nuance that specifying custom resolver is the only way to declare a [GraphQL union][1] variant.

```rust
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
    on Human = Character::get_human,
    on Droid = Character::get_droid,
)]
struct Character {
    id: String,
}

impl Character {
    fn get_human<'db>(&self, ctx: &'db Database) -> Option<'db Human>{
        ctx.humans.get(&self.id)
    }

    fn get_droid<'db>(&self, ctx: &'db Database) -> Option<'db Human>{
        ctx.humans.get(&self.id)
    }
}
#
# fn main() {}
```




## Traits

Sometimes it may seem very reasonable to use Rust trait for representing a [GraphQL union][1]. However, to do that, we should introduce a separate `#[graphql_union]` macro, because [Rust doesn't allow to use derive macros on traits](https://doc.rust-lang.org/stable/reference/procedural-macros.html#derive-macros) at the moment.

> __NOTICE__:  
> A __trait has to be [object safe](https://doc.rust-lang.org/stable/reference/items/traits.html#object-safety)__, because schema resolvers will need to return a [trait object](https://doc.rust-lang.org/stable/reference/types/trait-object.html) to specify a [GraphQL union][1] behind it.

```rust
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
    // NOTICE: The function signature is mandatory to accept `&self` 
    //         and return `Option<&VariantType>`.
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

If a context is required in trait method to resolve a [GraphQL union][1] variant, we may just specify it in arguments. 

```rust
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

#[graphql_union(Context = Database)]
trait Character {
    // NOTICE: The function signature, however, may optionally accept `&Context`.
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
    #[graphql_union(ignore)]  // or `#[graphql_union(skip)]`, on your choice
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


### Custom resolvers

And, of course, similarly to enums and structs, it's not mandatory to use trait methods as [GraphQL union][1] variants resolvers, the custom functions may be specified as well.

```rust
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

#[graphql_union(Context = Database)]
#[graphql_union(
    on Human = DynCharacter::get_human,
    on Droid = get_droid,
)]
trait Character {
    #[graphql_union(ignore)]  // or `#[graphql_union(skip)]`, on your choice
    fn id(&self) -> &str;
}

impl Character for Human {
    fn id(&self) -> &str { self.id.as_str() }
}

impl Character for Droid {
    fn id(&self) -> &str { self.id.as_str() }
}

// Used trait object is always `Send` and `Sync`.
type DynCharacter = dyn Character + Send + Sync;

impl DynCharacter {
    fn get_human<'db>(&self, ctx: &'db Database) -> Option<&'db Human> {
        ctx.humans.get(self.id())
    }
}

// Custom resolver function doesn't have to be a method.
// It's only a matter of the function signature to match the requirements.
fn get_droid<'db>(ch: &DynCharacter, ctx: &'db Database) -> Option<&'db Human> {
    ctx.humans.get(ch.id())
}
#
# fn main() {}
```





[1]: https://spec.graphql.org/June2018/#sec-Unions
