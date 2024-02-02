#![doc = include_str!("../README.md")]
#![recursion_limit = "1024"]

// NOTICE: Unfortunately this macro MUST be defined here, in the crate's root module, because Rust
//         doesn't allow to export `macro_rules!` macros from a `proc-macro` crate type currently,
//         and so we cannot move the definition into a sub-module and use the `#[macro_export]`
//         attribute.
/// Attempts to merge an [`Option`]ed `$field` of a `$self` struct with the same `$field` of
/// `$another` struct. If both are [`Some`], then throws a duplication error with a [`Span`] related
/// to the `$another` struct (a later one).
///
/// The type of [`Span`] may be explicitly specified as one of the [`SpanContainer`] methods.
/// By default, [`SpanContainer::span_ident`] is used.
///
/// [`Span`]: proc_macro2::Span
/// [`SpanContainer`]: crate::common::SpanContainer
/// [`SpanContainer::span_ident`]: crate::common::SpanContainer::span_ident
macro_rules! try_merge_opt {
    ($field:ident: $self:ident, $another:ident => $span:ident) => {{
        if let Some(v) = $self.$field {
            $another
                .$field
                .replace(v)
                .none_or_else(|dup| crate::common::parse::attr::err::dup_arg(&dup.$span()))?;
        }
        $another.$field
    }};

    ($field:ident: $self:ident, $another:ident) => {
        try_merge_opt!($field: $self, $another => span_ident)
    };
}

// NOTICE: Unfortunately this macro MUST be defined here, in the crate's root module, because Rust
//         doesn't allow to export `macro_rules!` macros from a `proc-macro` crate type currently,
//         and so we cannot move the definition into a sub-module and use the `#[macro_export]`
//         attribute.
/// Attempts to merge a [`HashMap`] `$field` of a `$self` struct with the same `$field` of
/// `$another` struct. If some [`HashMap`] entries are duplicated, then throws a duplication error
/// with a [`Span`] related to the `$another` struct (a later one).
///
/// The type of [`Span`] may be explicitly specified as one of the [`SpanContainer`] methods.
/// By default, [`SpanContainer::span_ident`] is used.
///
/// [`HashMap`]: std::collections::HashMap
/// [`Span`]: proc_macro2::Span
/// [`SpanContainer`]: crate::common::SpanContainer
/// [`SpanContainer::span_ident`]: crate::common::SpanContainer::span_ident
macro_rules! try_merge_hashmap {
    ($field:ident: $self:ident, $another:ident => $span:ident) => {{
        if !$self.$field.is_empty() {
            for (ty, rslvr) in $self.$field {
                $another
                    .$field
                    .insert(ty, rslvr)
                    .none_or_else(|dup| crate::common::parse::attr::err::dup_arg(&dup.$span()))?;
            }
        }
        $another.$field
    }};

    ($field:ident: $self:ident, $another:ident) => {
        try_merge_hashmap!($field: $self, $another => span_ident)
    };
}

// NOTICE: Unfortunately this macro MUST be defined here, in the crate's root module, because Rust
//         doesn't allow to export `macro_rules!` macros from a `proc-macro` crate type currently,
//         and so we cannot move the definition into a sub-module and use the `#[macro_export]`
//         attribute.
/// Attempts to merge a [`HashSet`] `$field` of a `$self` struct with the same `$field` of
/// `$another` struct. If some [`HashSet`] entries are duplicated, then throws a duplication error
/// with a [`Span`] related to the `$another` struct (a later one).
///
/// The type of [`Span`] may be explicitly specified as one of the [`SpanContainer`] methods.
/// By default, [`SpanContainer::span_ident`] is used.
///
/// [`HashSet`]: std::collections::HashSet
/// [`Span`]: proc_macro2::Span
/// [`SpanContainer`]: crate::common::SpanContainer
/// [`SpanContainer::span_ident`]: crate::common::SpanContainer::span_ident
macro_rules! try_merge_hashset {
    ($field:ident: $self:ident, $another:ident => $span:ident) => {{
        if !$self.$field.is_empty() {
            for ty in $self.$field {
                $another
                    .$field
                    .replace(ty)
                    .none_or_else(|dup| crate::common::parse::attr::err::dup_arg(&dup.$span()))?;
            }
        }
        $another.$field
    }};

    ($field:ident: $self:ident, $another:ident) => {
        try_merge_hashset!($field: $self, $another => span_ident)
    };
}

mod common;
mod graphql_enum;
mod graphql_input_object;
mod graphql_interface;
mod graphql_object;
mod graphql_scalar;
mod graphql_subscription;
mod graphql_union;
mod scalar_value;

use proc_macro::TokenStream;

use self::common::diagnostic::{self, ResultExt as _};

/// `#[derive(GraphQLInputObject)]` macro for deriving a
/// [GraphQL input object][0] implementation for a Rust struct. Each
/// non-ignored field type must itself be [GraphQL input object][0] or a
/// [GraphQL scalar][2].
///
/// The `#[graphql]` helper attribute is used for configuring the derived
/// implementation. Specifying multiple `#[graphql]` attributes on the same
/// definition is totally okay. They all will be treated as a single attribute.
///
/// ```rust
/// use juniper::GraphQLInputObject;
///
/// #[derive(GraphQLInputObject)]
/// struct Point2D {
///     x: f64,
///     y: f64,
/// }
/// ```
///
/// # Custom name and description
///
/// The name of a [GraphQL input object][0] or its [fields][1] may be overridden
/// with the `name` attribute's argument. By default, a type name or a struct
/// field name is used in a `camelCase`.
///
/// The description of a [GraphQL input object][0] or its [fields][1] may be
/// specified either with the `description`/`desc` attribute's argument, or with
/// a regular Rust doc comment.
///
/// ```rust
/// # use juniper::GraphQLInputObject;
/// #
/// #[derive(GraphQLInputObject)]
/// #[graphql(
///     // Rename the type for GraphQL by specifying the name here.
///     name = "Point",
///     // You may also specify a description here.
///     // If present, doc comments will be ignored.
///     desc = "A point is the simplest two-dimensional primitive.",
/// )]
/// struct Point2D {
///     /// Abscissa value.
///     x: f64,
///
///     #[graphql(name = "y", desc = "Ordinate value")]
///     y_coord: f64,
/// }
/// ```
///
/// # Renaming policy
///
/// By default, all [GraphQL input object fields][1] are renamed in a
/// `camelCase` manner (so a `y_coord` Rust struct field becomes a
/// `yCoord` [value][1] in GraphQL schema, and so on). This complies with
/// default GraphQL naming conventions as [demonstrated in spec][0].
///
/// However, if you need for some reason another naming convention, it's
/// possible to do so by using the `rename_all` attribute's argument. At the
/// moment, it supports the following policies only: `SCREAMING_SNAKE_CASE`,
/// `camelCase`, `none` (disables any renaming).
///
/// ```rust
/// # use juniper::GraphQLInputObject;
/// #
/// #[derive(GraphQLInputObject)]
/// #[graphql(rename_all = "none")] // disables renaming
/// struct Point2D {
///     x: f64,
///     y_coord: f64, // will be `y_coord` instead of `yCoord` in GraphQL schema
/// }
/// ```
///
/// # Ignoring fields
///
/// To omit exposing a Rust field in a GraphQL schema, use the `ignore`
/// attribute's argument directly on that field. Ignored fields must implement
/// [`Default`] or have the `default = <expression>` attribute's argument.
///
/// ```rust
/// # use juniper::GraphQLInputObject;
/// #
/// enum System {
///     Cartesian,
/// }
///
/// #[derive(GraphQLInputObject)]
/// struct Point2D {
///     x: f64,
///     y: f64,
///     #[graphql(ignore, default = System::Cartesian)]
///     //                ^^^^^^^^^^^^^^^^^^^^^^^^^^^
///     // This attribute is required, as we need to be able to construct
///     // a `Point2D` value from the `{ x: 0.0, y: 0.0 }` GraphQL input value,
///     // received from client-side.
///     system: System,
///     // `Default::default()` value is used, if no
///     // `#[graphql(default = <expression>)]` is specified.
///     #[graphql(skip)]
///     //        ^^^^ alternative naming, up to your preference
///     shift: f64,
/// }
/// ```
///
/// [`ScalarValue`]: juniper::ScalarValue
/// [0]: https://spec.graphql.org/October2021#sec-Input-Objects
/// [1]: https://spec.graphql.org/October2021#InputFieldsDefinition
/// [2]: https://spec.graphql.org/October2021#sec-Scalars
#[proc_macro_derive(GraphQLInputObject, attributes(graphql))]
pub fn derive_input_object(input: TokenStream) -> TokenStream {
    diagnostic::entry_point(|| {
        graphql_input_object::derive::expand(input.into())
            .unwrap_or_abort()
            .into()
    })
}

