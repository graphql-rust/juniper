pub use crate::{
    ast::InputValue, graphql_input_value as input_value, graphql_value as value, value::Value,
};

pub trait Interface<S>:
    OutputType<S>
/*
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

pub trait Object<S>:
    OutputType<S>
/*
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

pub trait Scalar<S>:
    OutputType<S> + /*
    resolve::TypeName + resolve::Value<S> + resolve::ValueAsync<S> */
{
    fn assert_scalar();
}

pub trait Union<S>:
    OutputType<S>
/*
    + resolve::TypeName
    + resolve::ConcreteTypeName
    + resolve::Value<S>
    + resolve::ValueAsync<S>
    + resolve::ConcreteValue<S>
    + resolve::ConcreteValueAsync<S> */
{
    fn assert_union();
}

pub trait InputType<'inp, Info: ?Sized, S: 'inp> /*:
    crate::resolve::Type<Info, S>
    + crate::resolve::ToInputValue<S>
    + crate::resolve::InputValue<'inp, S>*/
{
    fn assert_input_type();
}

pub trait OutputType<S>: /*Type<S>*/ {
    fn assert_output_type();
}
