use std::{marker::PhantomData, rc::Rc, thread::JoinHandle};

use derive_more::with_trait::{Deref, Display, From, Into};
use serde::{Deserialize, Serialize};

use crate::{
    GraphQLScalar, IntoFieldError, Scalar,
    ast::{InputValue, Selection, ToInputValue},
    executor::{ExecutionResult, Executor, Registry},
    graphql_scalar,
    macros::reflect,
    parser::{ParseError, ScalarToken, Token},
    schema::meta::MetaType,
    types::{
        async_await::GraphQLValueAsync,
        base::{GraphQLType, GraphQLValue},
        subscriptions::GraphQLSubscriptionValue,
    },
    value::{
        FromScalarValue, ParseScalarResult, ScalarValue, ToScalarValue, TryToPrimitive, Value,
        WrongInputScalarTypeError,
    },
};

/// An ID as defined by the GraphQL specification
///
/// Represented as a string, but can be converted _to_ from an integer as well.
#[derive(
    Clone, Debug, Deref, Deserialize, Display, Eq, From, GraphQLScalar, Into, PartialEq, Serialize,
)]
#[deref(forward)]
#[from(Box<str>, String)]
#[into(Box<str>, String)]
#[graphql(parse_token(String, i32))]
pub struct ID(Box<str>);

impl ID {
    fn to_output(&self) -> &str {
        &self.0
    }

    fn from_input<S: ScalarValue>(v: &Scalar<S>) -> Result<Self, WrongInputScalarTypeError<'_, S>> {
        v.try_to_string()
            .or_else(|| v.try_to_int().as_ref().map(ToString::to_string))
            .map(|s| Self(s.into()))
            .ok_or_else(|| WrongInputScalarTypeError {
                type_name: arcstr::literal!("String` or `Int"),
                input: &**v,
            })
    }
}

impl ID {
    /// Construct a new [`ID`] from anything implementing [`Into`]`<`[`String`]`>`.
    #[must_use]
    pub fn new<S: Into<String>>(value: S) -> Self {
        ID(value.into().into())
    }
}

#[graphql_scalar]
#[graphql(
    with = impl_string_scalar,
    to_output_with = String::as_str
    from_input_with = __builtin,
)]
type String = std::string::String;

mod impl_string_scalar {
    use super::*;

    impl<'s, S> FromScalarValue<'s, S> for String
    where
        S: TryToPrimitive<'s, Self, Error: IntoFieldError<S>> + 's,
    {
        type Error = S::Error;

        fn from_scalar_value(v: &'s S) -> Result<Self, Self::Error> {
            v.try_to_primitive()
        }
    }

    pub(super) fn parse_token<S: ScalarValue>(value: ScalarToken<'_>) -> ParseScalarResult<S> {
        if let ScalarToken::String(lit) = value {
            let parsed = lit.parse()?;
            // TODO: Allow cheaper from `Cow<'_, str>` conversions for `ScalarValue`.
            Ok(parsed.into_owned().into())
        } else {
            Err(ParseError::unexpected_token(Token::Scalar(value)))
        }
    }
}

#[graphql_scalar]
#[graphql(
    name = "String", 
    with = impl_arcstr_scalar,
    to_output_with = ScalarValue::from_displayable,
    parse_token(String)
)]
type ArcStr = arcstr::ArcStr;

mod impl_arcstr_scalar {
    use super::ArcStr;
    use crate::{FromScalarValue, Scalar, ScalarValue};

    pub(super) fn from_input<S: ScalarValue>(
        v: &Scalar<S>,
    ) -> Result<ArcStr, <&str as FromScalarValue<'_, S>>::Error> {
        if let Some(s) = v.downcast_type::<ArcStr>() {
            Ok(s.clone())
        } else {
            v.try_to::<&str>().map(ArcStr::from)
        }
    }
}

#[graphql_scalar]
#[graphql(
    name = "String", 
    with = impl_compactstring_scalar,
    to_output_with = ScalarValue::from_displayable,
    parse_token(String),
)]
type CompactString = compact_str::CompactString;

