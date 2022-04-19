use crate::{
    executor::{ExecutionResult, Executor, Registry},
    schema::meta::MetaType,
    Arguments, DefaultScalarValue,
};

pub trait TypeName<Info: ?Sized> {
    fn type_name(info: &Info) -> &str;
}

pub trait Type<Info: ?Sized, S = DefaultScalarValue> {
    fn meta<'r>(info: &Info, registry: &mut Registry<'r, S>) -> MetaType<'r, S>
    where
        S: 'r;
}

pub trait Field<Info: ?Sized, Ctx: ?Sized, S = DefaultScalarValue> {
    fn resolve_field(
        &self,
        info: &Info,
        field_name: &str,
        arguments: &Arguments<S>,
        executor: &Executor<Ctx, S>,
    ) -> ExecutionResult<S>;
}

pub trait ConcreteTypeName<Info: ?Sized> {
    fn concrete_type_name<'i>(&self, info: &'i Info) -> &'i str;
}
