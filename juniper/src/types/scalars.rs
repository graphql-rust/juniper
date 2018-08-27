use std::convert::From;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::ops::Deref;
use std::{char, u32};

use ast::{FromInputValue, InputValue, Selection, ToInputValue};
use executor::{Executor, Registry};
use parser::{LexerError, ParseError, Token};
use schema::meta::MetaType;
use types::base::GraphQLType;
use value::{ParseScalarValue, ScalarRefValue, ScalarValue, Value};

/// An ID as defined by the GraphQL specification
///
/// Represented as a string, but can be converted _to_ from an integer as well.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ID(String);

impl From<String> for ID {
    fn from(s: String) -> ID {
        ID(s)
    }
}

impl Deref for ID {
    type Target = str;

    fn deref(&self) -> &str {
        &self.0
    }
}

graphql_scalar!(ID as "ID" where Scalar = <S>{
    resolve(&self) -> Value {
        Value::string(&self.0)
    }

    from_input_value(v: &InputValue) -> Option<ID> {
        match *v {
            InputValue::Scalar(ref s) => {
                <_ as Into<Option<String>>>::into(s.clone()).or_else(||{
                    <_ as Into<Option<i32>>>::into(s.clone()).map(|i| i.to_string())
                }).map(ID)
            }
            _ => None
        }
    }

    from_str(value: &str) -> Result<S, ParseError> {
        Ok(S::from(value))
    }
});

graphql_scalar!(String as "String" where Scalar = <S>{
    resolve(&self) -> Value {
        Value::string(self)
    }

    from_input_value(v: &InputValue) -> Option<String> {
        match *v {
            InputValue::Scalar(ref s) => <_ as Into<Option<String>>>::into(s.clone()),
            _ => None,
        }
    }

    from_str(value: &str) -> Result<S, ParseError> {
        let mut ret = String::with_capacity(value.len());
        let mut char_iter = value.chars();
        while let Some(ch) = char_iter.next() {
            match ch {
                '\\' => {
                    match char_iter.next() {
                        Some('"') => {ret.push('"');}
                        Some('/') => {ret.push('/');}
                        Some('n') => {ret.push('\n');}
                        Some('r') => {ret.push('\r');}
                        Some('t') => {ret.push('\t');}
                        Some('\\') => {ret.push('\\');}
                        Some('f') => {ret.push('\u{000c}');}
                        Some('b') => {ret.push('\u{0008}');}
                        Some('u') => {
                            ret.push(parse_unicode_codepoint(&mut char_iter)?);
                        }
                        Some(s) => return Err(ParseError::LexerError(LexerError::UnknownEscapeSequence(format!("\\{}", s)))),
                        None => return Err(ParseError::LexerError(LexerError::UnterminatedString)),
                    }
                },
                ch => {ret.push(ch);}
            }
        }
        Ok(ret.into())
    }
});

fn parse_unicode_codepoint<'a, I>(char_iter: &mut I) -> Result<char, ParseError<'a>>
where
    I: Iterator<Item = char>,
{
    let escaped_code_point = char_iter
        .next()
        .ok_or_else(|| {
            ParseError::LexerError(LexerError::UnknownEscapeSequence(String::from("\\u")))
        }).and_then(|c1| {
            char_iter
                .next()
                .map(|c2| format!("{}{}", c1, c2))
                .ok_or_else(|| {
                    ParseError::LexerError(LexerError::UnknownEscapeSequence(format!("\\u{}", c1)))
                })
        }).and_then(|mut s| {
            char_iter
                .next()
                .ok_or_else(|| {
                    ParseError::LexerError(LexerError::UnknownEscapeSequence(format!(
                        "\\u{}",
                        s.clone()
                    )))
                }).map(|c2| {
                    s.push(c2);
                    s
                })
        }).and_then(|mut s| {
            char_iter
                .next()
                .ok_or_else(|| {
                    ParseError::LexerError(LexerError::UnknownEscapeSequence(format!(
                        "\\u{}",
                        s.clone()
                    )))
                }).map(|c2| {
                    s.push(c2);
                    s
                })
        })?;
    let code_point = u32::from_str_radix(&escaped_code_point, 16).map_err(|_| {
        ParseError::LexerError(LexerError::UnknownEscapeSequence(format!(
            "\\u{}",
            escaped_code_point
        )))
    })?;
    char::from_u32(code_point).ok_or_else(|| {
        ParseError::LexerError(LexerError::UnknownEscapeSequence(format!(
            "\\u{}",
            escaped_code_point
        )))
    })
}

