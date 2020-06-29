//! This crate supplies custom derive implementations for the
//! [juniper](https://github.com/graphql-rust/juniper) crate.
//!
//! You should not depend on juniper_codegen directly.
//! You only need the `juniper` crate.

#![doc(html_root_url = "https://docs.rs/juniper_codegen/0.14.2")]
#![recursion_limit = "1024"]

extern crate proc_macro;

mod result;
mod util;

mod derive_enum;
mod derive_input_object;
mod derive_object;
mod derive_scalar_value;
mod impl_object;
mod impl_scalar;

mod graphql_union;

use proc_macro::TokenStream;
use proc_macro_error::{proc_macro_error, ResultExt as _};
use result::GraphQLScope;

#[proc_macro_error]
#[proc_macro_derive(GraphQLEnum, attributes(graphql))]
pub fn derive_enum(input: TokenStream) -> TokenStream {
    let ast = syn::parse::<syn::DeriveInput>(input).unwrap();
    let gen = derive_enum::impl_enum(ast, GraphQLScope::DeriveEnum);
    match gen {
        Ok(gen) => gen.into(),
        Err(err) => proc_macro_error::abort!(err),
    }
}

#[proc_macro_error]
#[proc_macro_derive(GraphQLInputObject, attributes(graphql))]
pub fn derive_input_object(input: TokenStream) -> TokenStream {
    let ast = syn::parse::<syn::DeriveInput>(input).unwrap();
    let gen = derive_input_object::impl_input_object(ast, false, GraphQLScope::DeriveInputObject);
    match gen {
        Ok(gen) => gen.into(),
        Err(err) => proc_macro_error::abort!(err),
    }
}

#[proc_macro_error]
#[proc_macro_derive(GraphQLInputObjectInternal, attributes(graphql))]
#[doc(hidden)]
pub fn derive_input_object_internal(input: TokenStream) -> TokenStream {
    let ast = syn::parse::<syn::DeriveInput>(input).unwrap();
    let gen = derive_input_object::impl_input_object(ast, true, GraphQLScope::DeriveInputObject);
    match gen {
        Ok(gen) => gen.into(),
        Err(err) => proc_macro_error::abort!(err),
    }
}

#[proc_macro_error]
#[proc_macro_derive(GraphQLObject, attributes(graphql))]
pub fn derive_object(input: TokenStream) -> TokenStream {
    let ast = syn::parse::<syn::DeriveInput>(input).unwrap();
    let gen = derive_object::build_derive_object(ast, false, GraphQLScope::DeriveObject);
    match gen {
        Ok(gen) => gen.into(),
        Err(err) => proc_macro_error::abort!(err),
    }
}

#[proc_macro_error]
#[proc_macro_derive(GraphQLObjectInternal, attributes(graphql))]
pub fn derive_object_internal(input: TokenStream) -> TokenStream {
    let ast = syn::parse::<syn::DeriveInput>(input).unwrap();
    let gen = derive_object::build_derive_object(ast, true, GraphQLScope::DeriveObject);
    match gen {
        Ok(gen) => gen.into(),
        Err(err) => proc_macro_error::abort!(err),
    }
}

/// This custom derive macro implements the #[derive(GraphQLScalarValue)]
/// derive.
///
/// This can be used for two purposes.
///
/// ## Transparent Newtype Wrapper
///
/// Sometimes, you want to create a custerm scalar type by wrapping
/// an existing type. In Rust, this is often called the "newtype" pattern.
/// Thanks to this custom derive, this becomes really easy:
///
/// ```rust
/// // Deriving GraphQLScalar is all that is required.
/// #[derive(juniper::GraphQLScalarValue)]
/// struct UserId(String);
///
/// #[derive(juniper::GraphQLObject)]
/// struct User {
///   id: UserId,
/// }
/// ```
///
/// The type can also be customized.
///
/// ```rust
/// /// Doc comments are used for the GraphQL type description.
/// #[derive(juniper::GraphQLScalarValue)]
/// #[graphql(
///    transparent,
///    // Set a custom GraphQL name.
///    name= "MyUserId",
///    // A description can also specified in the attribute.
///    // This will the doc comment, if one exists.
///    description = "...",
/// )]
/// struct UserId(String);
/// ```
///
/// ### Base ScalarValue Enum
///
/// TODO: write documentation.
///
#[proc_macro_error]
#[proc_macro_derive(GraphQLScalarValue, attributes(graphql))]
pub fn derive_scalar_value(input: TokenStream) -> TokenStream {
    let ast = syn::parse::<syn::DeriveInput>(input).unwrap();
    let gen = derive_scalar_value::impl_scalar_value(&ast, GraphQLScope::DeriveScalar);
    match gen {
        Ok(gen) => gen.into(),
        Err(err) => proc_macro_error::abort!(err),
    }
}

