use crate::{executor::Registry, schema::meta::MetaType, DefaultScalarValue, ScalarValue};

pub trait TypeName<Info: ?Sized> {
    fn type_name(info: &Info) -> &str;
}

pub trait Type<Info: ?Sized, S = DefaultScalarValue> {
    fn meta<'r>(info: &Info, registry: &mut Registry<'r, S>) -> MetaType<'r, S>
    where
        S: 'r;
}

