use crate::{behavior, resolve};

pub use crate::{
    ast::InputValue, graphql_value as value, macros::input_value,
    resolve::Type, value::Value,
};

pub trait Interface<S>
/*: OutputType<S>
   + resolve::TypeName
   + resolve::ConcreteTypeName
   + resolve::Value<S>
   + resolve::ValueAsync<S>
   + resolve::ConcreteValue<S>
   + resolve::ConcreteValueAsync<S>
   + resolve::Field<S>
   + resolve::FieldAsync<S>

*/
{
    fn assert_interface();
}

pub trait Object<S>
/*: OutputType<S>
   + resolve::TypeName
   + resolve::ConcreteTypeName
   + resolve::Value<S>
   + resolve::ValueAsync<S>
   + resolve::Field<S>
   + resolve::FieldAsync<S>

*/
{
    fn assert_object();
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

pub trait Union<S>
/*: OutputType<S>
+ resolve::TypeName
+ resolve::ConcreteTypeName
+ resolve::Value<S>
+ resolve::ValueAsync<S>
+ resolve::ConcreteValue<S>
+ resolve::ConcreteValueAsync<S> */
{
    fn assert_union();
}

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