/**
The `object` proc macro is the primary way of defining GraphQL resolvers
that can not be implemented with the GraphQLObject derive.

It enables you to write GraphQL field resolvers for a type by declaring a
regular Rust `impl` block. Under the hood, the procedural macro implements
the GraphQLType trait.

`object` comes with many features that allow customization of
your fields, all of which are detailed below.

### Getting Started

This simple example will show you the most basic use of `object`.
More advanced use cases are introduced step by step.

```
// So we can declare it as a plain struct without any members.
struct Query;

// We prefix the impl Block with the procedural macro.
#[juniper::graphql_object]
impl Query {

    // A **warning**: only GraphQL fields can be specified in this impl block.
    // If you want to define normal methods on the struct,
    // you have to do so in a separate, normal `impl` block.


    // This defines a simple, static field which does not require any context.
    // You can return any value that implements the `GraphQLType` trait.
    // This trait is implemented for:
    //  - basic scalar types like bool, &str, String, i32, f64
    //  - GraphQL compatible wrappers like Option<_>, Vec<_>.
    //  - types which use the `#derive[juniper::GraphQLObject]`
    //  - `object` structs.
    //
    // An important note regarding naming:
    // By default, field names will be converted to camel case.
    // For your GraphQL queries, the field will be available as `apiVersion`.
    //
    // You can also manually customize the field name if required. (See below)
    fn api_version() -> &'static str {
        "0.1"
    }

    // This field takes two arguments.
    // GraphQL arguments are just regular function parameters.
    // **Note**: in Juniper, arguments are non-nullable by default.
    //           for optional arguments, you have to specify them with Option<T>.
    fn add(a: f64, b: f64, c: Option<f64>) -> f64 {
        a + b + c.unwrap_or(0.0)
    }
}
```

## Accessing self

```
struct Person {
    first_name: String,
    last_name: String,
}

impl Person {
    // The full name method is useful outside of GraphQL,
    // so we define it as a normal method.
    fn build_full_name(&self) -> String {
        format!("{} {}", self.first_name, self.last_name)
    }
}

#[juniper::graphql_object]
impl Person {
    fn first_name(&self) -> &str {
        &self.first_name
    }

    fn last_name(&self) -> &str {
        &self.last_name
    }

    fn full_name(&self) -> String {
        self.build_full_name()
    }
}
```

## Context (+ Executor)

You can specify a context that will be available across
all your resolvers during query execution.

The Context can be injected into your resolvers by just
specifying an argument with the same type as the context
(but as a reference).

```

# #[derive(juniper::GraphQLObject)] struct User { id: i32 }
# struct DbPool;
# impl DbPool { fn user(&self, id: i32) -> Option<User> { unimplemented!() } }

struct Context {
    db: DbPool,
}

// Mark our struct for juniper.
impl juniper::Context for Context {}

struct Query;

#[juniper::graphql_object(
    // Here we specify the context type for this object.
    Context = Context,
)]
impl Query {
    // Context is injected by specifying a argument
    // as a reference to the Context.
    fn user(context: &Context, id: i32) -> Option<User> {
        context.db.user(id)
    }

    // You can also gain access to the executor, which
    // allows you to do look aheads.
    fn with_executor(executor: &Executor) -> bool {
        let info = executor.look_ahead();
        // ...
        true
    }
}

```

## Customization (Documentation, Renaming, ...)

```
struct InternalQuery;

// Doc comments can be used to specify graphql documentation.
/// GRAPHQL DOCUMENTATION.
/// More info for GraphQL users....
#[juniper::graphql_object(
    // You can rename the type for GraphQL by specifying the name here.
    name = "Query",
    // You can also specify a description here.
    // If present, doc comments will be ignored.
    description = "...",
)]
impl InternalQuery {
    // Documentation doc comments also work on fields.
    /// GraphQL description...
    fn field_with_description() -> bool { true }

    // Fields can also be customized with the #[graphql] attribute.
    #[graphql(
        // overwrite the public name
        name = "actualFieldName",
        // Can be used instead of doc comments.
        description = "field description",
    )]
    fn internal_name() -> bool { true }

    // Fields can be deprecated too.
    #[graphql(
        deprecated = "deprecatin info...",
        // Note: just "deprecated," without a description works too.
    )]
    fn deprecated_field_simple() -> bool { true }


    // Customizing field arguments is a little awkward right now.
    // This will improve once [RFC 2564](https://github.com/rust-lang/rust/issues/60406)
    // is implemented, which will allow attributes on function parameters.

    #[graphql(
        arguments(
            arg1(
                // You can specify default values.
                // A default can be any valid expression that yields the right type.
                default = true,
                description = "Argument description....",
            ),
            arg2(
                default = false,
                description = "arg2 description...",
            ),
        ),
    )]
    fn args(arg1: bool, arg2: bool) -> bool {
        arg1 && arg2
    }
}
```

## Lifetimes, Generics and custom Scalars

Lifetimes work just like you'd expect.


```
struct WithLifetime<'a> {
    value: &'a str,
}

#[juniper::graphql_object]
impl<'a> WithLifetime<'a> {
    fn value(&self) -> &str {
        self.value
    }
}

```

Juniper has support for custom scalars.
Mostly you will only need the default scalar type juniper::DefaultScalarValue.

You can easily specify a custom scalar though.


```

# type MyCustomScalar = juniper::DefaultScalarValue;

struct Query;

#[juniper::graphql_object(
    Scalar = MyCustomScalar,
)]
impl Query {
    fn test(&self) -> i32 {
        0
    }
}
```

## Raw identifiers

You can use [raw identifiers](https://doc.rust-lang.org/stable/edition-guide/rust-2018/module-system/raw-identifiers.html)
if you want a GrahpQL field that happens to be a Rust keyword:

```
struct User {
    r#type: String,
}

#[juniper::graphql_object]
impl User {
    fn r#type(&self) -> &str {
        &self.r#type
    }
}
```

*/
#[proc_macro_error]
#[proc_macro_attribute]
pub fn graphql_object(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = proc_macro2::TokenStream::from(args);
    let input = proc_macro2::TokenStream::from(input);
    TokenStream::from(impl_object::build_object(
        args,
        input,
        false,
        GraphQLScope::ImplObject,
    ))
}

