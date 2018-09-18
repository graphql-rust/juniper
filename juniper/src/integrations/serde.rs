use indexmap::IndexMap;
use serde::ser::SerializeMap;
use serde::{de, ser};

use std::fmt;

use ast::InputValue;
use executor::ExecutionError;
use parser::{ParseError, SourcePosition, Spanning};
use validation::RuleError;
use {GraphQLError, Object, ScalarValue, Value};

#[derive(Serialize)]
struct SerializeHelper {
    message: &'static str,
}

impl<T> ser::Serialize for ExecutionError<T>
where
    T: ScalarValue,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        let mut map = serializer.serialize_map(Some(4))?;

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

impl<'a> ser::Serialize for GraphQLError<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        match *self {
            GraphQLError::ParseError(ref err) => vec![err].serialize(serializer),
            GraphQLError::ValidationError(ref errs) => errs.serialize(serializer),
            GraphQLError::NoOperationProvided => [SerializeHelper {
                message: "Must provide an operation",
            }]
                .serialize(serializer),
            GraphQLError::MultipleOperationsProvided => [SerializeHelper {
                message: "Must provide operation name \
                          if query contains multiple operations",
            }]
                .serialize(serializer),
            GraphQLError::UnknownOperationName => [SerializeHelper {
                message: "Unknown operation",
            }]
                .serialize(serializer),
        }
    }
}

impl<'de, S> de::Deserialize<'de> for InputValue<S>
where
    S: de::Deserialize<'de> + ScalarValue,
{
    fn deserialize<D>(deserializer: D) -> Result<InputValue<S>, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct InputValueVisitor<S>(::std::marker::PhantomData<S>);

        impl<S> Default for InputValueVisitor<S> {
            fn default() -> Self {
                InputValueVisitor(Default::default())
            }
        }

        impl<'de, S> de::Visitor<'de> for InputValueVisitor<S>
        where
            S: ScalarValue + de::Deserialize<'de>,
        {
            type Value = InputValue<S>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a valid input value")
            }

            fn visit_bool<E>(self, value: bool) -> Result<InputValue<S>, E> {
                Ok(InputValue::scalar(value))
            }

            fn visit_i64<E>(self, value: i64) -> Result<InputValue<S>, E>
            where
                E: de::Error,
            {
                if value >= i64::from(i32::min_value()) && value <= i64::from(i32::max_value()) {
                    Ok(InputValue::scalar(value as i32))
                } else {
                    // Browser's JSON.stringify serialize all numbers having no
                    // fractional part as integers (no decimal point), so we
                    // must parse large integers as floating point otherwise
                    // we would error on transferring large floating point
                    // numbers.
                    Ok(InputValue::scalar(value as f64))
                }
            }

            fn visit_u64<E>(self, value: u64) -> Result<InputValue<S>, E>
            where
                E: de::Error,
            {
                if value <= i32::max_value() as u64 {
                    self.visit_i64(value as i64)
                } else {
                    // Browser's JSON.stringify serialize all numbers having no
                    // fractional part as integers (no decimal point), so we
                    // must parse large integers as floating point otherwise
                    // we would error on transferring large floating point
                    // numbers.
                    Ok(InputValue::scalar(value as f64))
                }
            }

            fn visit_f64<E>(self, value: f64) -> Result<InputValue<S>, E> {
                Ok(InputValue::scalar(value))
            }

            fn visit_str<E>(self, value: &str) -> Result<InputValue<S>, E>
            where
                E: de::Error,
            {
                self.visit_string(value.into())
            }

            fn visit_string<E>(self, value: String) -> Result<InputValue<S>, E> {
                Ok(InputValue::scalar(value))
            }

            fn visit_none<E>(self) -> Result<InputValue<S>, E> {
                Ok(InputValue::null())
            }

            fn visit_unit<E>(self) -> Result<InputValue<S>, E> {
                Ok(InputValue::null())
            }

            fn visit_seq<V>(self, mut visitor: V) -> Result<InputValue<S>, V::Error>
            where
                V: de::SeqAccess<'de>,
            {
                let mut values = Vec::new();

                while let Some(el) = visitor.next_element()? {
                    values.push(el);
                }

                Ok(InputValue::list(values))
            }

            fn visit_map<V>(self, mut visitor: V) -> Result<InputValue<S>, V::Error>
            where
                V: de::MapAccess<'de>,
            {
                let mut object = IndexMap::<String, InputValue<S>>::with_capacity(
                    visitor.size_hint().unwrap_or(0),
                );

                while let Some((key, value)) = visitor.next_entry()? {
                    object.insert(key, value);
                }

                Ok(InputValue::object(object))
            }
        }

        deserializer.deserialize_any(InputValueVisitor::default())
    }
}

