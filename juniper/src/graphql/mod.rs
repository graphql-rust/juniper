pub mod resolve;

use crate::DefaultScalarValue;

pub use crate::{
    ast::InputValue, graphql_input_value as input_value, graphql_value as value, value::Value,
};

pub use self::resolve::Type;

pub trait Interface<S = DefaultScalarValue>:
    OutputType<S>
    + Type<S>
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

pub trait Object<S = DefaultScalarValue>:
    OutputType<S>
    + Type<S>
    + resolve::TypeName
    + resolve::ConcreteTypeName
    + resolve::Value<S>
    + resolve::ValueAsync<S>
    + resolve::Field<S>
    + resolve::FieldAsync<S>
{
    fn assert_object();
}

pub trait Scalar<S = DefaultScalarValue>:
    InputType<S>
    + OutputType<S>
    + Type<S>
    + resolve::TypeName
    + resolve::Value<S>
    + resolve::ValueAsync<S>
{
    fn assert_scalar();
}

pub trait Union<S = DefaultScalarValue>:
    OutputType<S>
    + Type<S>
    + resolve::TypeName
    + resolve::ConcreteTypeName
    + resolve::Value<S>
    + resolve::ValueAsync<S>
    + resolve::ConcreteValue<S>
    + resolve::ConcreteValueAsync<S>
{
    fn assert_union();
}

pub trait InputType<S = DefaultScalarValue> {
    fn assert_input_type();
}

pub trait OutputType<S = DefaultScalarValue> {
    fn assert_output_type();
}
