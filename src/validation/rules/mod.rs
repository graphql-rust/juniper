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

use ast::Document;
use validation::{ValidatorContext, MultiVisitor, visit};

#[doc(hidden)]
pub fn visit_all_rules<'a>(ctx: &mut ValidatorContext<'a>, doc: &'a Document) {
    let mut mv = MultiVisitor::new(vec![
        Box::new(self::arguments_of_correct_type::factory()),
        Box::new(self::default_values_of_correct_type::factory()),
        Box::new(self::fields_on_correct_type::factory()),
        Box::new(self::fragments_on_composite_types::factory()),
        Box::new(self::known_argument_names::factory()),
        Box::new(self::known_directives::factory()),
        Box::new(self::known_fragment_names::factory()),
        Box::new(self::known_type_names::factory()),
        Box::new(self::lone_anonymous_operation::factory()),
        Box::new(self::no_fragment_cycles::factory()),
        Box::new(self::no_undefined_variables::factory()),
        Box::new(self::no_unused_fragments::factory()),
        Box::new(self::no_unused_variables::factory()),
        Box::new(self::overlapping_fields_can_be_merged::factory()),
        Box::new(self::possible_fragment_spreads::factory()),
        Box::new(self::provided_non_null_arguments::factory()),
        Box::new(self::scalar_leafs::factory()),
        Box::new(self::unique_argument_names::factory()),
        Box::new(self::unique_fragment_names::factory()),
        Box::new(self::unique_input_field_names::factory()),
        Box::new(self::unique_operation_names::factory()),
        Box::new(self::unique_variable_names::factory()),
        Box::new(self::variables_are_input_types::factory()),
        Box::new(self::variables_in_allowed_position::factory()),
    ]);

    visit(&mut mv, ctx, doc);
}