impl<T> ser::Serialize for InputValue<T>
where
    T: ScalarValue,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        match *self {
            InputValue::Null | InputValue::Variable(_) => serializer.serialize_unit(),
            InputValue::Scalar(ref s) => s.serialize(serializer),
            InputValue::Enum(ref v) => serializer.serialize_str(v),
            InputValue::List(ref v) => v
                .iter()
                .map(|x| x.item.clone())
                .collect::<Vec<_>>()
                .serialize(serializer),
            InputValue::Object(ref v) => v
                .iter()
                .map(|&(ref k, ref v)| (k.item.clone(), v.item.clone()))
                .collect::<IndexMap<_, _>>()
                .serialize(serializer),
        }
    }
}

impl ser::Serialize for RuleError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        let mut map = serializer.serialize_map(Some(2))?;

        map.serialize_key("message")?;
        map.serialize_value(self.message())?;

        map.serialize_key("locations")?;
        map.serialize_value(self.locations())?;

        map.end()
    }
}

impl ser::Serialize for SourcePosition {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        let mut map = serializer.serialize_map(Some(2))?;

        let line = self.line() + 1;
        map.serialize_key("line")?;
        map.serialize_value(&line)?;

        let column = self.column() + 1;
        map.serialize_key("column")?;
        map.serialize_value(&column)?;

        map.end()
    }
}

impl<'a> ser::Serialize for Spanning<ParseError<'a>> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        let mut map = serializer.serialize_map(Some(2))?;

        let message = format!("{}", self.item);
        map.serialize_key("message")?;
        map.serialize_value(&message)?;

        let mut location = IndexMap::new();
        location.insert("line".to_owned(), self.start.line() + 1);
        location.insert("column".to_owned(), self.start.column() + 1);

        let locations = vec![location];

        map.serialize_key("locations")?;
        map.serialize_value(&locations)?;

        map.end()
    }
}

impl<T> ser::Serialize for Object<T>
where
    T: ser::Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.field_count()))?;

        for &(ref f, ref v) in self.iter() {
            map.serialize_key(f)?;
            map.serialize_value(v)?;
        }

        map.end()
    }
}

impl<T> ser::Serialize for Value<T>
where
    T: ser::Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        match *self {
            Value::Null => serializer.serialize_unit(),
            Value::Scalar(ref s) => s.serialize(serializer),
            Value::List(ref v) => v.serialize(serializer),
            Value::Object(ref v) => v.serialize(serializer),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ExecutionError, GraphQLError};
    use ast::InputValue;
    use executor::ExecutionError;
    use serde_json::from_str;
    use serde_json::to_string;
    use {FieldError, Value};
    use value::{DefaultScalarValue, Object};

    #[test]
    fn int() {
        assert_eq!(
            from_str::<InputValue<DefaultScalarValue>>("1235").unwrap(),
            InputValue::scalar(1235)
        );
    }

    #[test]
    fn float() {
        assert_eq!(
            from_str::<InputValue<DefaultScalarValue>>("2.0").unwrap(),
            InputValue::scalar(2.0)
        );
        // large value without a decimal part is also float
        assert_eq!(
            from_str::<InputValue<DefaultScalarValue>>("123567890123").unwrap(),
            InputValue::scalar(123567890123.0)
        );
    }

    #[test]
    fn errors() {
        assert_eq!(
            to_string(&GraphQLError::UnknownOperationName).unwrap(),
            r#"[{"message":"Unknown operation"}]"#
        );
    }

    #[test]
    fn error_extensions() {
        let mut obj = Object::with_capacity(1);
        obj.add_field("foo".to_string(), Value::String("bar".to_string()));
        assert_eq!(
            to_string(&ExecutionError::at_origin(FieldError::new(
                "foo error",
                Value::Object(obj),
            ))).unwrap(),
            r#"{"message":"foo error","locations":[{"line":1,"column":1}],"path":[],"extensions":{"foo":"bar"}}"#
        );
    }
}