mod impl_compactstring_scalar {
    use super::CompactString;
    use crate::{FromScalarValue, Scalar, ScalarValue};

    pub(super) fn from_input<S: ScalarValue>(
        v: &Scalar<S>,
    ) -> Result<CompactString, <&str as FromScalarValue<'_, S>>::Error> {
        if let Some(s) = v.downcast_type::<CompactString>() {
            Ok(s.clone())
        } else {
            v.try_to::<&str>().map(CompactString::from)
        }
    }
}

impl<S> reflect::WrappedType<S> for str {
    const VALUE: reflect::WrappedValue = 1;
}

impl<S> reflect::BaseType<S> for str {
    const NAME: reflect::Type = "String";
}

impl<S> reflect::BaseSubTypes<S> for str {
    const NAMES: reflect::Types = &[<Self as reflect::BaseType<S>>::NAME];
}

impl<S> GraphQLType<S> for str
where
    S: ScalarValue,
{
    fn name(_: &()) -> Option<ArcStr> {
        Some(arcstr::literal!("String"))
    }

    fn meta(_: &(), registry: &mut Registry<S>) -> MetaType<S> {
        registry.build_scalar_type::<String>(&()).into_meta()
    }
}

impl<S> GraphQLValue<S> for str
where
    S: ScalarValue,
{
    type Context = ();
    type TypeInfo = ();

    fn type_name(&self, info: &Self::TypeInfo) -> Option<ArcStr> {
        <Self as GraphQLType<S>>::name(info)
    }

    fn resolve(
        &self,
        _: &(),
        _: Option<&[Selection<S>]>,
        _: &Executor<Self::Context, S>,
    ) -> ExecutionResult<S> {
        Ok(Value::Scalar(self.to_scalar_value()))
    }
}

impl<S> GraphQLValueAsync<S> for str
where
    S: ScalarValue + Send + Sync,
{
    fn resolve_async<'a>(
        &'a self,
        info: &'a Self::TypeInfo,
        selection_set: Option<&'a [Selection<S>]>,
        executor: &'a Executor<Self::Context, S>,
    ) -> crate::BoxFuture<'a, ExecutionResult<S>> {
        use futures::future;
        Box::pin(future::ready(self.resolve(info, selection_set, executor)))
    }
}

impl<'s, S> FromScalarValue<'s, S> for &'s str
where
    S: TryToPrimitive<'s, Self, Error: IntoFieldError<S>> + 's,
{
    type Error = S::Error;

    fn from_scalar_value(v: &'s S) -> Result<Self, Self::Error> {
        v.try_to_primitive()
    }
}

impl<S: ScalarValue> ToScalarValue<S> for str {
    fn to_scalar_value(&self) -> S {
        S::from_displayable(self)
    }
}

impl<S> ToInputValue<S> for str
where
    Self: ToScalarValue<S>,
{
    fn to_input_value(&self) -> InputValue<S> {
        InputValue::Scalar(self.to_scalar_value())
    }
}

#[graphql_scalar]
#[graphql(with = impl_boolean_scalar, from_input_with = __builtin)]
type Boolean = bool;

mod impl_boolean_scalar {
    use super::*;

    impl<'s, S> FromScalarValue<'s, S> for Boolean
    where
        S: TryToPrimitive<'s, Self, Error: IntoFieldError<S>> + 's,
    {
        type Error = S::Error;

        fn from_scalar_value(v: &'s S) -> Result<Self, Self::Error> {
            v.try_to_primitive()
        }
    }

    pub(super) fn to_output<S: ScalarValue>(v: &Boolean) -> S {
        (*v).into()
    }

    pub(super) fn parse_token<S: ScalarValue>(value: ScalarToken<'_>) -> ParseScalarResult<S> {
        // `Boolean`s are parsed separately, they shouldn't reach this code path.
        Err(ParseError::unexpected_token(Token::Scalar(value)))
    }
}

