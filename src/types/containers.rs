use ast::{InputValue, ToInputValue, FromInputValue, Selection};
use value::Value;
use schema::meta::MetaType;

use executor::{Executor, Registry};
use types::base::{GraphQLType};

impl<T, CtxT> GraphQLType for Option<T> where T: GraphQLType<Context=CtxT> {
    type Context = CtxT;

    fn name() -> Option<&'static str> {
        None
    }

    fn meta<'r>(registry: &mut Registry<'r>) -> MetaType<'r> {
        registry.build_nullable_type::<T>().into_meta()
    }

    fn resolve(&self, _: Option<&[Selection]>, executor: &Executor<CtxT>) -> Value {
        match *self {
            Some(ref obj) => executor.resolve_into_value(obj),
            None => Value::null(),
        }
    }
}

impl<T> FromInputValue for Option<T> where T: FromInputValue {
    fn from(v: &InputValue) -> Option<Option<T>> {
        match v {
            &InputValue::Null => Some(None),
            v => match v.convert() {
                Some(x) => Some(Some(x)),
                None => None,
            }
        }
    }
}

impl<T> ToInputValue for Option<T> where T: ToInputValue {
    fn to(&self) -> InputValue {
        match *self {
            Some(ref v) => v.to(),
            None => InputValue::null(),
        }
    }
}

impl<T, CtxT> GraphQLType for Vec<T> where T: GraphQLType<Context=CtxT> {
    type Context = CtxT;

    fn name() -> Option<&'static str> {
        None
    }

    fn meta<'r>(registry: &mut Registry<'r>) -> MetaType<'r> {
        registry.build_list_type::<T>().into_meta()
    }

    fn resolve(&self, _: Option<&[Selection]>, executor: &Executor<CtxT>) -> Value {
        Value::list(
            self.iter().map(|e| executor.resolve_into_value(e)).collect()
        )
    }
}

impl<T> FromInputValue for Vec<T> where T: FromInputValue {
    fn from(v: &InputValue) -> Option<Vec<T>> {
        match *v {
            InputValue::List(ref ls) => {
                let v: Vec<_> = ls.iter().filter_map(|i| i.item.convert()).collect();

                if v.len() == ls.len() {
                    Some(v)
                }
                else {
                    None
                }
            },
            ref other =>
                if let Some(e) = other.convert() {
                    Some(vec![ e ])
                } else {
                    None
                }
        }
    }
}

impl<T> ToInputValue for Vec<T> where T: ToInputValue {
    fn to(&self) -> InputValue {
        InputValue::list(self.iter().map(|v| v.to()).collect())
    }
}

impl<'a, T, CtxT> GraphQLType for &'a [T] where T: GraphQLType<Context=CtxT> {
    type Context = CtxT;

    fn name() -> Option<&'static str> {
        None
    }

    fn meta<'r>(registry: &mut Registry<'r>) -> MetaType<'r> {
        registry.build_list_type::<T>().into_meta()
    }

    fn resolve(&self, _: Option<&[Selection]>, executor: &Executor<CtxT>) -> Value {
        Value::list(
            self.iter().map(|e| executor.resolve_into_value(e)).collect()
        )
    }
}

impl<'a, T> ToInputValue for &'a [T] where T: ToInputValue {
    fn to(&self) -> InputValue {
        InputValue::list(self.iter().map(|v| v.to()).collect())
    }
}
