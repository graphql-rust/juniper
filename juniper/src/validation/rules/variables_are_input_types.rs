use std::fmt;

use crate::{
    ast::VariableDefinition,
    parser::Spanning,
    validation::{ValidatorContext, Visitor},
    value::ScalarValue,
};

pub struct UniqueVariableNames;

pub fn factory() -> UniqueVariableNames {
    UniqueVariableNames
}

impl<'a, S> Visitor<'a, S> for UniqueVariableNames
where
    S: ScalarValue,
{
    fn enter_variable_definition(
        &mut self,
        ctx: &mut ValidatorContext<'a, S>,
        (var_name, var_def): &'a (Spanning<&'a str>, VariableDefinition<S>),
    ) {
        if let Some(var_type) = ctx
            .schema
            .concrete_type_by_name(var_def.var_type.item.innermost_name())
        {
            if !var_type.is_input() {
                ctx.report_error(
                    &error_message(var_name.item, &var_def.var_type.item),
                    &[var_def.var_type.span.start],
                );
            }
        }
    }
}

fn error_message(var_name: impl fmt::Display, type_name: impl fmt::Display) -> String {
    format!("Variable \"{var_name}\" cannot be of non-input type \"{type_name}\"")
}

#[cfg(test)]
mod tests {
    use super::{error_message, factory};

    use crate::{
        parser::SourcePosition,
        validation::{expect_fails_rule, expect_passes_rule, RuleError},
        value::DefaultScalarValue,
    };

    #[test]
    fn input_types_are_valid() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          query Foo($a: String, $b: [Boolean!]!, $c: ComplexInput) {
            field(a: $a, b: $b, c: $c)
          }
        "#,
        );
    }

    #[test]
    fn output_types_are_invalid() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          query Foo($a: Dog, $b: [[CatOrDog!]]!, $c: Pet) {
            field(a: $a, b: $b, c: $c)
          }
        "#,
            &[
                RuleError::new(
                    &error_message("a", "Dog"),
                    &[SourcePosition::new(25, 1, 24)],
                ),
                RuleError::new(
                    &error_message("b", "[[CatOrDog!]]!"),
                    &[SourcePosition::new(34, 1, 33)],
                ),
                RuleError::new(
                    &error_message("c", "Pet"),
                    &[SourcePosition::new(54, 1, 53)],
                ),
            ],
        );
    }
}
