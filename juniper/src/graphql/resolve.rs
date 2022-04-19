use crate::{
    executor::{ExecutionResult, Executor, Registry},
    resolve,
    schema::meta::MetaType,
    Arguments, DefaultScalarValue,
};

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

pub trait Field<S = DefaultScalarValue> {
    fn resolve_field<Info: ?Sized, Ctx: ?Sized>(
        &self,
        info: &Info,
        field_name: &str,
        arguments: &Arguments<S>,
        executor: &Executor<Ctx, S>,
    ) -> ExecutionResult<S>
    where
        Self: resolve::Field<Info, Ctx, S>;
}

impl<T: ?Sized, S> Field<S> for T {
    fn resolve_field<Info: ?Sized, Ctx: ?Sized>(
        &self,
        info: &Info,
        field_name: &str,
        arguments: &Arguments<S>,
        executor: &Executor<Ctx, S>,
    ) -> ExecutionResult<S>
    where
        Self: resolve::Field<Info, Ctx, S>,
    {
        <Self as resolve::Field<Info, Ctx, S>>::resolve_field(
            self, info, field_name, arguments, executor,
        )
    }
}

pub trait ConcreteTypeName {
    fn concrete_type_name<'i, Info: ?Sized>(&self, info: &'i Info) -> &'i str
    where
        Self: resolve::ConcreteTypeName<Info>;
}

impl<T: ?Sized> ConcreteTypeName for T {
    fn concrete_type_name<'i, Info: ?Sized>(&self, info: &'i Info) -> &'i str
    where
        Self: resolve::ConcreteTypeName<Info>,
    {
        <Self as resolve::ConcreteTypeName<Info>>::concrete_type_name(self, info)
    }
}
