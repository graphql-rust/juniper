use crate::{
    ast::{
        Directive, Document, Field, Fragment, FragmentSpread, InlineFragment, InputValue,
        Operation, Selection, VariableDefinition,
    },
    parser::Spanning,
    validation::ValidatorContext,
    value::ScalarValue,
    Span,
};

#[doc(hidden)]
pub trait Visitor<'a, S>
where
    S: ScalarValue,
{
    fn enter_document(&mut self, _: &mut ValidatorContext<'a, S>, _: &'a Document<S>) {}
    fn exit_document(&mut self, _: &mut ValidatorContext<'a, S>, _: &'a Document<S>) {}

    fn enter_operation_definition(
        &mut self,
        _: &mut ValidatorContext<'a, S>,
        _: &'a Spanning<Operation<S>>,
    ) {
    }
    fn exit_operation_definition(
        &mut self,
        _: &mut ValidatorContext<'a, S>,
        _: &'a Spanning<Operation<S>>,
    ) {
    }

    fn enter_fragment_definition(
        &mut self,
        _: &mut ValidatorContext<'a, S>,
        _: &'a Spanning<Fragment<S>>,
    ) {
    }
    fn exit_fragment_definition(
        &mut self,
        _: &mut ValidatorContext<'a, S>,
        _: &'a Spanning<Fragment<S>>,
    ) {
    }

    fn enter_variable_definition(
        &mut self,
        _: &mut ValidatorContext<'a, S>,
        _: &'a (Spanning<&'a str>, VariableDefinition<S>),
    ) {
    }
    fn exit_variable_definition(
        &mut self,
        _: &mut ValidatorContext<'a, S>,
        _: &'a (Spanning<&'a str>, VariableDefinition<S>),
    ) {
    }

    fn enter_directive(&mut self, _: &mut ValidatorContext<'a, S>, _: &'a Spanning<Directive<S>>) {}
    fn exit_directive(&mut self, _: &mut ValidatorContext<'a, S>, _: &'a Spanning<Directive<S>>) {}

    fn enter_argument(
        &mut self,
        _: &mut ValidatorContext<'a, S>,
        _: &'a (Spanning<&'a str>, Spanning<InputValue<S>>),
    ) {
    }
    fn exit_argument(
        &mut self,
        _: &mut ValidatorContext<'a, S>,
        _: &'a (Spanning<&'a str>, Spanning<InputValue<S>>),
    ) {
    }

    fn enter_selection_set(&mut self, _: &mut ValidatorContext<'a, S>, _: &'a [Selection<S>]) {}
    fn exit_selection_set(&mut self, _: &mut ValidatorContext<'a, S>, _: &'a [Selection<S>]) {}

    fn enter_field(&mut self, _: &mut ValidatorContext<'a, S>, _: &'a Spanning<Field<S>>) {}
    fn exit_field(&mut self, _: &mut ValidatorContext<'a, S>, _: &'a Spanning<Field<S>>) {}

    fn enter_fragment_spread(
        &mut self,
        _: &mut ValidatorContext<'a, S>,
        _: &'a Spanning<FragmentSpread<S>>,
    ) {
    }
    fn exit_fragment_spread(
        &mut self,
        _: &mut ValidatorContext<'a, S>,
        _: &'a Spanning<FragmentSpread<S>>,
    ) {
    }

    fn enter_inline_fragment(
        &mut self,
        _: &mut ValidatorContext<'a, S>,
        _: &'a Spanning<InlineFragment<S>>,
    ) {
    }
    fn exit_inline_fragment(
        &mut self,
        _: &mut ValidatorContext<'a, S>,
        _: &'a Spanning<InlineFragment<S>>,
    ) {
    }

    fn enter_null_value(&mut self, _: &mut ValidatorContext<'a, S>, _: SpannedInput<'a, ()>) {}
    fn exit_null_value(&mut self, _: &mut ValidatorContext<'a, S>, _: SpannedInput<'a, ()>) {}

    fn enter_scalar_value(&mut self, _: &mut ValidatorContext<'a, S>, _: SpannedInput<'a, S>) {}
    fn exit_scalar_value(&mut self, _: &mut ValidatorContext<'a, S>, _: SpannedInput<'a, S>) {}

    fn enter_enum_value(&mut self, _: &mut ValidatorContext<'a, S>, _: SpannedInput<'a, String>) {}
    fn exit_enum_value(&mut self, _: &mut ValidatorContext<'a, S>, _: SpannedInput<'a, String>) {}

    fn enter_variable_value(
        &mut self,
        _: &mut ValidatorContext<'a, S>,
        _: SpannedInput<'a, String>,
    ) {
    }
    fn exit_variable_value(
        &mut self,
        _: &mut ValidatorContext<'a, S>,
        _: SpannedInput<'a, String>,
    ) {
    }

    fn enter_list_value(
        &mut self,
        _: &mut ValidatorContext<'a, S>,
        _: SpannedInput<'a, Vec<Spanning<InputValue<S>>>>,
    ) {
    }
    fn exit_list_value(
        &mut self,
        _: &mut ValidatorContext<'a, S>,
        _: SpannedInput<'a, Vec<Spanning<InputValue<S>>>>,
    ) {
    }

    fn enter_object_value(&mut self, _: &mut ValidatorContext<'a, S>, _: SpannedObject<'a, S>) {}
    fn exit_object_value(&mut self, _: &mut ValidatorContext<'a, S>, _: SpannedObject<'a, S>) {}

    fn enter_object_field(
        &mut self,
        _: &mut ValidatorContext<'a, S>,
        _: (SpannedInput<'a, String>, SpannedInput<'a, InputValue<S>>),
    ) {
    }
    fn exit_object_field(
        &mut self,
        _: &mut ValidatorContext<'a, S>,
        _: (SpannedInput<'a, String>, SpannedInput<'a, InputValue<S>>),
    ) {
    }
}

type SpannedInput<'a, T> = Spanning<&'a T, &'a Span>;
type SpannedObject<'a, S> = SpannedInput<'a, Vec<(Spanning<String>, Spanning<InputValue<S>>)>>;
