use crate::{
    meta::MetaType, resolve, Arguments, BoxFuture, DefaultScalarValue, ExecutionResult, Executor,
    Registry, Selection,
};

pub trait Type<S = DefaultScalarValue> {
    fn meta<'r, Info: ?Sized>(registry: &mut Registry<'r, S>, info: &Info) -> MetaType<'r, S>
    where
        S: 'r,
        Self: resolve::Type<Info, S>;
}

impl<T: ?Sized, S> Type<S> for T {
    fn meta<'r, Info: ?Sized>(registry: &mut Registry<'r, S>, info: &Info) -> MetaType<'r, S>
    where
        S: 'r,
        Self: resolve::Type<Info, S>,
    {
        <Self as resolve::Type<Info, S>>::meta(registry, info)
    }
}

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

pub trait Value<S = DefaultScalarValue> {
    fn resolve_value<Info: ?Sized, Ctx: ?Sized>(
        &self,
        selection_set: Option<&[Selection<'_, S>]>,
        info: &Info,
        executor: &Executor<Ctx, S>,
    ) -> ExecutionResult<S>
    where
        Self: resolve::Value<Info, Ctx, S>;
}

impl<T: ?Sized, S> Value<S> for T {
    fn resolve_value<Info: ?Sized, Ctx: ?Sized>(
        &self,
        selection_set: Option<&[Selection<'_, S>]>,
        info: &Info,
        executor: &Executor<Ctx, S>,
    ) -> ExecutionResult<S>
    where
        Self: resolve::Value<Info, Ctx, S>,
    {
        <Self as resolve::Value<Info, Ctx, S>>::resolve_value(self, selection_set, info, executor)
    }
}

pub trait ValueAsync<S = DefaultScalarValue> {
    fn resolve_value_async<'r, Info: ?Sized, Ctx: ?Sized>(
        &'r self,
        selection_set: Option<&'r [Selection<'_, S>]>,
        info: &'r Info,
        executor: &'r Executor<Ctx, S>,
    ) -> BoxFuture<'r, ExecutionResult<S>>
    where
        Self: resolve::ValueAsync<Info, Ctx, S>;
}

impl<T: ?Sized, S> ValueAsync<S> for T {
    fn resolve_value_async<'r, Info: ?Sized, Ctx: ?Sized>(
        &'r self,
        selection_set: Option<&'r [Selection<'_, S>]>,
        info: &'r Info,
        executor: &'r Executor<Ctx, S>,
    ) -> BoxFuture<'r, ExecutionResult<S>>
    where
        Self: resolve::ValueAsync<Info, Ctx, S>,
    {
        <Self as resolve::ValueAsync<Info, Ctx, S>>::resolve_value_async(
            self,
            selection_set,
            info,
            executor,
        )
    }
}

pub trait ConcreteValue<S = DefaultScalarValue> {
    fn resolve_concrete_value<Info: ?Sized, Ctx: ?Sized>(
        &self,
        type_name: &str,
        selection_set: Option<&[Selection<'_, S>]>,
        info: &Info,
        executor: &Executor<Ctx, S>,
    ) -> ExecutionResult<S>
    where
        Self: resolve::ConcreteValue<Info, Ctx, S>;
}

impl<T: ?Sized, S> ConcreteValue<S> for T {
    fn resolve_concrete_value<Info: ?Sized, Ctx: ?Sized>(
        &self,
        type_name: &str,
        selection_set: Option<&[Selection<'_, S>]>,
        info: &Info,
        executor: &Executor<Ctx, S>,
    ) -> ExecutionResult<S>
    where
        Self: resolve::ConcreteValue<Info, Ctx, S>,
    {
        <Self as resolve::ConcreteValue<Info, Ctx, S>>::resolve_concrete_value(
            self,
            type_name,
            selection_set,
            info,
            executor,
        )
    }
}

pub trait ConcreteValueAsync<S = DefaultScalarValue> {
    fn resolve_concrete_value_async<'r, Info: ?Sized, Ctx: ?Sized>(
        &'r self,
        type_name: &str,
        selection_set: Option<&'r [Selection<'_, S>]>,
        info: &'r Info,
        executor: &'r Executor<Ctx, S>,
    ) -> BoxFuture<'r, ExecutionResult<S>>
    where
        Self: resolve::ConcreteValueAsync<Info, Ctx, S>;
}

impl<T: ?Sized, S> ConcreteValueAsync<S> for T {
    fn resolve_concrete_value_async<'r, Info: ?Sized, Ctx: ?Sized>(
        &'r self,
        type_name: &str,
        selection_set: Option<&'r [Selection<'_, S>]>,
        info: &'r Info,
        executor: &'r Executor<Ctx, S>,
    ) -> BoxFuture<'r, ExecutionResult<S>>
    where
        Self: resolve::ConcreteValueAsync<Info, Ctx, S>,
    {
        <Self as resolve::ConcreteValueAsync<Info, Ctx, S>>::resolve_concrete_value_async(
            self,
            type_name,
            selection_set,
            info,
            executor,
        )
    }
}

pub trait Field<S = DefaultScalarValue> {
    fn resolve_field<Info: ?Sized, Ctx: ?Sized>(
        &self,
        field_name: &str,
        arguments: &Arguments<S>,
        info: &Info,
        executor: &Executor<Ctx, S>,
    ) -> ExecutionResult<S>
    where
        Self: resolve::Field<Info, Ctx, S>;
}

impl<T: ?Sized, S> Field<S> for T {
    fn resolve_field<Info: ?Sized, Ctx: ?Sized>(
        &self,
        field_name: &str,
        arguments: &Arguments<S>,
        info: &Info,
        executor: &Executor<Ctx, S>,
    ) -> ExecutionResult<S>
    where
        Self: resolve::Field<Info, Ctx, S>,
    {
        <Self as resolve::Field<Info, Ctx, S>>::resolve_field(
            self, field_name, arguments, info, executor,
        )
    }
}

pub trait FieldAsync<S = DefaultScalarValue> {
    fn resolve_field_async<'r, Info: ?Sized, Ctx: ?Sized>(
        &'r self,
        field_name: &'r str,
        arguments: &'r Arguments<S>,
        info: &'r Info,
        executor: &'r Executor<Ctx, S>,
    ) -> BoxFuture<'r, ExecutionResult<S>>
    where
        Self: resolve::FieldAsync<Info, Ctx, S>;
}

impl<T: ?Sized, S> FieldAsync<S> for T {
    fn resolve_field_async<'r, Info: ?Sized, Ctx: ?Sized>(
        &'r self,
        field_name: &'r str,
        arguments: &'r Arguments<S>,
        info: &'r Info,
        executor: &'r Executor<Ctx, S>,
    ) -> BoxFuture<'r, ExecutionResult<S>>
    where
        Self: resolve::FieldAsync<Info, Ctx, S>,
    {
        <Self as resolve::FieldAsync<Info, Ctx, S>>::resolve_field_async(
            self, field_name, arguments, info, executor,
        )
    }
}
