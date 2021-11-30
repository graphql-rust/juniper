mod object;
mod scalar;

use std::{any::TypeId, borrow::Cow, fmt, mem};

use crate::{
    ast::{InputValue, ToInputValue},
    parser::Spanning,
};

pub use self::{
    object::Object,
    scalar::{DefaultScalarValue, ParseScalarResult, ParseScalarValue, ScalarValue},
};

/// Serializable value returned from query and field execution.
///
/// Used by the execution engine and resolvers to build up the response
/// structure. Similar to the `Json` type found in the serialize crate.
///
/// It is also similar to the `InputValue` type, but can not contain enum
/// values or variables. Also, lists and objects do not contain any location
/// information since they are generated by resolving fields and values rather
/// than parsing a source query.
#[derive(Debug, PartialEq, Clone)]
#[allow(missing_docs)]
pub enum Value<S = DefaultScalarValue> {
    Null,
    Scalar(S),
    List(Vec<Value<S>>),
    Object(Object<S>),
}

impl<S> Value<S> {
    // CONSTRUCTORS

    /// Construct a null value.
    pub fn null() -> Self {
        Self::Null
    }

    /// Construct a list value.
    pub fn list(l: Vec<Self>) -> Self {
        Self::List(l)
    }

    /// Construct an object value.
    pub fn object(o: Object<S>) -> Self {
        Self::Object(o)
    }

    /// Construct a scalar value
    pub fn scalar<T>(s: T) -> Self
    where
        S: From<T>,
    {
        Self::Scalar(s.into())
    }

    // DISCRIMINATORS

    /// Does this value represent null?
    pub fn is_null(&self) -> bool {
        matches!(*self, Self::Null)
    }

    /// View the underlying scalar value if present
    pub fn as_scalar_value<'a, T>(&'a self) -> Option<&'a T>
    where
        Option<&'a T>: From<&'a S>,
    {
        match self {
            Self::Scalar(s) => s.into(),
            _ => None,
        }
    }

    /// View the underlying float value, if present.
    pub fn as_float_value(&self) -> Option<f64>
    where
        S: ScalarValue,
    {
        match self {
            Self::Scalar(s) => s.as_float(),
            _ => None,
        }
    }

    /// View the underlying object value, if present.
    pub fn as_object_value(&self) -> Option<&Object<S>> {
        match self {
            Self::Object(o) => Some(o),
            _ => None,
        }
    }

    /// Convert this value into an Object.
    ///
    /// Returns None if value is not an Object.
    pub fn into_object(self) -> Option<Object<S>> {
        match self {
            Self::Object(o) => Some(o),
            _ => None,
        }
    }

    /// Mutable view into the underlying object value, if present.
    pub fn as_mut_object_value(&mut self) -> Option<&mut Object<S>> {
        match self {
            Self::Object(o) => Some(o),
            _ => None,
        }
    }

    /// View the underlying list value, if present.
    pub fn as_list_value(&self) -> Option<&Vec<Self>> {
        match self {
            Self::List(l) => Some(l),
            _ => None,
        }
    }

    /// View the underlying scalar value, if present
    pub fn as_scalar(&self) -> Option<&S> {
        match self {
            Self::Scalar(s) => Some(s),
            _ => None,
        }
    }

    /// View the underlying string value, if present.
    pub fn as_string_value<'a>(&'a self) -> Option<&'a str>
    where
        Option<&'a String>: From<&'a S>,
    {
        self.as_scalar_value::<String>().map(String::as_str)
    }

    /// Maps the [`ScalarValue`] type of this [`Value`] into the specified one.
    pub fn map_scalar_value<Into>(self) -> Value<Into>
    where
        S: ScalarValue,
        Into: ScalarValue,
    {
        if TypeId::of::<Into>() == TypeId::of::<S>() {
            // SAFETY: This is safe, because we're transmuting the value into
            //         itself, so no invariants may change and we're just
            //         satisfying the type checker.
            //         As `mem::transmute_copy` creates a copy of data, we need
            //         `mem::ManuallyDrop` here to omit double-free when
            //         `S: Drop`.
            let val = mem::ManuallyDrop::new(self);
            unsafe { mem::transmute_copy(&*val) }
        } else {
            match self {
                Self::Null => Value::Null,
                Self::Scalar(s) => Value::Scalar(s.into_another()),
                Self::List(l) => Value::List(l.into_iter().map(Value::map_scalar_value).collect()),
                Self::Object(o) => Value::Object(
                    o.into_iter()
                        .map(|(k, v)| (k, v.map_scalar_value()))
                        .collect(),
                ),
            }
        }
    }
}

