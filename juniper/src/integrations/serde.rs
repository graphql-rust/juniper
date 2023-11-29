use std::{fmt, marker::PhantomData};

use indexmap::IndexMap;
use serde::{
    de::{self, Deserializer, IntoDeserializer as _},
    ser::{SerializeMap as _, Serializer},
    serde_if_integer128, Deserialize, Serialize,
};

use crate::{
    ast::InputValue,
    executor::ExecutionError,
    parser::{ParseError, SourcePosition, Spanning},
    validation::RuleError,
    DefaultScalarValue, GraphQLError, Object, Value,
};

impl<T: Serialize> Serialize for ExecutionError<T> {
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        let mut map = ser.serialize_map(Some(4))?;

        map.serialize_key("message")?;
        map.serialize_value(self.error().message())?;

        let locations = vec![self.location()];
        map.serialize_key("locations")?;
        map.serialize_value(&locations)?;

        map.serialize_key("path")?;
        map.serialize_value(self.path())?;

        if !self.error().extensions().is_null() {
            map.serialize_key("extensions")?;
            map.serialize_value(self.error().extensions())?;
        }

        map.end()
    }
}

impl Serialize for GraphQLError {
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct Helper {
            message: &'static str,
        }

        match self {
            Self::ParseError(e) => [e].serialize(ser),
            Self::ValidationError(es) => es.serialize(ser),
            Self::NoOperationProvided => [Helper {
                message: "Must provide an operation",
            }]
            .serialize(ser),
            Self::MultipleOperationsProvided => [Helper {
                message: "Must provide operation name \
                          if query contains multiple operations",
            }]
            .serialize(ser),
            Self::UnknownOperationName => [Helper {
                message: "Unknown operation",
            }]
            .serialize(ser),
            Self::IsSubscription => [Helper {
                message: "Expected query, got subscription",
            }]
            .serialize(ser),
            Self::NotSubscription => [Helper {
                message: "Expected subscription, got query",
            }]
            .serialize(ser),
        }
    }
}