/// `#[derive(GraphQLEnum)]` macro for deriving a [GraphQL enum][0]
/// implementation for Rust enums.
///
/// The `#[graphql]` helper attribute is used for configuring the derived
/// implementation. Specifying multiple `#[graphql]` attributes on the same
/// definition is totally okay. They all will be treated as a single attribute.
///
/// ```rust
/// use juniper::GraphQLEnum;
///
/// #[derive(GraphQLEnum)]
/// enum Episode {
///     NewHope,
///     Empire,
///     Jedi,
/// }
/// ```
///
/// # Custom name, description and deprecation
///
/// The name of a [GraphQL enum][0] or its [values][1] may be overridden with
/// the `name` attribute's argument. By default, a type name is used or a
/// variant name in `SCREAMING_SNAKE_CASE`.
///
/// The description of a [GraphQL enum][0] or its [values][1] may be specified
/// either with the `description`/`desc` attribute's argument, or with a regular
/// Rust doc comment.
///
/// [GraphQL enum value][1] may be deprecated by specifying the `deprecated`
/// attribute's argument, or with regular a Rust `#[deprecated]` attribute.
///
/// ```rust
/// # #![allow(deprecated)]
/// #
/// # use juniper::GraphQLEnum;
/// #
/// #[derive(GraphQLEnum)]
/// #[graphql(
///     // Rename the type for GraphQL by specifying the name here.
///     name = "AvailableEpisodes",
///     // You may also specify a description here.
///     // If present, doc comments will be ignored.
///     desc = "Possible episodes.",
/// )]
/// enum Episode {
///     /// Doc comment, also acting as description.
///     #[deprecated(note = "Don't use it")]
///     NewHope,
///
///     #[graphql(name = "Jedi", desc = "Arguably the best one in the trilogy")]
///     #[graphql(deprecated = "Don't use it")]
///     Jedai,
///
///     Empire,
/// }
/// ```
///
/// # Renaming policy
///
/// By default, all [GraphQL enum values][1] are renamed in a
/// `SCREAMING_SNAKE_CASE` manner (so a `NewHope` Rust enum variant becomes a
/// `NEW_HOPE` [value][1] in GraphQL schema, and so on). This complies with
/// default GraphQL naming conventions as [demonstrated in spec][0].
///
/// However, if you need for some reason another naming convention, it's
/// possible to do so by using the `rename_all` attribute's argument. At the
/// moment, it supports the following policies only: `SCREAMING_SNAKE_CASE`,
/// `camelCase`, `none` (disables any renaming).
///
/// ```rust
/// # use juniper::GraphQLEnum;
/// #
/// #[derive(GraphQLEnum)]
/// #[graphql(rename_all = "none")] // disables renaming
/// enum Episode {
///     NewHope,
///     Empire,
///     Jedi,
/// }
/// ```
///
/// # Ignoring enum variants
///
/// To omit exposing a Rust enum variant in a GraphQL schema, use the `ignore`
/// attribute's argument directly on that variant. Only ignored Rust enum
/// variants are allowed to contain fields.
///
/// ```rust
/// # use juniper::GraphQLEnum;
/// #
/// #[derive(GraphQLEnum)]
/// enum Episode<T> {
///     NewHope,
///     Empire,
///     Jedi,
///     #[graphql(ignore)]
///     Legends(T),
///     #[graphql(skip)]
///     //        ^^^^ alternative naming, up to your preference
///     CloneWars(T),
/// }
/// ```
///
/// # Custom `ScalarValue`
///
/// By default, `#[derive(GraphQLEnum)]` macro generates code, which is generic
/// over a [`ScalarValue`] type. This can be changed with the `scalar`
/// attribute's argument.
///
/// ```rust
/// # use juniper::{DefaultScalarValue, GraphQLEnum};
/// #
/// #[derive(GraphQLEnum)]
/// #[graphql(scalar = DefaultScalarValue)]
/// enum Episode {
///     NewHope,
///     Empire,
///     Jedi,
/// }
/// ```
///
/// [`ScalarValue`]: juniper::ScalarValue
/// [0]: https://spec.graphql.org/October2021#sec-Enums
/// [1]: https://spec.graphql.org/October2021#sec-Enum-Value
#[proc_macro_derive(GraphQLEnum, attributes(graphql))]
pub fn derive_enum(input: TokenStream) -> TokenStream {
    diagnostic::entry_point(|| {
        graphql_enum::derive::expand(input.into())
            .unwrap_or_abort()
            .into()
    })
}