#[graphql_scalar]
#[graphql(with = impl_int_scalar, from_input_with = __builtin)]
type Int = i32;

mod impl_int_scalar {
    use super::*;

    impl<'s, S> FromScalarValue<'s, S> for Int
    where
        S: TryToPrimitive<'s, Self, Error: IntoFieldError<S>> + 's,
    {
        type Error = S::Error;

        fn from_scalar_value(v: &'s S) -> Result<Self, Self::Error> {
            v.try_to_primitive()
        }
    }

    pub(super) fn to_output<S: ScalarValue>(v: &Int) -> S {
        (*v).into()
    }

    pub(super) fn parse_token<S: ScalarValue>(value: ScalarToken<'_>) -> ParseScalarResult<S> {
        if let ScalarToken::Int(v) = value {
            v.parse()
                .map_err(|_| ParseError::unexpected_token(Token::Scalar(value)))
                .map(|s: i32| s.into())
        } else {
            Err(ParseError::unexpected_token(Token::Scalar(value)))
        }
    }
}

#[graphql_scalar]
#[graphql(with = impl_float_scalar, from_input_with = __builtin)]
type Float = f64;

mod impl_float_scalar {
    use super::*;

    impl<'s, S> FromScalarValue<'s, S> for Float
    where
        S: TryToPrimitive<'s, Self, Error: IntoFieldError<S>> + 's,
    {
        type Error = S::Error;

        fn from_scalar_value(v: &'s S) -> Result<Self, Self::Error> {
            v.try_to_primitive()
        }
    }

    pub(super) fn to_output<S: ScalarValue>(v: &Float) -> S {
        (*v).into()
    }

    pub(super) fn parse_token<S: ScalarValue>(value: ScalarToken<'_>) -> ParseScalarResult<S> {
        match value {
            ScalarToken::Int(v) => v
                .parse()
                .map_err(|_| ParseError::unexpected_token(Token::Scalar(value)))
                .map(|s: i32| f64::from(s).into()),
            ScalarToken::Float(v) => v
                .parse()
                .map_err(|_| ParseError::unexpected_token(Token::Scalar(value)))
                .map(|s: f64| s.into()),
            ScalarToken::String(_) => Err(ParseError::unexpected_token(Token::Scalar(value))),
        }
    }
}

/// Utility type to define read-only schemas
///
/// If you instantiate `RootNode` with this as the mutation, no mutation will be
/// generated for the schema.
#[derive(Debug)]
pub struct EmptyMutation<T: ?Sized = ()>(PhantomData<JoinHandle<Box<T>>>);

// `EmptyMutation` doesn't use `T`, so should be `Send` and `Sync` even when `T` is not.
crate::sa::assert_impl_all!(EmptyMutation<Rc<String>>: Send, Sync);

impl<T: ?Sized> EmptyMutation<T> {
    /// Construct a new empty mutation
    #[inline]
    pub fn new() -> Self {
        Self(PhantomData)
    }
}

impl<S, T> GraphQLType<S> for EmptyMutation<T>
where
    S: ScalarValue,
{
    fn name(_: &()) -> Option<ArcStr> {
        Some(arcstr::literal!("_EmptyMutation"))
    }

    fn meta(_: &(), registry: &mut Registry<S>) -> MetaType<S> {
        registry.build_object_type::<Self>(&(), &[]).into_meta()
    }
}

impl<S, T> GraphQLValue<S> for EmptyMutation<T>
where
    S: ScalarValue,
{
    type Context = T;
    type TypeInfo = ();

    fn type_name(&self, info: &Self::TypeInfo) -> Option<ArcStr> {
        <Self as GraphQLType<S>>::name(info)
    }
}

impl<S, T> GraphQLValueAsync<S> for EmptyMutation<T>
where
    Self::TypeInfo: Sync,
    Self::Context: Sync,
    S: ScalarValue + Send + Sync,
{
}