impl<'de, S: Deserialize<'de>> Deserialize<'de> for InputValue<S> {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        struct Visitor<S: ?Sized>(PhantomData<S>);

        impl<'de, S: Deserialize<'de>> de::Visitor<'de> for Visitor<S> {
            type Value = InputValue<S>;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a valid input value")
            }

            fn visit_bool<E: de::Error>(self, b: bool) -> Result<Self::Value, E> {
                S::deserialize(b.into_deserializer()).map(InputValue::Scalar)
            }

            fn visit_i8<E: de::Error>(self, n: i8) -> Result<Self::Value, E> {
                S::deserialize(n.into_deserializer()).map(InputValue::Scalar)
            }

            fn visit_i16<E: de::Error>(self, n: i16) -> Result<Self::Value, E> {
                S::deserialize(n.into_deserializer()).map(InputValue::Scalar)
            }

            fn visit_i32<E: de::Error>(self, n: i32) -> Result<Self::Value, E> {
                S::deserialize(n.into_deserializer()).map(InputValue::Scalar)
            }

            fn visit_i64<E: de::Error>(self, n: i64) -> Result<Self::Value, E> {
                S::deserialize(n.into_deserializer()).map(InputValue::Scalar)
            }

            serde_if_integer128! {
                fn visit_i128<E: de::Error>(self, n: i128) ->  Result<Self::Value, E> {
                    S::deserialize(n.into_deserializer()).map(InputValue::Scalar)
                }
            }

            fn visit_u8<E: de::Error>(self, n: u8) -> Result<Self::Value, E> {
                S::deserialize(n.into_deserializer()).map(InputValue::Scalar)
            }

            fn visit_u16<E: de::Error>(self, n: u16) -> Result<Self::Value, E> {
                S::deserialize(n.into_deserializer()).map(InputValue::Scalar)
            }

            fn visit_u32<E: de::Error>(self, n: u32) -> Result<Self::Value, E> {
                S::deserialize(n.into_deserializer()).map(InputValue::Scalar)
            }

            fn visit_u64<E: de::Error>(self, n: u64) -> Result<Self::Value, E> {
                S::deserialize(n.into_deserializer()).map(InputValue::Scalar)
            }

            serde_if_integer128! {
                fn visit_u128<E: de::Error>(self, n: u128) ->  Result<Self::Value, E> {
                    S::deserialize(n.into_deserializer()).map(InputValue::Scalar)
                }
            }

            fn visit_f32<E: de::Error>(self, n: f32) -> Result<Self::Value, E> {
                S::deserialize(n.into_deserializer()).map(InputValue::Scalar)
            }

            fn visit_f64<E: de::Error>(self, n: f64) -> Result<Self::Value, E> {
                S::deserialize(n.into_deserializer()).map(InputValue::Scalar)
            }

            fn visit_char<E: de::Error>(self, c: char) -> Result<Self::Value, E> {
                S::deserialize(c.into_deserializer()).map(InputValue::Scalar)
            }

            fn visit_str<E: de::Error>(self, s: &str) -> Result<Self::Value, E> {
                S::deserialize(s.into_deserializer()).map(InputValue::Scalar)
            }

            fn visit_string<E: de::Error>(self, s: String) -> Result<Self::Value, E> {
                S::deserialize(s.into_deserializer()).map(InputValue::Scalar)
            }

            fn visit_bytes<E: de::Error>(self, b: &[u8]) -> Result<Self::Value, E> {
                S::deserialize(b.into_deserializer()).map(InputValue::Scalar)
            }

            fn visit_byte_buf<E: de::Error>(self, b: Vec<u8>) -> Result<Self::Value, E> {
                S::deserialize(b.into_deserializer()).map(InputValue::Scalar)
            }

            fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(InputValue::Null)
            }

            fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(InputValue::Null)
            }

            fn visit_seq<V>(self, mut visitor: V) -> Result<Self::Value, V::Error>
            where
                V: de::SeqAccess<'de>,
            {
                let mut vals = Vec::new();
                while let Some(v) = visitor.next_element()? {
                    vals.push(v);
                }
                Ok(InputValue::list(vals))
            }

            fn visit_map<V>(self, mut visitor: V) -> Result<Self::Value, V::Error>
            where
                V: de::MapAccess<'de>,
            {
                let mut obj = IndexMap::<String, InputValue<S>>::with_capacity(
                    visitor.size_hint().unwrap_or(0),
                );
                while let Some((key, val)) = visitor.next_entry()? {
                    obj.insert(key, val);
                }
                Ok(InputValue::object(obj))
            }
        }

        de.deserialize_any(Visitor(PhantomData))
    }
}

impl<T: Serialize> Serialize for InputValue<T> {
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Null | Self::Variable(_) => ser.serialize_unit(),
            Self::Scalar(s) => s.serialize(ser),
            Self::Enum(e) => ser.serialize_str(e),
            Self::List(l) => l.iter().map(|x| &x.item).collect::<Vec<_>>().serialize(ser),
            Self::Object(o) => o
                .iter()
                .map(|(k, v)| (k.item.as_str(), &v.item))
                .collect::<IndexMap<_, _>>()
                .serialize(ser),
        }
    }
}

impl Serialize for RuleError {
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        let mut map = ser.serialize_map(Some(2))?;

        map.serialize_key("message")?;
        map.serialize_value(self.message())?;

        map.serialize_key("locations")?;
        map.serialize_value(self.locations())?;

        map.end()
    }
}

impl Serialize for SourcePosition {
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        let mut map = ser.serialize_map(Some(2))?;

        let line = self.line() + 1;
        map.serialize_key("line")?;
        map.serialize_value(&line)?;

        let column = self.column() + 1;
        map.serialize_key("column")?;
        map.serialize_value(&column)?;

        map.end()
    }
}

impl Serialize for Spanning<ParseError> {
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        let mut map = ser.serialize_map(Some(2))?;

        let msg = self.item.to_string();
        map.serialize_key("message")?;
        map.serialize_value(&msg)?;

        let mut loc = IndexMap::new();
        loc.insert("line".to_owned(), self.start().line() + 1);
        loc.insert("column".to_owned(), self.start().column() + 1);

        let locations = vec![loc];
        map.serialize_key("locations")?;
        map.serialize_value(&locations)?;