/// `#[derive(GraphQLScalar)]` macro for deriving a [GraphQL scalar][0]
/// implementation.
///
/// # Transparent delegation
///
/// Quite often we want to create a custom [GraphQL scalar][0] type by just
/// wrapping an existing one, inheriting all its behavior. In Rust, this is
/// often called as ["newtype pattern"][1]. This is achieved by annotating
/// the definition with the `#[graphql(transparent)]` attribute:
/// ```rust
/// # use juniper::{GraphQLObject, GraphQLScalar};
/// #
/// #[derive(GraphQLScalar)]
/// #[graphql(transparent)]
/// struct UserId(String);
///
/// #[derive(GraphQLScalar)]
/// #[graphql(transparent)]
/// struct DroidId {
///     value: String,
/// }
///
/// #[derive(GraphQLObject)]
/// struct Pair {
///   user_id: UserId,
///   droid_id: DroidId,
/// }
/// ```
///
/// The inherited behaviour may also be customized:
/// ```rust
/// # use juniper::GraphQLScalar;
/// #
/// /// Doc comments are used for the GraphQL type description.
/// #[derive(GraphQLScalar)]
/// #[graphql(
///     // Custom GraphQL name.
///     name = "MyUserId",
///     // Description can also specified in the attribute.
///     // This will the doc comment, if one exists.
///     description = "...",
///     // Optional specification URL.
///     specified_by_url = "https://tools.ietf.org/html/rfc4122",
///     // Explicit generic scalar.
///     scalar = S: juniper::ScalarValue,
///     transparent,
/// )]
/// struct UserId(String);
/// ```
///
/// All of the methods inherited from `Newtype`'s field may also be overridden
/// with the attributes described below.
///
/// # Custom resolving
///
/// Customization of a [GraphQL scalar][0] type resolving is possible via
/// `#[graphql(to_output_with = <fn path>)]` attribute:
/// ```rust
/// # use juniper::{GraphQLScalar, ScalarValue, Value};
/// #
/// #[derive(GraphQLScalar)]
/// #[graphql(to_output_with = to_output, transparent)]
/// struct Incremented(i32);
///
/// /// Increments [`Incremented`] before converting into a [`Value`].
/// fn to_output<S: ScalarValue>(v: &Incremented) -> Value<S> {
///     let inc = v.0 + 1;
///     Value::from(inc)
/// }
/// ```
///
/// # Custom parsing
///
/// Customization of a [GraphQL scalar][0] type parsing is possible via
/// `#[graphql(from_input_with = <fn path>)]` attribute:
/// ```rust
/// # use juniper::{DefaultScalarValue, GraphQLScalar, InputValue, ScalarValue};
/// #
/// #[derive(GraphQLScalar)]
/// #[graphql(from_input_with = Self::from_input, transparent)]
/// struct UserId(String);
///
/// impl UserId {
///     /// Checks whether [`InputValue`] is `String` beginning with `id: ` and
///     /// strips it.
///     fn from_input<S: ScalarValue>(
///         input: &InputValue<S>,
///     ) -> Result<Self, String> {
///         //            ^^^^^^ must implement `IntoFieldError`
///         input.as_string_value()
///             .ok_or_else(|| format!("Expected `String`, found: {input}"))
///             .and_then(|str| {
///                 str.strip_prefix("id: ")
///                     .ok_or_else(|| {
///                         format!(
///                             "Expected `UserId` to begin with `id: `, \
///                              found: {input}",
///                         )
///                     })
///             })
///             .map(|id| Self(id.into()))
///     }
/// }
/// ```
///
/// # Custom token parsing
///
/// Customization of which tokens a [GraphQL scalar][0] type should be parsed is
/// possible via `#[graphql(parse_token_with = <fn path>)]` or
/// `#[graphql(parse_token(<types>)]` attributes:
/// ```rust
/// # use juniper::{
/// #     GraphQLScalar, InputValue, ParseScalarResult, ParseScalarValue,
/// #     ScalarValue, ScalarToken, Value,
/// # };
/// #
/// #[derive(GraphQLScalar)]
/// #[graphql(
///     to_output_with = to_output,
///     from_input_with = from_input,
///     parse_token_with = parse_token,
/// )]
/// //  ^^^^^^^^^^^^^^^^ Can be replaced with `parse_token(String, i32)`, which
/// //                   tries to parse as `String` first, and then as `i32` if
/// //                   prior fails.
/// enum StringOrInt {
///     String(String),
///     Int(i32),
/// }
///
/// fn to_output<S: ScalarValue>(v: &StringOrInt) -> Value<S> {
///     match v {
///         StringOrInt::String(s) => Value::scalar(s.to_owned()),
///         StringOrInt::Int(i) => Value::scalar(*i),
///     }
/// }
///
/// fn from_input<S: ScalarValue>(v: &InputValue<S>) -> Result<StringOrInt, String> {
///     v.as_string_value()
///         .map(|s| StringOrInt::String(s.into()))
///         .or_else(|| v.as_int_value().map(StringOrInt::Int))
///         .ok_or_else(|| format!("Expected `String` or `Int`, found: {v}"))
/// }
///
/// fn parse_token<S: ScalarValue>(value: ScalarToken<'_>) -> ParseScalarResult<S> {
///     <String as ParseScalarValue<S>>::from_str(value)
///         .or_else(|_| <i32 as ParseScalarValue<S>>::from_str(value))
/// }
/// ```
/// > __NOTE:__ Once we provide all 3 custom functions, there is no sense in
/// >           following the [newtype pattern][1] anymore.
///
/// # Full behavior
///
/// Instead of providing all custom functions separately, it's possible to
/// provide a module holding the appropriate `to_output()`, `from_input()` and
/// `parse_token()` functions:
/// ```rust
/// # use juniper::{
/// #     GraphQLScalar, InputValue, ParseScalarResult, ParseScalarValue,
/// #     ScalarValue, ScalarToken, Value,
/// # };
/// #
/// #[derive(GraphQLScalar)]
/// #[graphql(with = string_or_int)]
/// enum StringOrInt {
///     String(String),
///     Int(i32),
/// }
///
/// mod string_or_int {
///     use super::*;
///
///     pub(super) fn to_output<S: ScalarValue>(v: &StringOrInt) -> Value<S> {
///         match v {
///             StringOrInt::String(s) => Value::scalar(s.to_owned()),
///             StringOrInt::Int(i) => Value::scalar(*i),
///         }
///     }
///
///     pub(super) fn from_input<S: ScalarValue>(v: &InputValue<S>) -> Result<StringOrInt, String> {
///         v.as_string_value()
///             .map(|s| StringOrInt::String(s.into()))
///             .or_else(|| v.as_int_value().map(StringOrInt::Int))
///             .ok_or_else(|| format!("Expected `String` or `Int`, found: {v}"))
///     }
///
///     pub(super) fn parse_token<S: ScalarValue>(t: ScalarToken<'_>) -> ParseScalarResult<S> {
///         <String as ParseScalarValue<S>>::from_str(t)
///             .or_else(|_| <i32 as ParseScalarValue<S>>::from_str(t))
///     }
/// }
/// #
/// # fn main() {}
/// ```
///
/// A regular `impl` block is also suitable for that:
/// ```rust
/// # use juniper::{
/// #     GraphQLScalar, InputValue, ParseScalarResult, ParseScalarValue,
/// #     ScalarValue, ScalarToken, Value,
/// # };
/// #
/// #[derive(GraphQLScalar)]
/// // #[graphql(with = Self)] <- default behaviour, so can be omitted
/// enum StringOrInt {
///     String(String),
///     Int(i32),
/// }
///
/// impl StringOrInt {
///     fn to_output<S: ScalarValue>(&self) -> Value<S> {
///         match self {
///             Self::String(s) => Value::scalar(s.to_owned()),
///             Self::Int(i) => Value::scalar(*i),
///         }
///     }
///
///     fn from_input<S>(v: &InputValue<S>) -> Result<Self, String>
///     where
///         S: ScalarValue
///     {
///         v.as_string_value()
///             .map(|s| Self::String(s.into()))
///             .or_else(|| v.as_int_value().map(Self::Int))
///             .ok_or_else(|| format!("Expected `String` or `Int`, found: {v}"))
///     }
///
///     fn parse_token<S>(value: ScalarToken<'_>) -> ParseScalarResult<S>
///     where
///         S: ScalarValue
///     {
///         <String as ParseScalarValue<S>>::from_str(value)
///             .or_else(|_| <i32 as ParseScalarValue<S>>::from_str(value))
///     }
/// }
/// #
/// # fn main() {}
/// ```
///
/// At the same time, any custom function still may be specified separately:
/// ```rust
/// # use juniper::{
/// #     GraphQLScalar, InputValue, ParseScalarResult, ScalarValue,
/// #     ScalarToken, Value
/// # };
/// #
/// #[derive(GraphQLScalar)]
/// #[graphql(
///     with = string_or_int,
///     parse_token(String, i32)
/// )]
/// enum StringOrInt {
///     String(String),
///     Int(i32),
/// }
///
/// mod string_or_int {
///     use super::*;
///
///     pub(super) fn to_output<S>(v: &StringOrInt) -> Value<S>
///     where
///         S: ScalarValue,
///     {
///         match v {
///             StringOrInt::String(s) => Value::scalar(s.to_owned()),
///             StringOrInt::Int(i) => Value::scalar(*i),
///         }
///     }
///
///     pub(super) fn from_input<S>(v: &InputValue<S>) -> Result<StringOrInt, String>
///     where
///         S: ScalarValue,
///     {
///         v.as_string_value()
///             .map(|s| StringOrInt::String(s.into()))
///             .or_else(|| v.as_int_value().map(StringOrInt::Int))
///             .ok_or_else(|| format!("Expected `String` or `Int`, found: {v}"))
///     }
///
///     // No need in `parse_token()` function.
/// }
/// #
/// # fn main() {}
/// ```
///
/// # Custom `ScalarValue`
///
/// By default, this macro generates code, which is generic over a
/// [`ScalarValue`] type. Concrete [`ScalarValue`] type may be specified via
/// `#[graphql(scalar = <type>)]` attribute.
///
/// It also may be used to provide additional bounds to the [`ScalarValue`]
/// generic, like the following: `#[graphql(scalar = S: Trait)]`.
///
/// # Additional arbitrary trait bounds
///
/// [GraphQL scalar][0] type implementation may be bound with any additional
/// trait bounds via `#[graphql(where(<bounds>))]` attribute, like the
/// following: `#[graphql(where(S: Trait, Self: fmt::Debug + fmt::Display))]`.
///
/// [0]: https://spec.graphql.org/October2021#sec-Scalars
/// [1]: https://rust-unofficial.github.io/patterns/patterns/behavioural/newtype.html
/// [`ScalarValue`]: juniper::ScalarValue
#[proc_macro_derive(GraphQLScalar, attributes(graphql))]
pub fn derive_scalar(input: TokenStream) -> TokenStream {
    diagnostic::entry_point(|| {
        graphql_scalar::derive::expand(input.into())
            .unwrap_or_abort()
            .into()
    })
}

/// `#[graphql_scalar]` macro.is interchangeable with
/// `#[derive(`[`GraphQLScalar`]`)]` macro, and is used for deriving a
/// [GraphQL scalar][0] implementation.
///
/// ```rust
/// # use juniper::graphql_scalar;
/// #
/// /// Doc comments are used for the GraphQL type description.
/// #[graphql_scalar]
/// #[graphql(
///     // Custom GraphQL name.
///     name = "MyUserId",
///     // Description can also specified in the attribute.
///     // This will the doc comment, if one exists.
///     description = "...",
///     // Optional specification URL.
///     specified_by_url = "https://tools.ietf.org/html/rfc4122",
///     // Explicit generic scalar.
///     scalar = S: juniper::ScalarValue,
///     transparent,
/// )]
/// struct UserId(String);
/// ```
///
/// # Foreign types
///
/// Additionally, `#[graphql_scalar]` can be used directly on foreign types via
/// type alias, without using the [newtype pattern][1].
///
/// > __NOTE:__ To satisfy [orphan rules] you should provide local
/// >           [`ScalarValue`] implementation.
///
/// ```rust
/// # mod date {
/// #    use std::{fmt, str::FromStr};
/// #
/// #    pub struct Date;
/// #
/// #    impl FromStr for Date {
/// #        type Err = String;
/// #
/// #        fn from_str(_: &str) -> Result<Self, Self::Err> {
/// #            unimplemented!()
/// #        }
/// #    }
/// #
/// #    impl fmt::Display for Date {
/// #        fn fmt(&self, _: &mut fmt::Formatter<'_>) -> fmt::Result {
/// #            unimplemented!()
/// #        }
/// #    }
/// # }
/// #
/// # use juniper::DefaultScalarValue as CustomScalarValue;
/// use juniper::{graphql_scalar, InputValue, ScalarValue, Value};
///
/// #[graphql_scalar]
/// #[graphql(
///     with = date_scalar,
///     parse_token(String),
///     scalar = CustomScalarValue,
/// )]
/// //           ^^^^^^^^^^^^^^^^^ local `ScalarValue` implementation
/// type Date = date::Date;
/// //          ^^^^^^^^^^ type from another crate
///
/// mod date_scalar {
///     use super::*;
///
///     pub(super) fn to_output(v: &Date) -> Value<CustomScalarValue> {
///         Value::scalar(v.to_string())
///     }
///
///     pub(super) fn from_input(v: &InputValue<CustomScalarValue>) -> Result<Date, String> {
///       v.as_string_value()
///           .ok_or_else(|| format!("Expected `String`, found: {v}"))
///           .and_then(|s| s.parse().map_err(|e| format!("Failed to parse `Date`: {e}")))
///     }
/// }
/// #
/// # fn main() {}
/// ```
///
/// [0]: https://spec.graphql.org/October2021#sec-Scalars
/// [1]: https://rust-unofficial.github.io/patterns/patterns/behavioural/newtype.html
/// [orphan rules]: https://bit.ly/3glAGC2
/// [`GraphQLScalar`]: juniper::GraphQLScalar
/// [`ScalarValue`]: juniper::ScalarValue
#[proc_macro_attribute]
pub fn graphql_scalar(attr: TokenStream, body: TokenStream) -> TokenStream {
    diagnostic::entry_point_with_preserved_body(body.clone(), || {
        graphql_scalar::attr::expand(attr.into(), body.into())
            .unwrap_or_abort()
            .into()
    })
}

