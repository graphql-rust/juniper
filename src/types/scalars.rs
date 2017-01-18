use std::convert::From;
use std::marker::PhantomData;
use std::ops::Deref;

use ast::{InputValue, Selection, FromInputValue, ToInputValue};
use value::Value;

use schema::meta::MetaType;

use executor::{Executor, Registry};
use types::base::GraphQLType;

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

graphql_scalar!(ID as "ID" {
    resolve(&self) -> Value {
        Value::string(&self.0)
    }

    from_input_value(v: &InputValue) -> Option<ID> {
        match *v {
            InputValue::String(ref s) => Some(ID(s.to_owned())),
            InputValue::Int(i) => Some(ID(format!("{}", i))),
            _ => None
        }
    }
});


graphql_scalar!(String as "String" {
    resolve(&self) -> Value {
        Value::string(self)
    }

    from_input_value(v: &InputValue) -> Option<String> {
        match *v {
            InputValue::String(ref s) => Some(s.clone()),
            _ => None,
        }
    }
});


impl<'a> GraphQLType for &'a str {
    type Context = ();

    fn name() -> Option<&'static str> {
        Some("String")
    }

    fn meta<'r>(registry: &mut Registry<'r>) -> MetaType<'r> {
        registry.build_scalar_type::<String>().into_meta()
    }

    fn resolve(&self, _: Option<&[Selection]>, _: &Executor<Self::Context>) -> Value {
        Value::string(self)
    }
}

impl<'a> ToInputValue for &'a str {
    fn to(&self) -> InputValue {
        InputValue::string(self)
    }
}



graphql_scalar!(bool as "Boolean" {
    resolve(&self) -> Value {
        Value::boolean(*self)
    }

    from_input_value(v: &InputValue) -> Option<bool> {
        match *v {
            InputValue::Boolean(b) => Some(b),
            _ => None,
        }
    }
});


graphql_scalar!(i64 as "Int" {
    resolve(&self) -> Value {
        Value::int(*self)
    }

    from_input_value(v: &InputValue) -> Option<i64> {
        match *v {
            InputValue::Int(i) => Some(i),
            _ => None,
        }
    }
});


graphql_scalar!(f64 as "Float" {
    resolve(&self) -> Value {
        Value::float(*self)
    }

    from_input_value(v: &InputValue) -> Option<f64> {
        match *v {
            InputValue::Int(i) => Some(i as f64),
            InputValue::Float(f) => Some(f),
            _ => None,
        }
    }
});


impl GraphQLType for () {
    type Context = ();

    fn name() -> Option<&'static str> {
        Some("__Unit")
    }

    fn meta<'r>(registry: &mut Registry<'r>) -> MetaType<'r> {
        registry.build_scalar_type::<Self>().into_meta()
    }
}

impl FromInputValue for () {
    fn from(_: &InputValue) -> Option<()> {
        None
    }
}


/// Utility type to define read-only schemas
///
/// If you instantiate `RootNode` with this as the mutation, no mutation will be
/// generated for the schema.
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

impl<T> GraphQLType for EmptyMutation<T> {
    type Context = T;

    fn name() -> Option<&'static str> {
        Some("__EmptyMutation")
    }

    fn meta<'r>(registry: &mut Registry<'r>) -> MetaType<'r> {
        registry.build_object_type::<Self>(&[]).into_meta()
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
