use crate::{
    behavior, graphql,
    meta::MetaType,
    parser::{self, ParseError},
    reflect, Arguments, BoxFuture, ExecutionResult, Executor, IntoFieldError, Registry, Selection,
};

pub trait Type<TypeInfo: ?Sized, ScalarValue, Behavior: ?Sized = behavior::Standard> {
    fn meta<'r>(
        registry: &mut Registry<'r, ScalarValue>,
        type_info: &TypeInfo,
    ) -> MetaType<'r, ScalarValue>
    where
        ScalarValue: 'r; // TODO: remove?
}

pub trait TypeName<TypeInfo: ?Sized, Behavior: ?Sized = behavior::Standard> {
    fn type_name(type_info: &TypeInfo) -> &str;
}

pub trait ConcreteTypeName<TypeInfo: ?Sized, Behavior: ?Sized = behavior::Standard> {
    fn concrete_type_name<'i>(&self, type_info: &'i TypeInfo) -> &'i str;
}

pub trait Value<
    TypeInfo: ?Sized,
    Context: ?Sized,
    ScalarValue,
    Behavior: ?Sized = behavior::Standard,
>
{
    fn resolve_value(
        &self,
        selection_set: Option<&[Selection<'_, ScalarValue>]>,
        type_info: &TypeInfo,
        executor: &Executor<Context, ScalarValue>,
    ) -> ExecutionResult<ScalarValue>;
}

pub trait ValueAsync<
    TypeInfo: ?Sized,
    Context: ?Sized,
    ScalarValue,
    Behavior: ?Sized = behavior::Standard,
>
{
    fn resolve_value_async<'r>(
        &'r self,
        selection_set: Option<&'r [Selection<'_, ScalarValue>]>,
        type_info: &'r TypeInfo,
        executor: &'r Executor<Context, ScalarValue>,
    ) -> BoxFuture<'r, ExecutionResult<ScalarValue>>;
}

pub trait ConcreteValue<
    TypeInfo: ?Sized,
    Context: ?Sized,
    ScalarValue,
    Behavior: ?Sized = behavior::Standard,
>
{
    fn resolve_concrete_value(
        &self,
        type_name: &str,
        selection_set: Option<&[Selection<'_, ScalarValue>]>,
        type_info: &TypeInfo,
        executor: &Executor<Context, ScalarValue>,
    ) -> ExecutionResult<ScalarValue>;
}

pub trait ConcreteValueAsync<
    TypeInfo: ?Sized,
    Context: ?Sized,
    ScalarValue,
    Behavior: ?Sized = behavior::Standard,
>
{
    fn resolve_concrete_value_async<'r>(
        &'r self,
        type_name: &str,
        selection_set: Option<&'r [Selection<'_, ScalarValue>]>,
        type_info: &'r TypeInfo,
        executor: &'r Executor<Context, ScalarValue>,
    ) -> BoxFuture<'r, ExecutionResult<ScalarValue>>;
}

pub trait Field<
    TypeInfo: ?Sized,
    Context: ?Sized,
    ScalarValue,
    Behavior: ?Sized = behavior::Standard,
>
{
    fn resolve_field(
        &self,
        field_name: &str,
        arguments: &Arguments<ScalarValue>,
        type_info: &TypeInfo,
        executor: &Executor<Context, ScalarValue>,
    ) -> ExecutionResult<ScalarValue>;
}

pub trait StaticField<
    const N: reflect::FieldName,
    TypeInfo: ?Sized,
    Context: ?Sized,
    ScalarValue,
    Behavior: ?Sized = behavior::Standard,
>
{
    fn resolve_static_field(
        &self,
        arguments: &Arguments<ScalarValue>,
        type_info: &TypeInfo,
        executor: &Executor<Context, ScalarValue>,
    ) -> ExecutionResult<ScalarValue>;
}

pub trait FieldAsync<
    TypeInfo: ?Sized,
    Context: ?Sized,
    ScalarValue,
    Behavior: ?Sized = behavior::Standard,
>
{
    fn resolve_field_async<'r>(
        &'r self,
        field_name: &'r str,
        arguments: &'r Arguments<ScalarValue>,
        type_info: &'r TypeInfo,
        executor: &'r Executor<Context, ScalarValue>,
    ) -> BoxFuture<'r, ExecutionResult<ScalarValue>>;
}

pub trait StaticFieldAsync<
    const N: reflect::FieldName,
    TypeInfo: ?Sized,
    Context: ?Sized,
    ScalarValue,
    Behavior: ?Sized = behavior::Standard,
>
{
    fn resolve_static_field_async<'r>(
        &'r self,
        arguments: &'r Arguments<ScalarValue>,
        type_info: &'r TypeInfo,
        executor: &'r Executor<Context, ScalarValue>,
    ) -> BoxFuture<'r, ExecutionResult<ScalarValue>>;
}

pub trait ToInputValue<ScalarValue, Behavior: ?Sized = behavior::Standard> {
    fn to_input_value(&self) -> graphql::InputValue<ScalarValue>;
}

pub trait InputValue<'input, ScalarValue: 'input, Behavior: ?Sized = behavior::Standard>:
    Sized
{
    type Error: IntoFieldError<ScalarValue>;

    fn try_from_input_value(
        v: &'input graphql::InputValue<ScalarValue>,
    ) -> Result<Self, Self::Error>;

    fn try_from_implicit_null() -> Result<Self, Self::Error> {
        Self::try_from_input_value(&graphql::InputValue::<ScalarValue>::Null)
    }
}

pub trait InputValueOwned<ScalarValue, Behavior: ?Sized = behavior::Standard>:
    for<'i> InputValue<'i, ScalarValue, Behavior>
{
}

impl<T, SV, BH: ?Sized> InputValueOwned<SV, BH> for T where T: for<'i> InputValue<'i, SV, BH> {}

pub trait InputValueAs<'input, Wrapper, ScalarValue: 'input, Behavior: ?Sized = behavior::Standard>
{
    type Error: IntoFieldError<ScalarValue>;

    fn try_from_input_value(
        v: &'input graphql::InputValue<ScalarValue>,
    ) -> Result<Wrapper, Self::Error>;

    fn try_from_implicit_null() -> Result<Wrapper, Self::Error> {
        Self::try_from_input_value(&graphql::InputValue::<ScalarValue>::Null)
    }
}

pub trait InputValueAsRef<ScalarValue, Behavior: ?Sized = behavior::Standard> {
    type Error: IntoFieldError<ScalarValue>;

    fn try_from_input_value(v: &graphql::InputValue<ScalarValue>) -> Result<&Self, Self::Error>;

    fn try_from_implicit_null<'a>() -> Result<&'a Self, Self::Error>
    where
        ScalarValue: 'a,
    {
        Self::try_from_input_value(&graphql::InputValue::<ScalarValue>::Null)
    }
}

pub trait ScalarToken<ScalarValue, Behavior: ?Sized = behavior::Standard> {
    fn parse_scalar_token(token: parser::ScalarToken<'_>) -> Result<ScalarValue, ParseError<'_>>;
}