/// `#[derive(ScalarValue)]` macro for deriving a [`ScalarValue`]
/// implementation.
///
/// To derive a [`ScalarValue`] on enum you should mark the corresponding enum
/// variants with `as_int`, `as_float`, `as_string`, `into_string`, `as_str` and
/// `as_bool` attribute argumentes (names correspond to [`ScalarValue`] required
/// methods).
///
/// ```rust
/// # use std::fmt;
/// #
/// # use serde::{de, Deserialize, Deserializer, Serialize};
/// # use juniper::ScalarValue;
/// #
/// #[derive(Clone, Debug, PartialEq, ScalarValue, Serialize)]
/// #[serde(untagged)]
/// enum MyScalarValue {
///     #[value(as_float, as_int)]
///     Int(i32),
///     Long(i64),
///     #[value(as_float)]
///     Float(f64),
///     #[value(
///         into_string,
///         as_str,
///         as_string = String::clone,
///     )]
///     //              ^^^^^^^^^^^^^ custom resolvers may be provided
///     String(String),
///     #[value(as_bool)]
///     Boolean(bool),
/// }
///
/// impl<'de> Deserialize<'de> for MyScalarValue {
///     fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
///         struct Visitor;
///
///         impl<'de> de::Visitor<'de> for Visitor {
///             type Value = MyScalarValue;
///
///             fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
///                 f.write_str("a valid input value")
///             }
///
///             fn visit_bool<E: de::Error>(self, b: bool) -> Result<Self::Value, E> {
///                 Ok(MyScalarValue::Boolean(b))
///             }
///
///             fn visit_i32<E: de::Error>(self, n: i32) -> Result<Self::Value, E> {
///                 Ok(MyScalarValue::Int(n))
///             }
///
///             fn visit_i64<E: de::Error>(self, n: i64) -> Result<Self::Value, E> {
///                 if n <= i64::from(i32::MAX) {
///                     self.visit_i32(n.try_into().unwrap())
///                 } else {
///                     Ok(MyScalarValue::Long(n))
///                 }
///             }
///
///             fn visit_u32<E: de::Error>(self, n: u32) -> Result<Self::Value, E> {
///                 if n <= i32::MAX as u32 {
///                     self.visit_i32(n.try_into().unwrap())
///                 } else {
///                     self.visit_u64(n.into())
///                 }
///             }
///
///             fn visit_u64<E: de::Error>(self, n: u64) -> Result<Self::Value, E> {
///                 if n <= i64::MAX as u64 {
///                     self.visit_i64(n.try_into().unwrap())
///                 } else {
///                     // Browser's `JSON.stringify()` serialize all numbers
///                     // having no fractional part as integers (no decimal
///                     // point), so we must parse large integers as floating
///                     // point, otherwise we would error on transferring large
///                     // floating point numbers.
///                     Ok(MyScalarValue::Float(n as f64))
///                 }
///             }
///
///             fn visit_f64<E: de::Error>(self, f: f64) -> Result<Self::Value, E> {
///                 Ok(MyScalarValue::Float(f))
///             }
///
///             fn visit_str<E: de::Error>(self, s: &str) -> Result<Self::Value, E> {
///                 self.visit_string(s.into())
///             }
///
///             fn visit_string<E: de::Error>(self, s: String) -> Result<Self::Value, E> {
///                 Ok(MyScalarValue::String(s))
///             }
///         }
///
///         de.deserialize_any(Visitor)
///     }
/// }
/// ```
///
/// [`ScalarValue`]: juniper::ScalarValue
#[proc_macro_derive(ScalarValue, attributes(value))]
pub fn derive_scalar_value(input: TokenStream) -> TokenStream {
    diagnostic::entry_point(|| {
        scalar_value::expand_derive(input.into())
            .unwrap_or_abort()
            .into()
    })
}

