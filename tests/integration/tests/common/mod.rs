use std::fmt;

use juniper::ScalarValue;
use serde::{de, Deserialize, Deserializer, Serialize};

/// Common utilities used across tests.
pub mod util {
    use futures::StreamExt as _;
    use juniper::{
        graphql_value, DefaultScalarValue, EmptyMutation, EmptySubscription, ExecutionError,
        GraphQLError, GraphQLType, RootNode, ScalarValue, Value, ValuesStream,
    };

    pub fn schema<C, Q>(query_root: Q) -> RootNode<Q, EmptyMutation<C>, EmptySubscription<C>>
    where
        Q: GraphQLType<DefaultScalarValue, Context = C, TypeInfo = ()>,
    {
        RootNode::new(
            query_root,
            EmptyMutation::<C>::new(),
            EmptySubscription::<C>::new(),
        )
    }

    pub fn schema_with_scalar<S, C, Q>(
        query_root: Q,
    ) -> RootNode<Q, EmptyMutation<C>, EmptySubscription<C>, S>
    where
        Q: GraphQLType<S, Context = C, TypeInfo = ()>,
        S: ScalarValue,
    {
        RootNode::new_with_scalar_value(
            query_root,
            EmptyMutation::<C>::new(),
            EmptySubscription::<C>::new(),
        )
    }

    /// Extracts a single next value from the result returned by
    /// [`juniper::resolve_into_stream()`] and transforms it into a regular
    /// [`Value`].
    ///
    /// # Errors
    ///
    /// Propagates the `input` [`GraphQLError`], if any.
    ///
    /// # Panics
    ///
    /// If the `input` [`Value`] doesn't represent a [`Value::Object`] containing a [`Stream`].
    ///
    /// [`Stream`]: futures::Stream
    #[allow(clippy::type_complexity)]
    pub async fn extract_next<S: ScalarValue>(
        input: Result<(Value<ValuesStream<'_, S>>, Vec<ExecutionError<S>>), GraphQLError>,
    ) -> Result<(Value<S>, Vec<ExecutionError<S>>), GraphQLError> {
        let (stream, errs) = input?;
        if !errs.is_empty() {
            return Ok((Value::Null, errs));
        }

        if let Value::Object(obj) = stream {
            for (name, mut val) in obj {
                if let Value::Scalar(ref mut stream) = val {
                    return match stream.next().await {
                        Some(Ok(val)) => Ok((graphql_value!({ name: val }), vec![])),
                        Some(Err(e)) => Ok((Value::Null, vec![e])),
                        None => Ok((Value::Null, vec![])),
                    };
                }
            }
        }

        panic!("Expected to get Value::Object containing a Stream")
    }
}

#[derive(Clone, Debug, PartialEq, ScalarValue, Serialize)]
#[serde(untagged)]
pub enum MyScalarValue {
    #[value(as_float, as_int)]
    Int(i32),
    Long(i64),
    #[value(as_float)]
    Float(f64),
    #[value(as_str, as_string, into_string)]
    String(String),
    #[value(as_bool)]
    Boolean(bool),
}

impl<'de> Deserialize<'de> for MyScalarValue {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = MyScalarValue;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a valid input value")
            }

            fn visit_bool<E: de::Error>(self, b: bool) -> Result<Self::Value, E> {
                Ok(MyScalarValue::Boolean(b))
            }

            fn visit_i32<E: de::Error>(self, n: i32) -> Result<Self::Value, E> {
                Ok(MyScalarValue::Int(n))
            }

            fn visit_i64<E: de::Error>(self, b: i64) -> Result<Self::Value, E> {
                if b <= i64::from(i32::MAX) {
                    self.visit_i32(b.try_into().unwrap())
                } else {
                    Ok(MyScalarValue::Long(b))
                }
            }

            fn visit_u32<E: de::Error>(self, n: u32) -> Result<Self::Value, E> {
                if n <= i32::MAX as u32 {
                    self.visit_i32(n.try_into().unwrap())
                } else {
                    self.visit_u64(n.into())
                }
            }

            fn visit_u64<E: de::Error>(self, n: u64) -> Result<Self::Value, E> {
                if n <= i64::MAX as u64 {
                    self.visit_i64(n.try_into().unwrap())
                } else {
                    // Browser's `JSON.stringify()` serializes all numbers
                    // having no fractional part as integers (no decimal point),
                    // so we must parse large integers as floating point,
                    // otherwise we would error on transferring large floating
                    // point numbers.
                    // TODO: Use `FloatToInt` conversion once stabilized:
                    //       https://github.com/rust-lang/rust/issues/67057
                    Ok(MyScalarValue::Float(n as f64))
                }
            }

            fn visit_f64<E: de::Error>(self, f: f64) -> Result<Self::Value, E> {
                Ok(MyScalarValue::Float(f))
            }

            fn visit_str<E: de::Error>(self, s: &str) -> Result<Self::Value, E> {
                self.visit_string(s.into())
            }

            fn visit_string<E: de::Error>(self, s: String) -> Result<Self::Value, E> {
                Ok(MyScalarValue::String(s))
            }
        }

        de.deserialize_any(Visitor)
    }
}

/// Definitions shadowing [`std::prelude`] items to check whether macro expansion is hygienic.
pub mod hygiene {
    pub use std::prelude::rust_2021 as prelude;

    pub trait Debug {}

    pub trait Display {}

    pub struct Box<T>(T);

    pub trait Clone {}

    pub trait Copy {}

    pub trait Future {}

    pub struct Option<T>(T);

    pub struct PhantomData<T>(T);

    pub struct Result<Ok, Err>(Ok, Err);

    pub trait Send {}

    pub struct String;

    pub trait Sync {}
}