        map.end()
    }
}

impl<T: Serialize> Serialize for Object<T> {
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        let mut map = ser.serialize_map(Some(self.field_count()))?;
        for (f, v) in self.iter() {
            map.serialize_key(f)?;
            map.serialize_value(v)?;
        }
        map.end()
    }
}

impl<T: Serialize> Serialize for Value<T> {
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Null => ser.serialize_unit(),
            Self::Scalar(s) => s.serialize(ser),
            Self::List(l) => l.serialize(ser),
            Self::Object(o) => o.serialize(ser),
        }
    }
}

impl<'de> Deserialize<'de> for DefaultScalarValue {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = DefaultScalarValue;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a valid input value")
            }

            fn visit_bool<E: de::Error>(self, b: bool) -> Result<Self::Value, E> {
                Ok(DefaultScalarValue::Boolean(b))
            }

            fn visit_i64<E: de::Error>(self, n: i64) -> Result<Self::Value, E> {
                if n >= i64::from(i32::MIN) && n <= i64::from(i32::MAX) {
                    Ok(DefaultScalarValue::Int(n.try_into().unwrap()))
                } else {
                    // Browser's `JSON.stringify()` serializes all numbers
                    // having no fractional part as integers (no decimal point),
                    // so we must parse large integers as floating point,
                    // otherwise we would error on transferring large floating
                    // point numbers.
                    // TODO: Use `FloatToInt` conversion once stabilized:
                    //       https://github.com/rust-lang/rust/issues/67057
                    Ok(DefaultScalarValue::Float(n as f64))
                }
            }

            fn visit_u64<E: de::Error>(self, n: u64) -> Result<Self::Value, E> {
                if n <= u64::try_from(i32::MAX).unwrap() {
                    self.visit_i64(n.try_into().unwrap())
                } else {
                    // Browser's `JSON.stringify()` serializes all numbers
                    // having no fractional part as integers (no decimal point),
                    // so we must parse large integers as floating point,
                    // otherwise we would error on transferring large floating
                    // point numbers.
                    // TODO: Use `FloatToInt` conversion once stabilized:
                    //       https://github.com/rust-lang/rust/issues/67057
                    Ok(DefaultScalarValue::Float(n as f64))
                }
            }

            fn visit_f64<E: de::Error>(self, f: f64) -> Result<Self::Value, E> {
                Ok(DefaultScalarValue::Float(f))
            }

            fn visit_str<E: de::Error>(self, s: &str) -> Result<Self::Value, E> {
                self.visit_string(s.into())
            }

            fn visit_string<E: de::Error>(self, s: String) -> Result<Self::Value, E> {
                Ok(DefaultScalarValue::String(s))
            }
        }

        de.deserialize_any(Visitor)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{from_str, to_string};

    use crate::{
        ast::InputValue,
        graphql_input_value,
        value::{DefaultScalarValue, Object},
        FieldError, Value,
    };

    use super::{ExecutionError, GraphQLError};

    #[test]
    fn int() {
        assert_eq!(
            from_str::<InputValue>("1235").unwrap(),
            graphql_input_value!(1235),
        );
    }

    #[test]
    fn float() {
        assert_eq!(
            from_str::<InputValue>("2.0").unwrap(),
            graphql_input_value!(2.0),
        );
        // large value without a decimal part is also float
        assert_eq!(
            from_str::<InputValue>("123567890123").unwrap(),
            graphql_input_value!(123_567_890_123.0),
        );
    }

    #[test]
    fn errors() {
        assert_eq!(
            to_string(&GraphQLError::UnknownOperationName).unwrap(),
            r#"[{"message":"Unknown operation"}]"#,
        );
    }

    #[test]
    fn error_extensions() {
        let mut obj: Object<DefaultScalarValue> = Object::with_capacity(1);
        obj.add_field("foo", Value::scalar("bar"));
        assert_eq!(
            to_string(&ExecutionError::at_origin(FieldError::new(
                "foo error",
                Value::Object(obj),
            )))
            .unwrap(),
            r#"{"message":"foo error","locations":[{"line":1,"column":1}],"path":[],"extensions":{"foo":"bar"}}"#,
        );
    }
}