/// `#[graphql_interface]` macro for generating a [GraphQL interface][1]
/// implementation for traits and its implementers.
///
/// Specifying multiple `#[graphql_interface]` attributes on the same definition
/// is totally okay. They all will be treated as a single attribute.
///
/// [GraphQL interfaces][1] are more like structurally-typed interfaces, while
/// Rust's traits are more like type classes. Using `impl Trait` isn't an
/// option, so you have to cover all trait's methods with type's fields or
/// impl block.
///
/// Another difference between [GraphQL interface][1] type and Rust trait is
/// that the former serves both as an _abstraction_ and a _value downcastable to
/// concrete implementers_, while in Rust, a trait is an _abstraction only_ and
/// you need a separate type to downcast into a concrete implementer, like enum
/// or [trait object][3], because trait doesn't represent a type itself.
/// Macro uses Rust enums only to represent a value type of a
/// [GraphQL interface][1].
///
/// [GraphQL interface][1] can be represented with struct in case methods don't
/// have any arguments:
///
/// ```rust
/// use juniper::{graphql_interface, GraphQLObject};
///
/// // NOTICE: By default a `CharacterValue` enum is generated by macro to represent values of this
/// //         GraphQL interface.
/// #[graphql_interface]
/// #[graphql(for = Human)] // enumerating all implementers is mandatory
/// struct Character {
///     id: String,
/// }
///
/// #[derive(GraphQLObject)]
/// #[graphql(impl = CharacterValue)] // notice the enum type name, not trait name
/// struct Human {
///     id: String, // this field is used to resolve Character::id
///     home_planet: String,
/// }
/// ```
///
/// Also [GraphQL interface][1] can be represented with trait:
///
/// ```rust
/// use juniper::{graphql_interface, GraphQLObject};
///
/// // NOTICE: By default a `CharacterValue` enum is generated by macro to represent values of this
/// //         GraphQL interface.
/// #[graphql_interface]
/// #[graphql(for = Human)] // enumerating all implementers is mandatory
/// trait Character {
///     fn id(&self) -> &str;
/// }
///
/// #[derive(GraphQLObject)]
/// #[graphql(impl = CharacterValue)] // notice the enum type name, not trait name
/// struct Human {
///     id: String, // this field is used to resolve Character::id
///     home_planet: String,
/// }
/// ```
///
/// > __NOTE:__ Struct or trait representing interface acts only as a blueprint
/// >           for names of methods, their arguments and return type, so isn't
/// >           actually used at a runtime. But no-one is stopping you from
/// >           implementing trait manually for your own usage.
///
/// # Custom name, description, deprecation and argument defaults
///
/// The name of [GraphQL interface][1], its field, or a field argument may be overridden with a
/// `name` attribute's argument. By default, a type name is used or `camelCased` method/argument
/// name.
///
/// The description of [GraphQL interface][1], its field, or a field argument may be specified
/// either with a `description`/`desc` attribute's argument, or with a regular Rust doc comment.
///
/// A field of [GraphQL interface][1] may be deprecated by specifying a `deprecated` attribute's
/// argument, or with regular Rust `#[deprecated]` attribute.
///
/// The default value of a field argument may be specified with a `default` attribute argument (if
/// no exact value is specified then [`Default::default`] is used).
///
/// ```rust
/// # use juniper::graphql_interface;
/// #
/// #[graphql_interface]
/// #[graphql(name = "Character", desc = "Possible episode characters.")]
/// trait Chrctr {
///     #[graphql(name = "id", desc = "ID of the character.")]
///     #[graphql(deprecated = "Don't use it")]
///     fn some_id(
///         &self,
///         #[graphql(name = "number", desc = "Arbitrary number.")]
///         #[graphql(default = 5)]
///         num: i32,
///     ) -> &str;
/// }
///
/// // NOTICE: Rust docs are used as GraphQL description.
/// /// Possible episode characters.
/// #[graphql_interface]
/// trait CharacterWithDocs {
///     /// ID of the character.
///     #[deprecated]
///     fn id(&self, #[graphql(default)] num: i32) -> &str;
/// }
/// ```
///
/// # Interfaces implementing other interfaces
///
/// GraphQL allows implementing interfaces on other interfaces in addition to
/// objects.
///
/// > __NOTE:__ Every interface has to specify all other interfaces/objects it
/// >           implements or is implemented for. Missing one of `for = ` or
/// >           `impl = ` attributes is an understandable compile-time error.
///
/// ```rust
/// # extern crate juniper;
/// use juniper::{graphql_interface, graphql_object, ID};
///
/// #[graphql_interface]
/// #[graphql(for = [HumanValue, Luke])]
/// struct Node {
///     id: ID,
/// }
///
/// #[graphql_interface]
/// #[graphql(impl = NodeValue, for = Luke)]
/// struct Human {
///     id: ID,
///     home_planet: String,
/// }
///
/// struct Luke {
///     id: ID,
/// }
///
/// #[graphql_object]
/// #[graphql(impl = [HumanValue, NodeValue])]
/// impl Luke {
///     fn id(&self) -> &ID {
///         &self.id
///     }
///
///     // As `String` and `&str` aren't distinguished by
///     // GraphQL spec, you can use them interchangeably.
///     // Same is applied for `Cow<'a, str>`.
///     //                  ⌄⌄⌄⌄⌄⌄⌄⌄⌄⌄⌄⌄
///     fn home_planet() -> &'static str {
///         "Tatooine"
///     }
/// }
/// ```
///
/// # GraphQL subtyping and additional `null`able fields
///
/// GraphQL allows implementers (both objects and other interfaces) to return
/// "subtypes" instead of an original value. Basically, this allows you to
/// impose additional bounds on the implementation.
///
/// Valid "subtypes" are:
/// - interface implementer instead of an interface itself:
///   - `I implements T` in place of a `T`;
///   - `Vec<I implements T>` in place of a `Vec<T>`.
/// - non-`null` value in place of a `null`able:
///   - `T` in place of a `Option<T>`;
///   - `Vec<T>` in place of a `Vec<Option<T>>`.
///
/// These rules are recursively applied, so `Vec<Vec<I implements T>>` is a
/// valid "subtype" of a `Option<Vec<Option<Vec<Option<T>>>>>`.
///
/// Also, GraphQL allows implementers to add `null`able fields, which aren't
/// present on an original interface.
///
/// ```rust
/// # extern crate juniper;
/// use juniper::{graphql_interface, graphql_object, ID};
///
/// #[graphql_interface]
/// #[graphql(for = [HumanValue, Luke])]
/// struct Node {
///     id: ID,
/// }
///
/// #[graphql_interface]
/// #[graphql(for = HumanConnectionValue)]
/// struct Connection {
///     nodes: Vec<NodeValue>,
/// }
///
/// #[graphql_interface]
/// #[graphql(impl = NodeValue, for = Luke)]
/// struct Human {
///     id: ID,
///     home_planet: String,
/// }
///
/// #[graphql_interface]
/// #[graphql(impl = ConnectionValue)]
/// struct HumanConnection {
///     nodes: Vec<HumanValue>,
///     //         ^^^^^^^^^^ notice not `NodeValue`
///     // This can happen, because every `Human` is a `Node` too, so we are
///     // just imposing additional bounds, which still can be resolved with
///     // `... on Connection { nodes }`.
/// }
///
/// struct Luke {
///     id: ID,
/// }
///
/// #[graphql_object]
/// #[graphql(impl = [HumanValue, NodeValue])]
/// impl Luke {
///     fn id(&self) -> &ID {
///         &self.id
///     }
///
///     fn home_planet(language: Option<String>) -> &'static str {
///         //                   ^^^^^^^^^^^^^^
///         // Notice additional `null`able field, which is missing on `Human`.
///         // Resolving `...on Human { homePlanet }` will provide `None` for
///         // this argument.
///         match language.as_deref() {
///             None | Some("en") => "Tatooine",
///             Some("ko") => "타투인",
///             _ => todo!(),
///         }
///     }
/// }
/// #
/// # fn main() {}
/// ```
///
/// # Renaming policy
///
/// By default, all [GraphQL interface][1] fields and their arguments are renamed
/// via `camelCase` policy (so `fn my_id(&self) -> String` becomes `myId` field
/// in GraphQL schema, and so on). This complies with default GraphQL naming
/// conventions [demonstrated in spec][0].
///
/// However, if you need for some reason apply another naming convention, it's
/// possible to do by using `rename_all` attribute's argument. At the moment it
/// supports the following policies only: `SCREAMING_SNAKE_CASE`, `camelCase`,
/// `none` (disables any renaming).
///
/// ```rust
/// # use juniper::{graphql_interface, graphql_object};
/// #
/// #[graphql_interface]
/// #[graphql(for = Human, rename_all = "none")] // disables renaming
/// trait Character {
///     // NOTICE: In the generated GraphQL schema this field and its argument
///     //         will be `detailed_info` and `info_kind`.
///     fn detailed_info(&self, info_kind: String) -> String;
/// }
///
/// struct Human {
///     id: String,
///     home_planet: String,
/// }
///
/// #[graphql_object]
/// #[graphql(impl = CharacterValue, rename_all = "none")]
/// impl Human {
///     fn id(&self) -> &str {
///         &self.id
///     }
///
///     fn home_planet(&self) -> &str {
///         &self.home_planet
///     }
///
///     // You can return `&str` even if trait definition returns `String`.
///     fn detailed_info(&self, info_kind: String) -> &str {
///         (info_kind == "planet")
///             .then_some(&self.home_planet)
///             .unwrap_or(&self.id)
///     }
/// }
/// ```
///
/// # Ignoring trait methods
///
/// To omit some trait method to be assumed as a [GraphQL interface][1] field
/// and ignore it, use an `ignore` attribute's argument directly on that method.
///
/// ```rust
/// # use juniper::graphql_interface;
/// #
/// #[graphql_interface]
/// trait Character {
///     fn id(&self) -> &str;
///
///     #[graphql(ignore)]
///     fn kaboom(&mut self);
/// }
/// ```
///
/// # Custom context
///
/// By default, the generated implementation tries to infer [`Context`] type from signatures of
/// trait methods, and uses [unit type `()`][4] if signatures contains no [`Context`] arguments.
///
/// If [`Context`] type cannot be inferred or is inferred incorrectly, then specify it explicitly
/// with `context` attribute's argument.
///
/// If trait method represents a [GraphQL interface][1] field and its argument is named as `context`
/// or `ctx` then this argument is assumed as [`Context`] and will be omitted in GraphQL schema.
/// Additionally, any argument may be marked as [`Context`] with a `context` attribute's argument.
///
/// ```rust
/// # use std::collections::HashMap;
/// # use juniper::{graphql_interface, graphql_object};
/// #
/// struct Database {
///     humans: HashMap<String, Human>,
///     droids: HashMap<String, Droid>,
/// }
/// impl juniper::Context for Database {}
///
/// #[graphql_interface]
/// #[graphql(for = [Human, Droid], Context = Database)]
/// trait Character {
///     fn id<'db>(&self, ctx: &'db Database) -> Option<&'db str>;
///     fn info<'db>(&self, #[graphql(context)] db: &'db Database) -> Option<&'db str>;
/// }
///
/// struct Human {
///     id: String,
///     home_planet: String,
/// }
/// #[graphql_object]
/// #[graphql(impl = CharacterValue, Context = Database)]
/// impl Human {
///     fn id<'db>(&self, context: &'db Database) -> Option<&'db str> {
///         context.humans.get(&self.id).map(|h| h.id.as_str())
///     }
///     fn info<'db>(&self, #[graphql(context)] db: &'db Database) -> Option<&'db str> {
///         db.humans.get(&self.id).map(|h| h.home_planet.as_str())
///     }
///     fn home_planet(&self) -> &str {
///         &self.home_planet
///     }
/// }
///
/// struct Droid {
///     id: String,
///     primary_function: String,
/// }
/// #[graphql_object]
/// #[graphql(impl = CharacterValue, Context = Database)]
/// impl Droid {
///     fn id<'db>(&self, ctx: &'db Database) -> Option<&'db str> {
///         ctx.droids.get(&self.id).map(|h| h.id.as_str())
///     }
///     fn info<'db>(&self, #[graphql(context)] db: &'db Database) -> Option<&'db str> {
///         db.droids.get(&self.id).map(|h| h.primary_function.as_str())
///     }
///     fn primary_function(&self) -> &str {
///         &self.primary_function
///     }
/// }
/// ```
///
/// # Using `Executor`
///
/// If an [`Executor`] is required in a trait method to resolve a [GraphQL interface][1] field,
/// specify it as an argument named as `executor` or explicitly marked with an `executor`
/// attribute's argument. Such method argument will be omitted in GraphQL schema.
///
/// However, this requires to explicitly parametrize over [`ScalarValue`], as [`Executor`] does so.
///
/// ```rust
/// # use juniper::{graphql_interface, graphql_object, Executor, ScalarValue};
/// #
/// #[graphql_interface]
/// // NOTICE: Specifying `ScalarValue` as existing type parameter.
/// #[graphql(for = Human, scalar = S)]
/// trait Character<S: ScalarValue> {
///     fn id<'a>(&self, executor: &'a Executor<'_, '_, (), S>) -> &'a str;
///
///     fn name<'b>(
///         &'b self,
///         #[graphql(executor)] another: &Executor<'_, '_, (), S>,
///     ) -> &'b str;
/// }
///
/// struct Human {
///     id: String,
///     name: String,
/// }
/// #[graphql_object]
/// #[graphql(scalar = S: ScalarValue, impl = CharacterValue<S>)]
/// impl Human {
///     async fn id<'a, S>(&self, executor: &'a Executor<'_, '_, (), S>) -> &'a str
///     where
///         S: ScalarValue,
///     {
///         executor.look_ahead().field_name()
///     }
///
///     async fn name<'b, S>(&'b self, _executor: &Executor<'_, '_, (), S>) -> &'b str {
///         &self.name
///     }
/// }
/// ```
///
/// # Custom `ScalarValue`
///
/// By default, `#[graphql_interface]` macro generates code, which is generic
/// over a [`ScalarValue`] type. This may introduce a problem when at least one
/// of [GraphQL interface][1] implementers is restricted to a concrete
/// [`ScalarValue`] type in its implementation. To resolve such problem, a
/// concrete [`ScalarValue`] type should be specified with a `scalar`
/// attribute's argument.
///
/// ```rust
/// # use juniper::{graphql_interface, DefaultScalarValue, GraphQLObject};
/// #
/// #[graphql_interface]
/// // NOTICE: Removing `Scalar` argument will fail compilation.
/// #[graphql(for = Human, scalar = DefaultScalarValue)]
/// trait Character {
///     fn id(&self) -> &str;
/// }
///
/// #[derive(GraphQLObject)]
/// #[graphql(impl = CharacterValue, scalar = DefaultScalarValue)]
/// struct Human {
///     id: String,
///     home_planet: String,
/// }
/// ```
///
/// [`Context`]: juniper::Context
/// [`Executor`]: juniper::Executor
/// [`ScalarValue`]: juniper::ScalarValue
/// [0]: https://spec.graphql.org/October2021
/// [1]: https://spec.graphql.org/October2021#sec-Interfaces
/// [2]: https://doc.rust-lang.org/stable/reference/items/traits.html#object-safety
/// [3]: https://doc.rust-lang.org/stable/reference/types/trait-object.html
/// [4]: https://doc.rust-lang.org/stable/std/primitive.unit.html
#[proc_macro_attribute]
pub fn graphql_interface(attr: TokenStream, body: TokenStream) -> TokenStream {
    diagnostic::entry_point_with_preserved_body(body.clone(), || {
        self::graphql_interface::attr::expand(attr.into(), body.into())
            .unwrap_or_abort()
            .into()
    })
}