/// A proc macro for defining a GraphQL object.
#[proc_macro_error]
#[proc_macro_attribute]
#[doc(hidden)]
pub fn graphql_object_internal(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = proc_macro2::TokenStream::from(args);
    let input = proc_macro2::TokenStream::from(input);
    TokenStream::from(impl_object::build_object(
        args,
        input,
        true,
        GraphQLScope::ImplObject,
    ))
}

/// Expose GraphQL scalars
///
/// The GraphQL language defines a number of built-in scalars: strings, numbers, and
/// booleans. This macro can be used either to define new types of scalars (e.g.
/// timestamps), or expose other types as one of the built-in scalars (e.g. bigints
/// as numbers or strings).
///
/// Since the preferred transport protocol for GraphQL responses is JSON, most
/// custom scalars will be transferred as strings. You therefore need to ensure that
/// the client library you are sending data to can parse the custom value into a
/// datatype appropriate for that platform.
///
/// By default the trait is implemented in terms of the default scalar value
/// representation provided by juniper. If that does not fit your needs it is
/// possible to specify a custom representation.
///
/// ```rust
/// // The data type
/// struct UserID(String);
///
/// #[juniper::graphql_scalar(
///     // You can rename the type for GraphQL by specifying the name here.    
///     name = "MyName",
///     // You can also specify a description here.
///     // If present, doc comments will be ignored.
///     description = "An opaque identifier, represented as a string")]
/// impl<S> GraphQLScalar for UserID
/// where
///     S: juniper::ScalarValue
///  {
///     fn resolve(&self) -> juniper::Value {
///         juniper::Value::scalar(self.0.to_owned())
///     }
///
///     fn from_input_value(value: &juniper::InputValue) -> Option<UserID> {
///         value.as_string_value().map(|s| UserID(s.to_owned()))
///     }
///
///     fn from_str<'a>(value: juniper::ScalarToken<'a>) -> juniper::ParseScalarResult<'a, S> {
///         <String as juniper::ParseScalarValue<S>>::from_str(value)
///     }
/// }
///
/// # fn main() { }
/// ```
///
/// In addition to implementing `GraphQLType` for the type in question,
/// `FromInputValue` and `ToInputValue` is also implemented. This makes the type
/// usable as arguments and default values.
#[proc_macro_error]
#[proc_macro_attribute]
pub fn graphql_scalar(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = proc_macro2::TokenStream::from(args);
    let input = proc_macro2::TokenStream::from(input);
    let gen = impl_scalar::build_scalar(args, input, GraphQLScope::ImplScalar);
    match gen {
        Ok(gen) => gen.into(),
        Err(err) => proc_macro_error::abort!(err),
    }
}

