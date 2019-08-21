mod arguments_of_correct_type;
mod default_values_of_correct_type;
mod fields_on_correct_type;
mod fragments_on_composite_types;
mod known_argument_names;
mod known_directives;
mod known_fragment_names;
mod known_type_names;
mod lone_anonymous_operation;
mod no_fragment_cycles;
mod no_undefined_variables;
mod no_unused_fragments;
mod no_unused_variables;
mod overlapping_fields_can_be_merged;
mod possible_fragment_spreads;
mod provided_non_null_arguments;
mod scalar_leafs;
mod unique_argument_names;
mod unique_fragment_names;
mod unique_input_field_names;
mod unique_operation_names;
mod unique_variable_names;
mod variables_are_input_types;
mod variables_in_allowed_position;

use crate::{
    ast::Document,
    validation::{visit, MultiVisitorNil, ValidatorContext},
    value::ScalarValue,
};
use std::fmt::Debug;

pub(crate) fn visit_all_rules<'a, S: Debug>(ctx: &mut ValidatorContext<'a, S>, doc: &'a Document<S>)
where
    S: ScalarValue,
{
    let mut mv = MultiVisitorNil
        .with(self::arguments_of_correct_type::factory())
        .with(self::default_values_of_correct_type::factory())
        .with(self::fields_on_correct_type::factory())
        .with(self::fragments_on_composite_types::factory())
        .with(self::known_argument_names::factory())
        .with(self::known_directives::factory())
        .with(self::known_fragment_names::factory())
        .with(self::known_type_names::factory())
        .with(self::lone_anonymous_operation::factory())
        .with(self::no_fragment_cycles::factory())
        .with(self::no_undefined_variables::factory())
        .with(self::no_unused_fragments::factory())
        .with(self::no_unused_variables::factory())
        .with(self::overlapping_fields_can_be_merged::factory())
        .with(self::possible_fragment_spreads::factory())
        .with(self::provided_non_null_arguments::factory())
        .with(self::scalar_leafs::factory())
        .with(self::unique_argument_names::factory())
        .with(self::unique_fragment_names::factory())
        .with(self::unique_input_field_names::factory())
        .with(self::unique_operation_names::factory())
        .with(self::unique_variable_names::factory())
        .with(self::variables_are_input_types::factory())
        .with(self::variables_in_allowed_position::factory());

    visit(&mut mv, ctx, doc)
}