/// `#[derive(GraphQLInterface)]` macro for generating a [GraphQL interface][1]
/// implementation for traits and its implementers.
///
/// This macro is applicable only to structs and useful in case [interface][1]
/// fields don't have any arguments:
///
/// ```rust
/// use juniper::{GraphQLInterface, GraphQLObject};
///
/// // NOTICE: By default a `CharacterValue` enum is generated by macro to represent values of this
/// //         GraphQL interface.
/// #[derive(GraphQLInterface)]
/// #[graphql(for = Human)] // enumerating all implementers is mandatory
/// struct Character {
///     id: String,
/// }
///
/// #[derive(GraphQLObject)]
/// #[graphql(impl = CharacterValue)] // notice the enum type name, not trait name
/// struct Human {
///     id: String, // this field is used to resolve Character::id
///     home_planet: String,
/// }
/// ```
///
/// For more info and possibilities see [`#[graphql_interface]`][0] macro.
///
/// [0]: crate::graphql_interface
/// [1]: https://spec.graphql.org/October2021#sec-Interfaces
#[proc_macro_derive(GraphQLInterface, attributes(graphql))]
pub fn derive_interface(body: TokenStream) -> TokenStream {
    diagnostic::entry_point(|| {
        self::graphql_interface::derive::expand(body.into())
            .unwrap_or_abort()
            .into()
    })
}

/// `#[derive(GraphQLObject)]` macro for deriving a [GraphQL object][1]
/// implementation for structs.
///
/// The `#[graphql]` helper attribute is used for configuring the derived
/// implementation. Specifying multiple `#[graphql]` attributes on the same
/// definition is totally okay. They all will be treated as a single attribute.
///
/// ```
/// use juniper::GraphQLObject;
///
/// #[derive(GraphQLObject)]
/// struct Query {
///     // NOTICE: By default, field names will be converted to `camelCase`.
///     //         In the generated GraphQL schema this field will be available
///     //         as `apiVersion`.
///     api_version: &'static str,
/// }
/// ```
///
/// # Custom name, description and deprecation
///
/// The name of [GraphQL object][1] or its field may be overridden with a `name`
/// attribute's argument. By default, a type name is used or `camelCased` field
/// name.
///
/// The description of [GraphQL object][1] or its field may be specified either
/// with a `description`/`desc` attribute's argument, or with a regular Rust doc
/// comment.
///
/// A field of [GraphQL object][1] may be deprecated by specifying a
/// `deprecated` attribute's argument, or with regular Rust `#[deprecated]`
/// attribute.
///
/// ```
/// # use juniper::GraphQLObject;
/// #
/// #[derive(GraphQLObject)]
/// #[graphql(
///     // Rename the type for GraphQL by specifying the name here.
///     name = "Human",
///     // You may also specify a description here.
///     // If present, doc comments will be ignored.
///     desc = "Possible episode human.",
/// )]
/// struct HumanWithAttrs {
///     #[graphql(name = "id", desc = "ID of the human.")]
///     #[graphql(deprecated = "Don't use it")]
///     some_id: String,
/// }
///
/// // Rust docs are used as GraphQL description.
/// /// Possible episode human.
/// #[derive(GraphQLObject)]
/// struct HumanWithDocs {
///     // Doc comments also work on fields.
///     /// ID of the human.
///     #[deprecated]
///     id: String,
/// }
/// ```
///
/// # Renaming policy
///
/// By default, all [GraphQL object][1] fields are renamed via `camelCase`
/// policy (so `api_version: String` becomes `apiVersion` field in GraphQL
/// schema, and so on). This complies with default GraphQL naming conventions
/// [demonstrated in spec][0].
///
/// However, if you need for some reason apply another naming convention, it's
/// possible to do by using `rename_all` attribute's argument. At the moment it
/// supports the following policies only: `SCREAMING_SNAKE_CASE`, `camelCase`,
/// `none` (disables any renaming).
///
/// ```
/// # use juniper::GraphQLObject;
/// #
/// #[derive(GraphQLObject)]
/// #[graphql(rename_all = "none")] // disables renaming
/// struct Query {
///     // NOTICE: In the generated GraphQL schema this field will be available
///     //         as `api_version`.
///     api_version: String,
/// }
/// ```
///
/// # Ignoring struct fields
///
/// To omit exposing a struct field in the GraphQL schema, use an `ignore`
/// attribute's argument directly on that field.
///
/// ```
/// # use juniper::GraphQLObject;
/// #
/// #[derive(GraphQLObject)]
/// struct Human {
///     id: String,
///     #[graphql(ignore)]
///     home_planet: String,
///     #[graphql(skip)]
///     //        ^^^^ alternative naming, up to your preference
///     password_hash: String,
/// }
/// ```
///
/// # Custom `ScalarValue`
///
/// By default, `#[derive(GraphQLObject)]` macro generates code, which is
/// generic over a [`ScalarValue`] type. This may introduce a problem when at
/// least one of its fields is restricted to a concrete [`ScalarValue`] type in
/// its implementation. To resolve such problem, a concrete [`ScalarValue`] type
/// should be specified with a `scalar` attribute's argument.
///
/// ```
/// # use juniper::{DefaultScalarValue, GraphQLObject};
/// #
/// #[derive(GraphQLObject)]
/// // NOTICE: Removing `scalar` argument will fail compilation.
/// #[graphql(scalar = DefaultScalarValue)]
/// struct Human {
///     id: String,
///     helper: Droid,
/// }
///
/// #[derive(GraphQLObject)]
/// #[graphql(scalar = DefaultScalarValue)]
/// struct Droid {
///     id: String,
/// }
/// ```
///
/// [`ScalarValue`]: juniper::ScalarValue
/// [1]: https://spec.graphql.org/October2021#sec-Objects
#[proc_macro_derive(GraphQLObject, attributes(graphql))]
pub fn derive_object(body: TokenStream) -> TokenStream {
    diagnostic::entry_point(|| {
        self::graphql_object::derive::expand(body.into())
            .unwrap_or_abort()
            .into()
    })
}