// Implemented manually to omit redundant `T: Default` trait bound, imposed by
// `#[derive(Default)]`.
impl<T> Default for EmptyMutation<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Utillity type to define read-only schemas
///
/// If you instantiate `RootNode` with this as the subscription,
/// no subscriptions will be generated for the schema.
pub struct EmptySubscription<T: ?Sized = ()>(PhantomData<JoinHandle<Box<T>>>);

// `EmptySubscription` doesn't use `T`, so should be `Send` and `Sync` even when `T` is not.
crate::sa::assert_impl_all!(EmptySubscription<Rc<String>>: Send, Sync);

impl<T: ?Sized> EmptySubscription<T> {
    /// Construct a new empty subscription
    #[inline]
    pub fn new() -> Self {
        Self(PhantomData)
    }
}

impl<S, T> GraphQLType<S> for EmptySubscription<T>
where
    S: ScalarValue,
{
    fn name(_: &()) -> Option<ArcStr> {
        Some(arcstr::literal!("_EmptySubscription"))
    }

    fn meta(_: &(), registry: &mut Registry<S>) -> MetaType<S> {
        registry.build_object_type::<Self>(&(), &[]).into_meta()
    }
}

impl<S, T> GraphQLValue<S> for EmptySubscription<T>
where
    S: ScalarValue,
{
    type Context = T;
    type TypeInfo = ();

    fn type_name(&self, info: &Self::TypeInfo) -> Option<ArcStr> {
        <Self as GraphQLType<S>>::name(info)
    }
}

impl<T, S> GraphQLSubscriptionValue<S> for EmptySubscription<T>
where
    Self::TypeInfo: Sync,
    Self::Context: Sync,
    S: ScalarValue + Send + Sync + 'static,
{
}

