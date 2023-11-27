//! Definitions of rules for validation.

mod arguments_of_correct_type;
mod default_values_of_correct_type;
pub mod disable_introspection;
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

use std::fmt::Debug;

use crate::{
    ast::Document,
    validation::{visit, MultiVisitorNil, ValidatorContext},
    value::ScalarValue,
};

#[doc(hidden)]
pub fn visit_all_rules<'a, S: Debug>(ctx: &mut ValidatorContext<'a, S>, doc: &'a Document<S>)
where
    S: ScalarValue,
{
    // Some validators are depending on the results of other ones.
    // For example, validators checking fragments usually rely on the fact that
    // they have no cycles (`no_fragment_cycles`), otherwise may stall in an
    // infinite recursion. So, we should run validators in stages, moving to the
    // next stage only once the previous succeeds. This is better than making
    // every single validator being aware of fragments cycles and/or other
    // assumptions.
    let mut stage1 = MultiVisitorNil
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
    visit(&mut stage1, ctx, doc);
    if ctx.has_errors() {
        return;
    }

    let mut stage2 = MultiVisitorNil.with(self::overlapping_fields_can_be_merged::factory());
    visit(&mut stage2, ctx, doc);
}

#[cfg(test)]
mod tests {
    use crate::{parser::SourcePosition, DefaultScalarValue};

    use crate::validation::{expect_fails_fn, RuleError};

    #[test]
    fn handles_recursive_fragments() {
        expect_fails_fn::<_, DefaultScalarValue>(
            super::visit_all_rules,
            "fragment f on QueryRoot { ...f }",
            &[
                RuleError::new(
                    "Fragment \"f\" is never used",
                    &[SourcePosition::new(0, 0, 0)],
                ),
                RuleError::new(
                    "Cannot spread fragment \"f\"",
                    &[SourcePosition::new(26, 0, 26)],
                ),
            ],
        );
    }

    #[test]
    fn handles_nested_recursive_fragments() {
        expect_fails_fn::<_, DefaultScalarValue>(
            super::visit_all_rules,
            "fragment f on QueryRoot { a { ...f a { ...f } } }",
            &[
                RuleError::new(
                    "Fragment \"f\" is never used",
                    &[SourcePosition::new(0, 0, 0)],
                ),
                RuleError::new(
                    r#"Unknown field "a" on type "QueryRoot""#,
                    &[SourcePosition::new(26, 0, 26)],
                ),
                RuleError::new(
                    "Cannot spread fragment \"f\"",
                    &[SourcePosition::new(30, 0, 30)],
                ),
                RuleError::new(
                    "Cannot spread fragment \"f\"",
                    &[SourcePosition::new(39, 0, 39)],
                ),
            ],
        );
    }
}