impl<'a, S> GraphQLType<S> for &'a str
where
    S: ScalarValue,
    for<'b> &'b S: ScalarRefValue<'b>,
{
    type Context = ();
    type TypeInfo = ();

    fn name(_: &()) -> Option<&str> {
        Some("String")
    }

    fn meta<'r>(_: &(), registry: &mut Registry<'r, S>) -> MetaType<'r, S>
    where
        S: 'r,
        for<'b> &'b S: ScalarRefValue<'b>,
    {
        registry.build_scalar_type::<String>(&()).into_meta()
    }

    fn resolve(
        &self,
        _: &(),
        _: Option<&[Selection<S>]>,
        _: &Executor<S, Self::Context>,
    ) -> Value<S> {
        Value::string(self)
    }
}

impl<'a, S> ToInputValue<S> for &'a str
where
    S: ScalarValue,
{
    fn to_input_value(&self) -> InputValue<S> {
        InputValue::string(self)
    }
}

graphql_scalar!(bool as "Boolean" where Scalar = <S>{
    resolve(&self) -> Value {
        Value::boolean(*self)
    }

    from_input_value(v: &InputValue) -> Option<bool> {
        match *v {
            InputValue::Scalar(ref b) => <_ as Into<Option<bool>>>::into(b),
            _ => None,
        }
    }

    from_str(value: &str) -> Result<S, ParseError> {
        value
            .parse()
            .map_err(|_| ParseError::UnexpectedToken(Token::Scalar(value)))
            .map(|s: bool| s.into())
    }
});

graphql_scalar!(i32 as "Int" where Scalar = <S>{
    resolve(&self) -> Value {
        Value::int(*self)
    }

    from_input_value(v: &InputValue) -> Option<i32> {
        match *v {
            InputValue::Scalar(ref i) => <_ as Into<Option<i32>>>::into(i),
            _ => None,
        }
    }

     from_str(value: &str) -> Result<S, ParseError> {
        value
            .parse()
            .map_err(|_| ParseError::UnexpectedToken(Token::Scalar(value)))
            .map(|s: i32| s.into())
    }
});

graphql_scalar!(f64 as "Float" where Scalar = <S>{
    resolve(&self) -> Value {
        Value::float(*self)
    }

    from_input_value(v: &InputValue) -> Option<f64> {
        match *v {
            InputValue::Scalar(ref s) => {
                <_ as Into<Option<f64>>>::into(s)
            }
            _ => None,
        }
    }

    from_str(value: &str) -> Result<S, ParseError> {
        value
            .parse()
            .map_err(|_| ParseError::UnexpectedToken(Token::Scalar(value)))
            .map(|s: f64| s.into())
    }
});

impl<S> GraphQLType<S> for ()
where
    S: ScalarValue,
    for<'b> &'b S: ScalarRefValue<'b>,
{
    type Context = ();
    type TypeInfo = ();

    fn name(_: &()) -> Option<&str> {
        Some("__Unit")
    }

    fn meta<'r>(_: &(), registry: &mut Registry<'r, S>) -> MetaType<'r, S>
    where
        S: 'r,
        for<'b> &'b S: ScalarRefValue<'b>,
    {
        registry.build_scalar_type::<Self>(&()).into_meta()
    }
}

impl<S> ParseScalarValue<S> for ()
where
    S: ScalarValue,
{
    fn from_str(_value: &str) -> Result<S, ParseError> {
        Ok(S::from(0))
    }
}

impl<S: Debug> FromInputValue<S> for () {
    fn from_input_value<'a>(_: &'a InputValue<S>) -> Option<()>
    where
        for<'b> &'b S: ScalarRefValue<'b>,
    {
        None
    }
}

/// Utility type to define read-only schemas
///
/// If you instantiate `RootNode` with this as the mutation, no mutation will be
/// generated for the schema.
#[derive(Debug)]
pub struct EmptyMutation<T> {
    phantom: PhantomData<T>,
}

impl<T> EmptyMutation<T> {
    /// Construct a new empty mutation
    pub fn new() -> EmptyMutation<T> {
        EmptyMutation {
            phantom: PhantomData,
        }
    }
}

impl<S, T> GraphQLType<S> for EmptyMutation<T>
where
    S: ScalarValue,
    for<'b> &'b S: ScalarRefValue<'b>,
{
    type Context = T;
    type TypeInfo = ();

    fn name(_: &()) -> Option<&str> {
        Some("_EmptyMutation")
    }

    fn meta<'r>(_: &(), registry: &mut Registry<'r, S>) -> MetaType<'r, S>
    where
        S: 'r,
        for<'b> &'b S: ScalarRefValue<'b>,
    {
        registry.build_object_type::<Self>(&(), &[]).into_meta()
    }
}

#[cfg(test)]
mod tests {
    use super::ID;

    #[test]
    fn test_id_from_string() {
        let actual = ID::from(String::from("foo"));
        let expected = ID(String::from("foo"));
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_id_deref() {
        let id = ID(String::from("foo"));
        assert_eq!(id.len(), 3);
    }
}