/// A proc macro for defining a GraphQL subscription.
#[proc_macro_error]
#[proc_macro_attribute]
pub fn graphql_subscription(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = proc_macro2::TokenStream::from(args);
    let input = proc_macro2::TokenStream::from(input);
    TokenStream::from(impl_object::build_subscription(
        args,
        input,
        false,
        GraphQLScope::ImplObject,
    ))
}

#[proc_macro_error]
#[proc_macro_attribute]
#[doc(hidden)]
pub fn graphql_subscription_internal(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = proc_macro2::TokenStream::from(args);
    let input = proc_macro2::TokenStream::from(input);
    TokenStream::from(impl_object::build_subscription(
        args,
        input,
        true,
        GraphQLScope::ImplObject,
    ))
}

/// `#[derive(GraphQLUnion)]` macro for deriving a [GraphQL union][1] implementation for enums and
/// structs.
///
/// The `#[graphql]` helper attribute is used for configuring the derived implementation. Specifying
/// multiple `#[graphql]` attributes on the same definition is totally okay. They all will be
/// treated as a single attribute.
///
/// ```
/// use derive_more::From;
/// use juniper::{GraphQLObject, GraphQLUnion};
///
/// #[derive(GraphQLObject)]
/// struct Human {
///     id: String,
///     home_planet: String,
/// }
///
/// #[derive(GraphQLObject)]
/// struct Droid {
///     id: String,
///     primary_function: String,
/// }
///
/// #[derive(From, GraphQLUnion)]
/// enum CharacterEnum {
///     Human(Human),
///     Droid(Droid),
/// }
/// ```
///
/// # Custom name and description
///
/// The name of [GraphQL union][1] may be overriden with a `name` attribute's argument. By default,
/// a type name is used.
///
/// The description of [GraphQL union][1] may be specified either with a `description`/`desc`
/// attribute's argument, or with a regular Rust doc comment.
///
/// ```
/// # use juniper::{GraphQLObject, GraphQLUnion};
/// #
/// # #[derive(GraphQLObject)]
/// # struct Human {
/// #    id: String,
/// #    home_planet: String,
/// # }
/// #
/// # #[derive(GraphQLObject)]
/// # struct Droid {
/// #     id: String,
/// #     primary_function: String,
/// # }
/// #
/// #[derive(GraphQLUnion)]
/// #[graphql(name = "Character", desc = "Possible episode characters.")]
/// enum Chrctr {
///     Human(Human),
///     Droid(Droid),
/// }
///
/// // NOTICE: Rust docs are used as GraphQL description.
/// /// Possible episode characters.
/// #[derive(GraphQLUnion)]
/// enum CharacterWithDocs {
///     Human(Human),
///     Droid(Droid),
/// }
///
/// // NOTICE: `description` argument takes precedence over Rust docs.
/// /// Not a GraphQL description anymore.
/// #[derive(GraphQLUnion)]
/// #[graphql(description = "Possible episode characters.")]
/// enum CharacterWithDescription {
///     Human(Human),
///     Droid(Droid),
/// }
/// ```
///
/// # Custom context
///
/// By default, the generated implementation uses [unit type `()`][4] as context. To use a custom
/// context type for [GraphQL union][1] variants types or external resolver functions, specify it
/// with `context`/`Context` attribute's argument.
///
/// ```
/// # use juniper::{GraphQLObject, GraphQLUnion};
/// #
/// #[derive(GraphQLObject)]
/// #[graphql(Context = CustomContext)]
/// struct Human {
///     id: String,
///     home_planet: String,
/// }
///
/// #[derive(GraphQLObject)]
/// #[graphql(Context = CustomContext)]
/// struct Droid {
///     id: String,
///     primary_function: String,
/// }
///
/// pub struct CustomContext;
/// impl juniper::Context for CustomContext {}
///
/// #[derive(GraphQLUnion)]
/// #[graphql(Context = CustomContext)]
/// enum Character {
///     Human(Human),
///     Droid(Droid),
/// }
/// ```
///
/// # Custom `ScalarValue`
///
/// By default, this macro generates code, which is generic over a `ScalarValue` type.
/// This may introduce a problem when at least one of [GraphQL union][1] variants is restricted to a
/// concrete `ScalarValue` type in its implementation. To resolve such problem, a concrete
/// `ScalarValue` type should be specified with a `scalar`/`Scalar`/`ScalarValue` attribute's
/// argument.
///
/// ```
/// # use juniper::{DefaultScalarValue, GraphQLObject, GraphQLUnion};
/// #
/// #[derive(GraphQLObject)]
/// #[graphql(Scalar = DefaultScalarValue)]
/// struct Human {
///     id: String,
///     home_planet: String,
/// }
///
/// #[derive(GraphQLObject)]
/// struct Droid {
///     id: String,
///     primary_function: String,
/// }
///
/// // NOTICE: Removing `Scalar` argument will fail compilation.
/// #[derive(GraphQLUnion)]
/// #[graphql(Scalar = DefaultScalarValue)]
/// enum Character {
///     Human(Human),
///     Droid(Droid),
/// }
/// ```
///
/// # Ignoring enum variants
///
/// To omit exposing an enum variant in the GraphQL schema, use an `ignore`/`skip` attribute's
/// argument directly on that variant.
///
/// > __WARNING__:
/// > It's the _library user's responsibility_ to ensure that ignored enum variant is _never_
/// > returned from resolvers, otherwise resolving the GraphQL query will __panic at runtime__.
///
/// ```
/// # use std::marker::PhantomData;
/// use derive_more::From;
/// use juniper::{GraphQLObject, GraphQLUnion};
///
/// #[derive(GraphQLObject)]
/// struct Human {
///     id: String,
///     home_planet: String,
/// }
///
/// #[derive(GraphQLObject)]
/// struct Droid {
///     id: String,
///     primary_function: String,
/// }
///
/// #[derive(From, GraphQLUnion)]
/// enum Character<S> {
///     Human(Human),
///     Droid(Droid),
///     #[from(ignore)]
///     #[graphql(ignore)]  // or `#[graphql(skip)]`, your choice
///     _State(PhantomData<S>),
/// }
/// ```
///
/// # External resolver functions
///
/// To use a custom logic for resolving a [GraphQL union][1] variant, an external resolver function
/// may be specified with:
/// - either a `with` attribute's argument on an enum variant;
/// - or an `on` attribute's argument on an enum/struct itself.
///
/// ```
/// # use juniper::{GraphQLObject, GraphQLUnion};
/// #
/// #[derive(GraphQLObject)]
/// #[graphql(Context = CustomContext)]
/// struct Human {
///     id: String,
///     home_planet: String,
/// }
///
/// #[derive(GraphQLObject)]
/// #[graphql(Context = CustomContext)]
/// struct Droid {
///     id: String,
///     primary_function: String,
/// }
///
/// pub struct CustomContext {
///     droid: Droid,
/// }
/// impl juniper::Context for CustomContext {}
///
/// #[derive(GraphQLUnion)]
/// #[graphql(Context = CustomContext)]
/// enum Character {
///     Human(Human),
///     #[graphql(with = Character::droid_from_context)]
///     Droid(Droid),
/// }
///
/// impl Character {
///     // NOTICE: The function signature must contain `&self` and `&Context`,
///     //         and return `Option<&VariantType>`.
///     fn droid_from_context<'c>(&self, ctx: &'c CustomContext) -> Option<&'c Droid> {
///         Some(&ctx.droid)
///     }
/// }
///
/// #[derive(GraphQLUnion)]
/// #[graphql(Context = CustomContext)]
/// #[graphql(on Droid = CharacterWithoutDroid::droid_from_context)]
/// enum CharacterWithoutDroid {
///     Human(Human),
///     #[graphql(ignore)]
///     Droid,
/// }
///
/// impl CharacterWithoutDroid {
///     fn droid_from_context<'c>(&self, ctx: &'c CustomContext) -> Option<&'c Droid> {
///         if let Self::Droid = self {
///             Some(&ctx.droid)
///         } else {
///             None
///         }
///     }
/// }
/// ```
///
/// # Deriving structs
///
/// Specifying external resolver functions is mandatory for using a struct as a [GraphQL union][1],
/// because this is the only way to declare [GraphQL union][1] variants in this case.
///
/// ```
/// # use std::collections::HashMap;
/// # use juniper::{GraphQLObject, GraphQLUnion};
/// #
/// #[derive(GraphQLObject)]
/// #[graphql(Context = Database)]
/// struct Human {
///     id: String,
///     home_planet: String,
/// }
///
/// #[derive(GraphQLObject)]
/// #[graphql(Context = Database)]
/// struct Droid {
///     id: String,
///     primary_function: String,
/// }
///
/// struct Database {
///     humans: HashMap<String, Human>,
///     droids: HashMap<String, Droid>,
/// }
/// impl juniper::Context for Database {}
///
/// #[derive(GraphQLUnion)]
/// #[graphql(
///     Context = Database,
///     on Human = Character::get_human,
///     on Droid = Character::get_droid,
/// )]
/// struct Character {
///     id: String,
/// }
///
/// impl Character {
///     fn get_human<'db>(&self, ctx: &'db Database) -> Option<&'db Human>{
///         ctx.humans.get(&self.id)
///     }
///
///     fn get_droid<'db>(&self, ctx: &'db Database) -> Option<&'db Droid>{
///         ctx.droids.get(&self.id)
///     }
/// }
/// ```
///
/// [1]: https://spec.graphql.org/June2018/#sec-Unions
/// [4]: https://doc.rust-lang.org/stable/std/primitive.unit.html
#[proc_macro_error]
#[proc_macro_derive(GraphQLUnion, attributes(graphql))]
pub fn derive_union(input: TokenStream) -> TokenStream {
    self::graphql_union::derive::expand(input.into())
        .unwrap_or_abort()
        .into()
}

