use crate::{
    ast::{Fragment, InlineFragment, VariableDefinition},
    parser::{SourcePosition, Spanning},
    validation::{ValidatorContext, Visitor},
    value::ScalarValue,
};
use std::fmt::Debug;

pub struct KnownTypeNames;

pub fn factory() -> KnownTypeNames {
    KnownTypeNames
}

impl<'a, S> Visitor<'a, S> for KnownTypeNames
where
    S: ScalarValue,
{
    fn enter_inline_fragment(
        &mut self,
        ctx: &mut ValidatorContext<'a, S>,
        fragment: &'a Spanning<InlineFragment<S>>,
    ) {
        if let Some(ref type_cond) = fragment.item.type_condition {
            validate_type(ctx, type_cond.item, &type_cond.span.start);
        }
    }

    fn enter_fragment_definition(
        &mut self,
        ctx: &mut ValidatorContext<'a, S>,
        fragment: &'a Spanning<Fragment<S>>,
    ) {
        let type_cond = &fragment.item.type_condition;
        validate_type(ctx, type_cond.item, &type_cond.span.start);
    }

    fn enter_variable_definition(
        &mut self,
        ctx: &mut ValidatorContext<'a, S>,
        (_, var_def): &'a (Spanning<&'a str>, VariableDefinition<S>),
    ) {
        let type_name = var_def.var_type.item.innermost_name();
        validate_type(ctx, type_name, &var_def.var_type.span.start);
    }
}

fn validate_type<S: Debug>(
    ctx: &mut ValidatorContext<'_, S>,
    type_name: &str,
    location: &SourcePosition,
) {
    if ctx.schema.type_by_name(type_name).is_none() {
        ctx.report_error(&error_message(type_name), &[*location]);
    }
}

fn error_message(type_name: &str) -> String {
    format!(r#"Unknown type "{type_name}""#)
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
    fn known_type_names_are_valid() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          query Foo($var: String, $required: [String!]!) {
            user(id: 4) {
              pets { ... on Pet { name }, ...PetFields, ... { name } }
            }
          }
          fragment PetFields on Pet {
            name
          }
        "#,
        );
    }

    #[test]
    fn unknown_type_names_are_invalid() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          query Foo($var: JumbledUpLetters) {
            user(id: 4) {
              name
              pets { ... on Badger { name }, ...PetFields }
            }
          }
          fragment PetFields on Peettt {
            name
          }
        "#,
            &[
                RuleError::new(
                    &error_message("JumbledUpLetters"),
                    &[SourcePosition::new(27, 1, 26)],
                ),
                RuleError::new(&error_message("Badger"), &[SourcePosition::new(120, 4, 28)]),
                RuleError::new(&error_message("Peettt"), &[SourcePosition::new(210, 7, 32)]),
            ],
        );
    }
}