impl<S: Clone> ToInputValue<S> for Value<S> {
    fn to_input_value(&self) -> InputValue<S> {
        match self {
            Self::Null => InputValue::Null,
            Self::Scalar(s) => InputValue::Scalar(s.clone()),
            Self::List(l) => InputValue::List(
                l.iter()
                    .map(|x| Spanning::unlocated(x.to_input_value()))
                    .collect(),
            ),
            Self::Object(o) => InputValue::Object(
                o.iter()
                    .map(|(k, v)| {
                        (
                            Spanning::unlocated(k.clone()),
                            Spanning::unlocated(v.to_input_value()),
                        )
                    })
                    .collect(),
            ),
        }
    }
}

impl<S: ScalarValue> fmt::Display for Value<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Null => write!(f, "null"),
            Self::Scalar(s) => {
                if let Some(string) = s.as_string() {
                    write!(f, "\"{}\"", string)
                } else {
                    write!(f, "{}", s)
                }
            }
            Self::List(list) => {
                write!(f, "[")?;
                for (idx, item) in list.iter().enumerate() {
                    write!(f, "{}", item)?;
                    if idx < list.len() - 1 {
                        write!(f, ", ")?;
                    }
                }
                write!(f, "]")?;

                Ok(())
            }
            Self::Object(obj) => {
                write!(f, "{{")?;
                for (idx, (key, value)) in obj.iter().enumerate() {
                    write!(f, "\"{}\": {}", key, value)?;

                    if idx < obj.field_count() - 1 {
                        write!(f, ", ")?;
                    }
                }
                write!(f, "}}")?;

                Ok(())
            }
        }
    }
}

impl<S, T> From<Option<T>> for Value<S>
where
    Self: From<T>,
{
    fn from(v: Option<T>) -> Self {
        match v {
            Some(v) => v.into(),
            None => Self::Null,
        }
    }
}

impl<'a, S: From<String>> From<&'a str> for Value<S> {
    fn from(s: &'a str) -> Self {
        Self::scalar(s.to_owned())
    }
}

impl<'a, S: From<String>> From<Cow<'a, str>> for Value<S> {
    fn from(s: Cow<'a, str>) -> Self {
        Self::scalar(s.into_owned())
    }
}

impl<S: From<String>> From<String> for Value<S> {
    fn from(s: String) -> Self {
        Self::scalar(s)
    }
}

impl<S: From<i32>> From<i32> for Value<S> {
    fn from(i: i32) -> Self {
        Self::scalar(i)
    }
}

impl<S: From<f64>> From<f64> for Value<S> {
    fn from(f: f64) -> Self {
        Self::scalar(f)
    }
}

impl<S: From<bool>> From<bool> for Value<S> {
    fn from(b: bool) -> Self {
        Self::scalar(b)
    }
}

#[cfg(test)]
mod tests {
    use crate::graphql_value;

    use super::Value;

    #[test]
    fn display_null() {
        let s: Value = graphql_value!(null);
        assert_eq!("null", format!("{}", s));
    }

    #[test]
    fn display_int() {
        let s: Value = graphql_value!(123);
        assert_eq!("123", format!("{}", s));
    }

    #[test]
    fn display_float() {
        let s: Value = graphql_value!(123.456);
        assert_eq!("123.456", format!("{}", s));
    }

    #[test]
    fn display_string() {
        let s: Value = graphql_value!("foo");
        assert_eq!("\"foo\"", format!("{}", s));
    }

    #[test]
    fn display_bool() {
        let s: Value = graphql_value!(false);
        assert_eq!("false", format!("{}", s));

        let s: Value = graphql_value!(true);
        assert_eq!("true", format!("{}", s));
    }

    #[test]
    fn display_list() {
        let s: Value = graphql_value!([1, null, "foo"]);
        assert_eq!("[1, null, \"foo\"]", format!("{}", s));
    }

    #[test]
    fn display_list_one_element() {
        let s: Value = graphql_value!([1]);
        assert_eq!("[1]", format!("{}", s));
    }

    #[test]
    fn display_list_empty() {
        let s: Value = graphql_value!([]);
        assert_eq!("[]", format!("{}", s));
    }

    #[test]
    fn display_object() {
        let s: Value = graphql_value!({
            "int": 1,
            "null": null,
            "string": "foo",
        });
        assert_eq!(
            r#"{"int": 1, "null": null, "string": "foo"}"#,
            format!("{}", s)
        );
    }

    #[test]
    fn display_object_one_field() {
        let s: Value = graphql_value!({
            "int": 1,
        });
        assert_eq!(r#"{"int": 1}"#, format!("{}", s));
    }

    #[test]
    fn display_object_empty() {
        let s: Value = graphql_value!({});
        assert_eq!(r#"{}"#, format!("{}", s));
    }
}