/// `#[graphql_union]` macro for deriving a [GraphQL union][1] implementation for traits.
///
/// Specifying multiple `#[graphql_union]` attributes on the same definition is totally okay. They
/// all will be treated as a single attribute.
///
/// A __trait has to be [object safe][2]__, because schema resolvers will need to return a
/// [trait object][3] to specify a [GraphQL union][1] behind it. The [trait object][3] has to be
/// [`Send`] and [`Sync`].
///
/// ```
/// use juniper::{graphql_union, GraphQLObject};
///
/// #[derive(GraphQLObject)]
/// struct Human {
///     id: String,
///     home_planet: String,
/// }
///
/// #[derive(GraphQLObject)]
/// struct Droid {
///     id: String,
///     primary_function: String,
/// }
///
/// #[graphql_union]
/// trait Character {
///     // NOTICE: The method signature must contain `&self` and return `Option<&VariantType>`.
///     fn as_human(&self) -> Option<&Human> { None }
///     fn as_droid(&self) -> Option<&Droid> { None }
/// }
///
/// impl Character for Human {
///     fn as_human(&self) -> Option<&Human> { Some(&self) }
/// }
///
/// impl Character for Droid {
///     fn as_droid(&self) -> Option<&Droid> { Some(&self) }
/// }
/// ```
///
/// # Custom name and description
///
/// The name of [GraphQL union][1] may be overriden with a `name` attribute's argument. By default,
/// a type name is used.
///
/// The description of [GraphQL union][1] may be specified either with a `description`/`desc`
/// attribute's argument, or with a regular Rust doc comment.
///
/// ```
/// # use juniper::{graphql_union, GraphQLObject};
/// #
/// # #[derive(GraphQLObject)]
/// # struct Human {
/// #    id: String,
/// #    home_planet: String,
/// # }
/// #
/// # #[derive(GraphQLObject)]
/// # struct Droid {
/// #     id: String,
/// #     primary_function: String,
/// # }
/// #
/// #[graphql_union(name = "Character", desc = "Possible episode characters.")]
/// trait Chrctr {
///     fn as_human(&self) -> Option<&Human> { None }
///     fn as_droid(&self) -> Option<&Droid> { None }
/// }
///
/// // NOTICE: Rust docs are used as GraphQL description.
/// /// Possible episode characters.
/// trait CharacterWithDocs {
///     fn as_human(&self) -> Option<&Human> { None }
///     fn as_droid(&self) -> Option<&Droid> { None }
/// }
///
/// // NOTICE: `description` argument takes precedence over Rust docs.
/// /// Not a GraphQL description anymore.
/// #[graphql_union(description = "Possible episode characters.")]
/// trait CharacterWithDescription {
///     fn as_human(&self) -> Option<&Human> { None }
///     fn as_droid(&self) -> Option<&Droid> { None }
/// }
/// #
/// # impl Chrctr for Human {}
/// # impl Chrctr for Droid {}
/// # impl CharacterWithDocs for Human {}
/// # impl CharacterWithDocs for Droid {}
/// # impl CharacterWithDescription for Human {}
/// # impl CharacterWithDescription for Droid {}
/// ```
///
/// # Custom context
///
/// By default, the generated implementation tries to infer `juniper::Context` type from signatures
/// of trait methods, and uses [unit type `()`][4] if signatures contains no context arguments.
///
/// If `juniper::Context` type cannot be inferred or is inferred incorrectly, then specify it
/// explicitly with `context`/`Context` attribute's argument.
///
/// ```
/// # use std::collections::HashMap;
/// # use juniper::{graphql_union, GraphQLObject};
/// #
/// #[derive(GraphQLObject)]
/// #[graphql(Context = Database)]
/// struct Human {
///     id: String,
///     home_planet: String,
/// }
///
/// #[derive(GraphQLObject)]
/// #[graphql(Context = Database)]
/// struct Droid {
///     id: String,
///     primary_function: String,
/// }
///
/// struct Database {
///     humans: HashMap<String, Human>,
///     droids: HashMap<String, Droid>,
/// }
/// impl juniper::Context for Database {}
///
/// #[graphql_union(Context = Database)]
/// trait Character {
///     fn as_human<'db>(&self, ctx: &'db Database) -> Option<&'db Human> { None }
///     fn as_droid<'db>(&self, ctx: &'db Database) -> Option<&'db Droid> { None }
/// }
///
/// impl Character for Human {
///     fn as_human<'db>(&self, ctx: &'db Database) -> Option<&'db Human> {
///         ctx.humans.get(&self.id)
///     }
/// }
///
/// impl Character for Droid {
///     fn as_droid<'db>(&self, ctx: &'db Database) -> Option<&'db Droid> {
///         ctx.droids.get(&self.id)
///     }
/// }
/// ```
///
/// # Custom `ScalarValue`
///
/// By default, `#[graphql_union]` macro generates code, which is generic over a `ScalarValue` type.
/// This may introduce a problem when at least one of [GraphQL union][1] variants is restricted to a
/// concrete `ScalarValue` type in its implementation. To resolve such problem, a concrete
/// `ScalarValue` type should be specified with a `scalar`/`Scalar`/`ScalarValue` attribute's
/// argument.
///
/// ```
/// # use juniper::{graphql_union, DefaultScalarValue, GraphQLObject};
/// #
/// #[derive(GraphQLObject)]
/// #[graphql(Scalar = DefaultScalarValue)]
/// struct Human {
///     id: String,
///     home_planet: String,
/// }
///
/// #[derive(GraphQLObject)]
/// struct Droid {
///     id: String,
///     primary_function: String,
/// }
///
/// // NOTICE: Removing `Scalar` argument will fail compilation.
/// #[graphql_union(Scalar = DefaultScalarValue)]
/// trait Character {
///     fn as_human(&self) -> Option<&Human> { None }
///     fn as_droid(&self) -> Option<&Droid> { None }
/// }
/// #
/// # impl Character for Human {}
/// # impl Character for Droid {}
/// ```
///
/// # Ignoring trait methods
///
/// To omit some trait method to be assumed as a [GraphQL union][1] variant and ignore it, use an
/// `ignore`/`skip` attribute's argument directly on that method.
///
/// ```
/// # use juniper::{graphql_union, GraphQLObject};
/// #
/// # #[derive(GraphQLObject)]
/// # struct Human {
/// #     id: String,
/// #     home_planet: String,
/// # }
/// #
/// # #[derive(GraphQLObject)]
/// # struct Droid {
/// #     id: String,
/// #     primary_function: String,
/// # }
/// #
/// #[graphql_union]
/// trait Character {
///     fn as_human(&self) -> Option<&Human> { None }
///     fn as_droid(&self) -> Option<&Droid> { None }
///     #[graphql_union(ignore)]  // or `#[graphql_union(skip)]`, your choice
///     fn id(&self) -> &str;
/// }
/// #
/// # impl Character for Human {
/// #     fn id(&self) -> &str { self.id.as_str() }
/// # }
/// #
/// # impl Character for Droid {
/// #     fn id(&self) -> &str { self.id.as_str() }
/// # }
/// ```
///
/// # External resolver functions
///
/// It's not mandatory to use trait methods as [GraphQL union][1] variant resolvers, and instead
/// custom functions may be specified with an `on` attribute's argument.
///
/// ```
/// # use std::collections::HashMap;
/// # use juniper::{graphql_union, GraphQLObject};
/// #
/// #[derive(GraphQLObject)]
/// #[graphql(Context = Database)]
/// struct Human {
///     id: String,
///     home_planet: String,
/// }
///
/// #[derive(GraphQLObject)]
/// #[graphql(Context = Database)]
/// struct Droid {
///     id: String,
///     primary_function: String,
/// }
///
/// struct Database {
///     humans: HashMap<String, Human>,
///     droids: HashMap<String, Droid>,
/// }
/// impl juniper::Context for Database {}
///
/// #[graphql_union(Context = Database)]
/// #[graphql_union(
///     on Human = DynCharacter::get_human,
///     on Droid = get_droid,
/// )]
/// trait Character {
///     #[graphql_union(ignore)]
///     fn id(&self) -> &str;
/// }
///
/// impl Character for Human {
///     fn id(&self) -> &str { self.id.as_str() }
/// }
///
/// impl Character for Droid {
///     fn id(&self) -> &str { self.id.as_str() }
/// }
///
/// // NOTICE: The trait object is always `Send` and `Sync`.
/// type DynCharacter<'a> = dyn Character + Send + Sync + 'a;
///
/// impl<'a> DynCharacter<'a> {
///     fn get_human<'db>(&self, ctx: &'db Database) -> Option<&'db Human> {
///         ctx.humans.get(self.id())
///     }
/// }
///
/// // NOTICE: Custom resolver function doesn't have to be a method of a type.
/// //         It's only a matter of the function signature to match the requirements.
/// fn get_droid<'db>(ch: &DynCharacter<'_>, ctx: &'db Database) -> Option<&'db Droid> {
///     ctx.droids.get(ch.id())
/// }
/// ```
///
/// [1]: https://spec.graphql.org/June2018/#sec-Unions
/// [2]: https://doc.rust-lang.org/stable/reference/items/traits.html#object-safety
/// [3]: https://doc.rust-lang.org/stable/reference/types/trait-object.html
/// [4]: https://doc.rust-lang.org/stable/std/primitive.unit.html
#[proc_macro_error]
#[proc_macro_attribute]
pub fn graphql_union(attr: TokenStream, body: TokenStream) -> TokenStream {
    self::graphql_union::attr::expand(attr.into(), body.into())
        .unwrap_or_abort()
        .into()
}
