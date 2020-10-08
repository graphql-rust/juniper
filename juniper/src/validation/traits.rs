use crate::{
    ast::{
        Directive, Document, Field, Fragment, FragmentSpread, InlineFragment, InputValue,
        Operation, Selection, VariableDefinition,
    },
    parser::Spanning,
    validation::ValidatorContext,
    DefaultScalarValue,
};

#[doc(hidden)]
pub trait Visitor<'a> {
    fn enter_document(&mut self, _: &mut ValidatorContext<'a>, _: &'a Document) {}
    fn exit_document(&mut self, _: &mut ValidatorContext<'a>, _: &'a Document) {}

    fn enter_operation_definition(
        &mut self,
        _: &mut ValidatorContext<'a>,
        _: &'a Spanning<Operation>,
    ) {
    }
    fn exit_operation_definition(
        &mut self,
        _: &mut ValidatorContext<'a>,
        _: &'a Spanning<Operation>,
    ) {
    }

    fn enter_fragment_definition(
        &mut self,
        _: &mut ValidatorContext<'a>,
        _: &'a Spanning<Fragment>,
    ) {
    }
    fn exit_fragment_definition(
        &mut self,
        _: &mut ValidatorContext<'a>,
        _: &'a Spanning<Fragment>,
    ) {
    }

    fn enter_variable_definition(
        &mut self,
        _: &mut ValidatorContext<'a>,
        _: &'a (Spanning<&'a str>, VariableDefinition),
    ) {
    }
    fn exit_variable_definition(
        &mut self,
        _: &mut ValidatorContext<'a>,
        _: &'a (Spanning<&'a str>, VariableDefinition),
    ) {
    }

    fn enter_directive(&mut self, _: &mut ValidatorContext<'a>, _: &'a Spanning<Directive>) {}
    fn exit_directive(&mut self, _: &mut ValidatorContext<'a>, _: &'a Spanning<Directive>) {}

    fn enter_argument(
        &mut self,
        _: &mut ValidatorContext<'a>,
        _: &'a (Spanning<&'a str>, Spanning<InputValue>),
    ) {
    }
    fn exit_argument(
        &mut self,
        _: &mut ValidatorContext<'a>,
        _: &'a (Spanning<&'a str>, Spanning<InputValue>),
    ) {
    }

    fn enter_selection_set(&mut self, _: &mut ValidatorContext<'a>, _: &'a [Selection]) {}
    fn exit_selection_set(&mut self, _: &mut ValidatorContext<'a>, _: &'a [Selection]) {}

    fn enter_field(&mut self, _: &mut ValidatorContext<'a>, _: &'a Spanning<Field>) {}
    fn exit_field(&mut self, _: &mut ValidatorContext<'a>, _: &'a Spanning<Field>) {}

    fn enter_fragment_spread(
        &mut self,
        _: &mut ValidatorContext<'a>,
        _: &'a Spanning<FragmentSpread>,
    ) {
    }
    fn exit_fragment_spread(
        &mut self,
        _: &mut ValidatorContext<'a>,
        _: &'a Spanning<FragmentSpread>,
    ) {
    }

    fn enter_inline_fragment(
        &mut self,
        _: &mut ValidatorContext<'a>,
        _: &'a Spanning<InlineFragment>,
    ) {
    }
    fn exit_inline_fragment(
        &mut self,
        _: &mut ValidatorContext<'a>,
        _: &'a Spanning<InlineFragment>,
    ) {
    }

    fn enter_null_value(&mut self, _: &mut ValidatorContext<'a>, _: Spanning<()>) {}
    fn exit_null_value(&mut self, _: &mut ValidatorContext<'a>, _: Spanning<()>) {}

    fn enter_scalar_value(
        &mut self,
        _: &mut ValidatorContext<'a>,
        _: Spanning<&'a DefaultScalarValue>,
    ) {
    }
    fn exit_scalar_value(
        &mut self,
        _: &mut ValidatorContext<'a>,
        _: Spanning<&'a DefaultScalarValue>,
    ) {
    }

    fn enter_enum_value(&mut self, _: &mut ValidatorContext<'a>, _: Spanning<&'a String>) {}
    fn exit_enum_value(&mut self, _: &mut ValidatorContext<'a>, _: Spanning<&'a String>) {}

    fn enter_variable_value(&mut self, _: &mut ValidatorContext<'a>, _: Spanning<&'a String>) {}
    fn exit_variable_value(&mut self, _: &mut ValidatorContext<'a>, _: Spanning<&'a String>) {}

    fn enter_list_value(
        &mut self,
        _: &mut ValidatorContext<'a>,
        _: Spanning<&'a Vec<Spanning<InputValue>>>,
    ) {
    }
    fn exit_list_value(
        &mut self,
        _: &mut ValidatorContext<'a>,
        _: Spanning<&'a Vec<Spanning<InputValue>>>,
    ) {
    }

    fn enter_object_value(
        &mut self,
        _: &mut ValidatorContext<'a>,
        _: Spanning<&'a Vec<(Spanning<String>, Spanning<InputValue>)>>,
    ) {
    }
    fn exit_object_value(
        &mut self,
        _: &mut ValidatorContext<'a>,
        _: Spanning<&'a Vec<(Spanning<String>, Spanning<InputValue>)>>,
    ) {
    }

    fn enter_object_field(
        &mut self,
        _: &mut ValidatorContext<'a>,
        _: &'a (Spanning<String>, Spanning<InputValue>),
    ) {
    }
    fn exit_object_field(
        &mut self,
        _: &mut ValidatorContext<'a>,
        _: &'a (Spanning<String>, Spanning<InputValue>),
    ) {
    }
}
