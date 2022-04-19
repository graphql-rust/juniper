use crate::{executor::Registry, resolve, schema::meta::MetaType, DefaultScalarValue};

pub trait TypeName {
    fn type_name<Info: ?Sized>(info: &Info) -> &str
    where
        Self: resolve::TypeName<Info>;
}

impl<T: ?Sized> TypeName for T {
    fn type_name<Info: ?Sized>(info: &Info) -> &str
    where
        Self: resolve::TypeName<Info>,
    {
        <Self as resolve::TypeName<Info>>::type_name(info)
    }
}

pub trait Type<S = DefaultScalarValue> {
    fn meta<'r, Info: ?Sized>(info: &Info, registry: &mut Registry<'r, S>) -> MetaType<'r, S>
    where
        S: 'r,
        Self: resolve::Type<Info, S>;
}

impl<T: ?Sized, S> Type<S> for T {
    fn meta<'r, Info: ?Sized>(info: &Info, registry: &mut Registry<'r, S>) -> MetaType<'r, S>
    where
        S: 'r,
        Self: resolve::Type<Info, S>,
    {
        <Self as resolve::Type<Info, S>>::meta(info, registry)
    }
}
