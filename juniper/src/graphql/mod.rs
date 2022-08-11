use crate::{behavior, resolve};

pub use crate::{
    ast::InputValue,
    executor::Variables,
    macros::{input_value, value, vars},
    resolve::Type,
    value::Value,
    GraphQLEnum as Enum, GraphQLScalar as Scalar,
};

pub trait Enum<
    'inp,
    TypeInfo: ?Sized,
    Context: ?Sized,
    ScalarValue: 'inp,
    Behavior: ?Sized = behavior::Standard,
>:
    InputType<'inp, TypeInfo, ScalarValue, Behavior>
    + OutputType<TypeInfo, Context, ScalarValue, Behavior>
{
    fn assert_enum();
}

/*
pub trait Interface<S>: OutputType<S>
   + resolve::TypeName
   + resolve::ConcreteTypeName
   + resolve::Value<S>
   + resolve::ValueAsync<S>
   + resolve::ConcreteValue<S>
   + resolve::ConcreteValueAsync<S>
   + resolve::Field<S>
   + resolve::FieldAsync<S>
{
    fn assert_interface();
}

pub trait Object<S>: OutputType<S>
   + resolve::TypeName
   + resolve::ConcreteTypeName
   + resolve::Value<S>
   + resolve::ValueAsync<S>
   + resolve::Field<S>
   + resolve::FieldAsync<S>
{
    fn assert_object();
}*/

pub trait InputObject<
    'inp,
    TypeInfo: ?Sized,
    ScalarValue: 'inp,
    Behavior: ?Sized = behavior::Standard,
>: InputType<'inp, TypeInfo, ScalarValue, Behavior>
{
    fn assert_input_object();
}

pub trait Scalar<
    'inp,
    TypeInfo: ?Sized,
    Context: ?Sized,
    ScalarValue: 'inp,
    Behavior: ?Sized = behavior::Standard,
>:
    InputType<'inp, TypeInfo, ScalarValue, Behavior>
    + OutputType<TypeInfo, Context, ScalarValue, Behavior>
    + resolve::ScalarToken<ScalarValue, Behavior>
{
    fn assert_scalar();
}

pub trait ScalarAs<
    'inp,
    Wrapper,
    TypeInfo: ?Sized,
    Context: ?Sized,
    ScalarValue: 'inp,
    Behavior: ?Sized = behavior::Standard,
>:
    InputTypeAs<'inp, Wrapper, TypeInfo, ScalarValue, Behavior>
    + OutputType<TypeInfo, Context, ScalarValue, Behavior>
    + resolve::ScalarToken<ScalarValue, Behavior>
{
    fn assert_scalar();
}

/*
pub trait Union<S>
 OutputType<S>
+ resolve::TypeName
+ resolve::ConcreteTypeName
+ resolve::Value<S>
+ resolve::ValueAsync<S>
+ resolve::ConcreteValue<S>
+ resolve::ConcreteValueAsync<S>
{
    fn assert_union();
}*/

pub trait InputType<
    'inp,
    TypeInfo: ?Sized,
    ScalarValue: 'inp,
    Behavior: ?Sized = behavior::Standard,
>:
    Type<TypeInfo, ScalarValue, Behavior>
    + resolve::ToInputValue<ScalarValue, Behavior>
    + resolve::InputValue<'inp, ScalarValue, Behavior>
{
    fn assert_input_type();
}

pub trait InputTypeAs<
    'inp,
    Wrapper,
    TypeInfo: ?Sized,
    ScalarValue: 'inp,
    Behavior: ?Sized = behavior::Standard,
>:
    Type<TypeInfo, ScalarValue, Behavior>
    + resolve::ToInputValue<ScalarValue, Behavior>
    + resolve::InputValueAs<'inp, Wrapper, ScalarValue, Behavior>
{
    fn assert_input_type();
}

pub trait OutputType<
    TypeInfo: ?Sized,
    Context: ?Sized,
    ScalarValue,
    Behavior: ?Sized = behavior::Standard,
>:
    Type<TypeInfo, ScalarValue, Behavior>
    + resolve::Value<TypeInfo, Context, ScalarValue, Behavior>
    + resolve::ValueAsync<TypeInfo, Context, ScalarValue, Behavior>
{
    fn assert_output_type();
}