/// `#[graphql_object]` macro for generating a [GraphQL object][1]
/// implementation for structs with computable field resolvers (declared via
/// a regular Rust `impl` block).
///
/// It enables you to write GraphQL field resolvers for a type by declaring a
/// regular Rust `impl` block. Under the hood, the macro implements
/// [`GraphQLType`]/[`GraphQLValue`] traits.
///
/// Specifying multiple `#[graphql_object]` attributes on the same definition
/// is totally okay. They all will be treated as a single attribute.
///
/// ```
/// use juniper::graphql_object;
///
/// // We can declare the type as a plain struct without any members.
/// struct Query;
///
/// #[graphql_object]
/// impl Query {
///     // WARNING: Only GraphQL fields can be specified in this `impl` block.
///     //          If normal methods are required on the struct, they can be
///     //          defined either in a separate "normal" `impl` block, or
///     //          marked with `#[graphql(ignore)]` attribute.
///
///     // This defines a simple, static field which does not require any
///     // context.
///     // Such field can return any value that implements `GraphQLType` and
///     // `GraphQLValue` traits.
///     //
///     // NOTICE: By default, field names will be converted to `camelCase`.
///     //         In the generated GraphQL schema this field will be available
///     //         as `apiVersion`.
///     fn api_version() -> &'static str {
///         "0.1"
///     }
///
///     // This field takes two arguments.
///     // GraphQL arguments are just regular function parameters.
///     //
///     // NOTICE: In `juniper`, arguments are non-nullable by default. For
///     //         optional arguments, you have to specify them as `Option<_>`.
///     async fn add(a: f64, b: f64, c: Option<f64>) -> f64 {
///         a + b + c.unwrap_or(0.0)
///     }
/// }
/// ```
///
/// # Accessing self
///
/// Fields may also have a `self` receiver.
///
/// ```
/// # use juniper::graphql_object;
/// #
/// struct Person {
///     first_name: String,
///     last_name: String,
/// }
///
/// #[graphql_object]
/// impl Person {
///     fn first_name(&self) -> &str {
///         &self.first_name
///     }
///
///     fn last_name(&self) -> &str {
///         &self.last_name
///     }
///
///     fn full_name(&self) -> String {
///         self.build_full_name()
///     }
///
///     // This method is useful only to define GraphQL fields, but is not
///     // a field itself, so we ignore it in schema.
///     #[graphql(ignore)] // or `#[graphql(skip)]`, up to your preference
///     fn build_full_name(&self) -> String {
///         format!("{} {}", self.first_name, self.last_name)
///     }
/// }
/// ```
///
/// # Custom name, description, deprecation and argument defaults
///
/// The name of [GraphQL object][1], its field, or a field argument may be
/// overridden with a `name` attribute's argument. By default, a type name is
/// used or `camelCased` method/argument name.
///
/// The description of [GraphQL object][1], its field, or a field argument may
/// be specified either with a `description`/`desc` attribute's argument, or
/// with a regular Rust doc comment.
///
/// A field of [GraphQL object][1] may be deprecated by specifying a
/// `deprecated` attribute's argument, or with regular Rust `#[deprecated]`
/// attribute.
///
/// The default value of a field argument may be specified with a `default`
/// attribute argument (if no exact value is specified then [`Default::default`]
/// is used).
///
/// ```
/// # use juniper::graphql_object;
/// #
/// struct HumanWithAttrs;
///
/// #[graphql_object]
/// #[graphql(
///     // Rename the type for GraphQL by specifying the name here.
///     name = "Human",
///     // You may also specify a description here.
///     // If present, doc comments will be ignored.
///     desc = "Possible episode human.",
/// )]
/// impl HumanWithAttrs {
///     #[graphql(name = "id", desc = "ID of the human.")]
///     #[graphql(deprecated = "Don't use it")]
///     fn some_id(
///         &self,
///         #[graphql(name = "number", desc = "Arbitrary number.")]
///         // You may specify default values.
///         // A default can be any valid expression that yields the right type.
///         #[graphql(default = 5)]
///         num: i32,
///     ) -> &str {
///         "Don't use me!"
///     }
/// }
///
/// struct HumanWithDocs;
///
/// // Rust docs are used as GraphQL description.
/// /// Possible episode human.
/// #[graphql_object]
/// impl HumanWithDocs {
///     // Doc comments also work on fields.
///     /// ID of the human.
///     #[deprecated]
///     fn id(
///         &self,
///         // If expression is not specified then `Default::default()` is used.
///         #[graphql(default)] num: i32,
///     ) -> &str {
///         "Deprecated"
///     }
/// }
/// ```
///
/// # Renaming policy
///
/// By default, all [GraphQL object][1] fields and their arguments are renamed
/// via `camelCase` policy (so `fn api_version() -> String` becomes `apiVersion`
/// field in GraphQL schema, and so on). This complies with default GraphQL
/// naming conventions [demonstrated in spec][0].
///
/// However, if you need for some reason apply another naming convention, it's
/// possible to do by using `rename_all` attribute's argument. At the moment it
/// supports the following policies only: `SCREAMING_SNAKE_CASE`, `camelCase`,
/// `none` (disables any renaming).
///
/// ```
/// # use juniper::graphql_object;
/// #
/// struct Query;
///
/// #[graphql_object]
/// #[graphql(rename_all = "none")] // disables renaming
/// impl Query {
///     // NOTICE: In the generated GraphQL schema this field will be available
///     //         as `api_version`.
///     fn api_version() -> &'static str {
///         "0.1"
///     }
///
///     // NOTICE: In the generated GraphQL schema these field arguments will be
///     //         available as `arg_a` and `arg_b`.
///     async fn add(arg_a: f64, arg_b: f64, c: Option<f64>) -> f64 {
///         arg_a + arg_b + c.unwrap_or(0.0)
///     }
/// }
/// ```
///
/// # Ignoring methods
///
/// To omit some method to be assumed as a [GraphQL object][1] field and ignore
/// it, use an `ignore` attribute's argument directly on that method.
///
/// ```
/// # use juniper::graphql_object;
/// #
/// struct Human(String);
///
/// #[graphql_object]
/// impl Human {
///     fn id(&self) -> &str {
///         &self.0
///     }
///
///     #[graphql(ignore)]
///     fn kaboom(&mut self) {}
/// }
/// ```
///
/// # Custom context
///
/// By default, the generated implementation tries to infer [`Context`] type
/// from signatures of `impl` block methods, and uses [unit type `()`][4] if
/// signatures contains no [`Context`] arguments.
///
/// If [`Context`] type cannot be inferred or is inferred incorrectly, then
/// specify it explicitly with `context` attribute's argument.
///
/// If method argument is named as `context` or `ctx` then this argument is
/// assumed as [`Context`] and will be omitted in GraphQL schema.
/// Additionally, any argument may be marked as [`Context`] with a `context`
/// attribute's argument.
///
/// ```
/// # use std::collections::HashMap;
/// # use juniper::graphql_object;
/// #
/// struct Database {
///     humans: HashMap<String, Human>,
/// }
/// impl juniper::Context for Database {}
///
/// struct Human {
///     id: String,
///     home_planet: String,
/// }
///
/// #[graphql_object]
/// #[graphql(context = Database)]
/// impl Human {
///     fn id<'db>(&self, context: &'db Database) -> Option<&'db str> {
///         context.humans.get(&self.id).map(|h| h.id.as_str())
///     }
///     fn info<'db>(&self, context: &'db Database) -> Option<&'db str> {
///         context.humans.get(&self.id).map(|h| h.home_planet.as_str())
///     }
/// }
/// ```
///
/// # Using `Executor`
///
/// If an [`Executor`] is required in a method to resolve a [GraphQL object][1]
/// field, specify it as an argument named as `executor` or explicitly marked
/// with an `executor` attribute's argument. Such method argument will be
/// omitted in GraphQL schema.
///
/// However, this requires to explicitly parametrize over [`ScalarValue`], as
/// [`Executor`] does so.
///
/// ```
/// # use juniper::{graphql_object, Executor, GraphQLObject, ScalarValue};
/// #
/// struct Human {
///     name: String,
/// }
///
/// #[graphql_object]
/// // NOTICE: Specifying `ScalarValue` as custom named type parameter.
/// //         Its name should be similar to the one used in methods.
/// #[graphql(scalar = S: ScalarValue)]
/// impl Human {
///     async fn id<'a, S: ScalarValue>(
///         &self,
///         executor: &'a Executor<'_, '_, (), S>,
///     ) -> &'a str {
///         executor.look_ahead().field_name()
///     }
///
///     fn name<'b, S: ScalarValue>(
///         &'b self,
///         #[graphql(executor)] _another: &Executor<'_, '_, (), S>,
///     ) -> &'b str {
///         &self.name
///     }
/// }
/// ```
///
/// # Custom `ScalarValue`
///
/// By default, `#[graphql_object]` macro generates code, which is generic over
/// a [`ScalarValue`] type. This may introduce a problem when at least one of
/// its fields is restricted to a concrete [`ScalarValue`] type in its
/// implementation. To resolve such problem, a concrete [`ScalarValue`] type
/// should be specified with a `scalar` attribute's argument.
///
/// ```
/// # use juniper::{graphql_object, DefaultScalarValue, GraphQLObject};
/// #
/// struct Human(String);
///
/// #[graphql_object]
/// // NOTICE: Removing `scalar` argument will fail compilation.
/// #[graphql(scalar = DefaultScalarValue)]
/// impl Human {
///     fn id(&self) -> &str {
///         &self.0
///     }
///
///     fn helper(&self) -> Droid {
///         Droid {
///             id: self.0.clone(),
///         }
///     }
/// }
///
/// #[derive(GraphQLObject)]
/// #[graphql(scalar = DefaultScalarValue)]
/// struct Droid {
///     id: String,
/// }
/// ```
///
/// [`Context`]: juniper::Context
/// [`Executor`]: juniper::Executor
/// [`GraphQLType`]: juniper::GraphQLType
/// [`GraphQLValue`]: juniper::GraphQLValue
/// [`ScalarValue`]: juniper::ScalarValue
/// [0]: https://spec.graphql.org/October2021
/// [1]: https://spec.graphql.org/October2021#sec-Objects
#[proc_macro_attribute]
pub fn graphql_object(attr: TokenStream, body: TokenStream) -> TokenStream {
    diagnostic::entry_point_with_preserved_body(body.clone(), || {
        self::graphql_object::attr::expand(attr.into(), body.into())
            .unwrap_or_abort()
            .into()
    })
}