// Implemented manually to omit redundant `T: Default` trait bound, imposed by
// `#[derive(Default)]`.
impl<T> Default for EmptySubscription<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        parser::{ScalarToken, StringLiteral},
        value::{DefaultScalarValue, ParseScalarValue, ScalarValue as _},
    };

    use super::{EmptyMutation, EmptySubscription, ID};

    #[test]
    fn test_id_from_string() {
        let actual = ID::from(String::from("foo"));
        let expected = ID("foo".into());
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_id_new() {
        let actual = ID::new("foo");
        let expected = ID("foo".into());
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_id_deref() {
        let id = ID("foo".into());
        assert_eq!(id.len(), 3);
    }

    #[test]
    fn test_id_display() {
        let id = ID("foo".into());
        assert_eq!(id.to_string(), "foo");
    }

    #[test]
    fn parse_strings() {
        for (input, expected) in [
            (r#""simple""#, "simple"),
            (r#"" white space ""#, " white space "),
            (r#""quote \"""#, r#"quote ""#),
            (r#""escaped \n\r\b\t\f""#, "escaped \n\r\u{0008}\t\u{000c}"),
            (r#""slashes \\ \/""#, r"slashes \ /"),
            (
                r#""unicode \u1234\u5678\u90AB\uCDEF""#,
                "unicode \u{1234}\u{5678}\u{90ab}\u{cdef}",
            ),
        ] {
            let res = <String as ParseScalarValue<DefaultScalarValue>>::from_str(
                ScalarToken::String(StringLiteral::Quoted(input)),
            );
            assert!(res.is_ok(), "parsing error occurred: {}", res.unwrap_err());

            let s: Option<String> = res.unwrap().try_to().ok();
            assert!(s.is_some(), "no string returned");
            assert_eq!(s.unwrap(), expected);
        }
    }

    #[test]
    fn parse_block_strings() {
        for (input, expected) in [
            (r#""""""""#, ""),
            (r#""""simple""""#, "simple"),
            (r#"""" white space """"#, " white space "),
            (r#""""contains " quote""""#, r#"contains " quote"#),
            (
                r#""""contains \""" triple quote""""#,
                r#"contains """ triple quote"#,
            ),
            (
                r#""""contains \"" double quote""""#,
                r#"contains \"" double quote"#,
            ),
            (
                r#""""contains \\""" triple quote""""#,
                r#"contains \""" triple quote"#,
            ),
            (r#""""\"""quote" """"#, r#""""quote" "#),
            (r#""""multi\nline""""#, r"multi\nline"),
            (
                r#""""multi\rline\r\nnormalized""""#,
                r"multi\rline\r\nnormalized",
            ),
            (
                r#""""unescaped \\n\\r\\b\\t\\f\\u1234""""#,
                r"unescaped \\n\\r\\b\\t\\f\\u1234",
            ),
            (
                r#""""unescaped unicode outside BMP \u{1f600}""""#,
                r"unescaped unicode outside BMP \u{1f600}",
            ),
            (r#""""slashes \\\\ \\/""""#, r"slashes \\\\ \\/"),
            (
                r#""""

        spans
          multiple
            lines

        """"#,
                "spans\n  multiple\n    lines",
            ),
            // removes uniform indentation
            (
                r#""""
    Hello,
      World!

    Yours,
      GraphQL.""""#,
                "Hello,\n  World!\n\nYours,\n  GraphQL.",
            ),
            // removes empty leading and trailing lines
            (
                r#""""

    Hello,
      World!

    Yours,
      GraphQL.

        """"#,
                "Hello,\n  World!\n\nYours,\n  GraphQL.",
            ),
            // retains indentation from first line
            (
                r#""""    Hello,
      World!

    Yours,
      GraphQL.""""#,
                "    Hello,\n  World!\n\nYours,\n  GraphQL.",
            ),
            // does not alter trailing spaces
            (
                r#""""
    Hello,
      World!

    Yours,
      GraphQL.   """"#,
                "Hello,\n  World!\n\nYours,\n  GraphQL.   ",
            ),
        ] {
            let res = <String as ParseScalarValue<DefaultScalarValue>>::from_str(
                ScalarToken::String(StringLiteral::Block(input)),
            );
            assert!(res.is_ok(), "parsing error occurred: {}", res.unwrap_err());

            let s: Option<String> = res.unwrap().try_to().ok();
            assert!(s.is_some(), "no string returned");
            assert_eq!(s.unwrap(), expected);
        }
    }

    #[test]
    fn parse_f64_from_int() {
        for (v, expected) in [
            ("0", 0),
            ("128", 128),
            ("1601942400", 1601942400),
            ("1696550400", 1696550400),
            ("-1", -1),
        ] {
            let n = <f64 as ParseScalarValue<DefaultScalarValue>>::from_str(ScalarToken::Int(v));
            assert!(n.is_ok(), "A parsing error occurred: {:?}", n.unwrap_err());

            let n: Option<f64> = n.unwrap().try_to().ok();
            assert!(n.is_some(), "No `f64` returned");
            assert_eq!(n.unwrap(), f64::from(expected));
        }
    }

    #[test]
    fn parse_f64_from_float() {
        for (v, expected) in [
            ("0.", 0.),
            ("1.2", 1.2),
            ("1601942400.", 1601942400.),
            ("1696550400.", 1696550400.),
            ("-1.2", -1.2),
        ] {
            let n = <f64 as ParseScalarValue<DefaultScalarValue>>::from_str(ScalarToken::Float(v));
            assert!(n.is_ok(), "A parsing error occurred: {:?}", n.unwrap_err());

            let n: Option<f64> = n.unwrap().try_to().ok();
            assert!(n.is_some(), "No `f64` returned");
            assert_eq!(n.unwrap(), expected);
        }
    }

    #[test]
    fn empty_mutation_is_send() {
        fn check_if_send<T: Send>() {}
        check_if_send::<EmptyMutation<()>>();
    }

    #[test]
    fn empty_subscription_is_send() {
        fn check_if_send<T: Send>() {}
        check_if_send::<EmptySubscription<()>>();
    }

    #[test]
    fn default_is_invariant_over_type() {
        struct Bar;
        let _ = EmptySubscription::<Bar>::default();
        let _ = EmptyMutation::<Bar>::default();
    }
}
