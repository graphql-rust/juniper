use ast::{InputValue, Selection, FromInputValue, ToInputValue};
use value::Value;

use schema::meta::MetaType;

use executor::{Executor, Registry, FieldResult, IntoFieldResult};
use types::base::GraphQLType;

/// An ID as defined by the GraphQL specification
///
/// Represented as a string, but can be converted _to_ from an integer as well.
pub struct ID(String);

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


impl<'a, CtxT> GraphQLType<CtxT> for &'a str {
    fn name() -> Option<&'static str> {
        Some("String")
    }

    fn meta(registry: &mut Registry<CtxT>) -> MetaType {
        registry.build_scalar_type::<String>().into_meta()
    }

    fn resolve(&self, _: Option<Vec<Selection>>, _: &Executor<CtxT>) -> Value {
        Value::string(self)
    }
}

impl<'a> ToInputValue for &'a str {
    fn to(&self) -> InputValue {
        InputValue::string(self)
    }
}

impl<'a> IntoFieldResult<&'a str> for &'a str {
    fn into(self) -> FieldResult<&'a str> {
        Ok(self)
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


impl<CtxT> GraphQLType<CtxT> for () {
    fn name() -> Option<&'static str> {
        Some("__Unit")
    }

    fn meta(registry: &mut Registry<CtxT>) -> MetaType {
        registry.build_scalar_type::<Self>().into_meta()
    }
}

impl FromInputValue for () {
    fn from(_: &InputValue) -> Option<()> {
        None
    }
}

impl IntoFieldResult<()> for () {
    fn into(self) -> FieldResult<()> {
        Ok(self)
    }
}