/// `#[graphql_subscription]` macro for generating a [GraphQL subscription][1]
/// implementation for structs with computable field resolvers (declared via
/// a regular Rust `impl` block).
///
/// It enables you to write GraphQL field resolvers for a type by declaring a
/// regular Rust `impl` block. Under the hood, the macro implements
/// [`GraphQLType`]/[`GraphQLSubscriptionValue`] traits.
///
/// Specifying multiple `#[graphql_subscription]` attributes on the same
/// definition is totally okay. They all will be treated as a single attribute.
///
/// This macro is similar to [`#[graphql_object]` macro](macro@graphql_object)
/// and has all its properties, but requires methods to be `async` and return
/// [`Stream`] of values instead of a value itself.
///
/// ```
/// # use futures::stream::{self, BoxStream};
/// use juniper::graphql_subscription;
///
/// // We can declare the type as a plain struct without any members.
/// struct Subscription;
///
/// #[graphql_subscription]
/// impl Subscription {
///     // WARNING: Only GraphQL fields can be specified in this `impl` block.
///     //          If normal methods are required on the struct, they can be
///     //          defined either in a separate "normal" `impl` block, or
///     //          marked with `#[graphql(ignore)]` attribute.
///
///     // This defines a simple, static field which does not require any
///     // context.
///     // Such field can return a `Stream` of any value implementing
///     // `GraphQLType` and `GraphQLValue` traits.
///     //
///     // NOTICE: Method must be `async`.
///     async fn api_version() -> BoxStream<'static, &'static str> {
///         Box::pin(stream::once(async { "0.1" }))
///     }
/// }
/// ```
///
/// [`GraphQLType`]: juniper::GraphQLType
/// [`GraphQLSubscriptionValue`]: juniper::GraphQLSubscriptionValue
/// [`Stream`]: futures::Stream
/// [1]: https://spec.graphql.org/October2021#sec-Subscription
#[proc_macro_attribute]
pub fn graphql_subscription(attr: TokenStream, body: TokenStream) -> TokenStream {
    diagnostic::entry_point_with_preserved_body(body.clone(), || {
        self::graphql_subscription::attr::expand(attr.into(), body.into())
            .unwrap_or_abort()
            .into()
    })
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
/// By default, the generated implementation uses [unit type `()`][4] as [`Context`]. To use a
/// custom [`Context`] type for [GraphQL union][1] variants types or external resolver functions,
/// specify it with `context` attribute's argument.
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
/// By default, this macro generates code, which is generic over a
/// [`ScalarValue`] type. This may introduce a problem when at least one of
/// [GraphQL union][1] variants is restricted to a concrete [`ScalarValue`] type
/// in its implementation. To resolve such problem, a concrete [`ScalarValue`]
/// type should be specified with a `scalar` attribute's argument.
///
/// ```
/// # use juniper::{DefaultScalarValue, GraphQLObject, GraphQLUnion};
/// #
/// #[derive(GraphQLObject)]
/// #[graphql(scalar = DefaultScalarValue)]
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
/// #[graphql(scalar = DefaultScalarValue)]
/// enum Character {
///     Human(Human),
///     Droid(Droid),
/// }
/// ```
///
/// # Ignoring enum variants
///
/// To omit exposing an enum variant in the GraphQL schema, use an `ignore`
/// attribute's argument directly on that variant.
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
///     #[graphql(ignore)]
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
/// [`Context`]: juniper::Context
/// [`ScalarValue`]: juniper::ScalarValue
/// [1]: https://spec.graphql.org/October2021#sec-Unions
/// [4]: https://doc.rust-lang.org/stable/std/primitive.unit.html
#[proc_macro_derive(GraphQLUnion, attributes(graphql))]
pub fn derive_union(body: TokenStream) -> TokenStream {
    diagnostic::entry_point(|| {
        self::graphql_union::derive::expand(body.into())
            .unwrap_or_abort()
            .into()
    })
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
/// #[graphql_union]
/// #[graphql(name = "Character", desc = "Possible episode characters.")]
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
/// #[graphql_union]
/// #[graphql(description = "Possible episode characters.")]
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
/// By default, the generated implementation tries to infer [`Context`] type from signatures of
/// trait methods, and uses [unit type `()`][4] if signatures contains no [`Context`] arguments.
///
/// If [`Context`] type cannot be inferred or is inferred incorrectly, then specify it explicitly
/// with `context` attribute's argument.
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
/// #[graphql_union]
/// #[graphql(context = Database)]
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
/// By default, `#[graphql_union]` macro generates code, which is generic over
/// a [`ScalarValue`] type. This may introduce a problem when at least one of
/// [GraphQL union][1] variants is restricted to a concrete [`ScalarValue`] type
/// in its implementation. To resolve such problem, a concrete [`ScalarValue`]
/// type should be specified with a `scalar` attribute's argument.
///
/// ```
/// # use juniper::{graphql_union, DefaultScalarValue, GraphQLObject};
/// #
/// #[derive(GraphQLObject)]
/// #[graphql(scalar = DefaultScalarValue)]
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
/// // NOTICE: Removing `scalar` argument will fail compilation.
/// #[graphql_union]
/// #[graphql(scalar = DefaultScalarValue)]
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
/// To omit some trait method to be assumed as a [GraphQL union][1] variant and
/// ignore it, use an `ignore` attribute's argument directly on that method.
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
///     #[graphql(ignore)]
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
/// #[graphql_union]
/// #[graphql(context = Database)]
/// #[graphql(
///     on Human = DynCharacter::get_human,
///     on Droid = get_droid,
/// )]
/// trait Character {
///     #[graphql(ignore)]
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
/// [`Context`]: juniper::Context
/// [`ScalarValue`]: juniper::ScalarValue
/// [1]: https://spec.graphql.org/October2021#sec-Unions
/// [2]: https://doc.rust-lang.org/stable/reference/items/traits.html#object-safety
/// [3]: https://doc.rust-lang.org/stable/reference/types/trait-object.html
/// [4]: https://doc.rust-lang.org/stable/std/primitive.unit.html
#[proc_macro_attribute]
pub fn graphql_union(attr: TokenStream, body: TokenStream) -> TokenStream {
    diagnostic::entry_point_with_preserved_body(body.clone(), || {
        self::graphql_union::attr::expand(attr.into(), body.into())
            .unwrap_or_abort()
            .into()
    })
}
