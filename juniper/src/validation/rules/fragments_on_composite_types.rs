use crate::{
    ast::{Fragment, InlineFragment},
    parser::Spanning,
    validation::{ValidatorContext, Visitor},
    value::ScalarValue,
};

pub struct FragmentsOnCompositeTypes;

pub fn factory() -> FragmentsOnCompositeTypes {
    FragmentsOnCompositeTypes
}

impl<'a, S> Visitor<'a, S> for FragmentsOnCompositeTypes
where
    S: ScalarValue,
{
    fn enter_fragment_definition(
        &mut self,
        context: &mut ValidatorContext<'a, S>,
        f: &'a Spanning<Fragment<S>>,
    ) {
        {
            if let Some(current_type) = context.current_type() {
                if !current_type.is_composite() {
                    let type_name = current_type.name().unwrap_or("<unknown>");
                    let type_cond = &f.item.type_condition;

                    context.report_error(
                        &error_message(Some(f.item.name.item), type_name),
                        &[type_cond.span.start],
                    );
                }
            }
        }
    }

    fn enter_inline_fragment(
        &mut self,
        context: &mut ValidatorContext<'a, S>,
        f: &'a Spanning<InlineFragment<S>>,
    ) {
        {
            if let Some(ref type_cond) = f.item.type_condition {
                let invalid_type_name = context
                    .current_type()
                    .iter()
                    .filter(|&t| !t.is_composite())
                    .map(|t| t.name().unwrap_or("<unknown>"))
                    .next();

                if let Some(name) = invalid_type_name {
                    context.report_error(&error_message(None, name), &[type_cond.span.start]);
                }
            }
        }
    }
}

fn error_message(fragment_name: Option<&str>, on_type: &str) -> String {
    if let Some(name) = fragment_name {
        format!(r#"Fragment "{name}" cannot condition non composite type "{on_type}"#)
    } else {
        format!(r#"Fragment cannot condition on non composite type "{on_type}""#)
    }
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
    fn on_object() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment validFragment on Dog {
            barks
          }
        "#,
        );
    }

    #[test]
    fn on_interface() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment validFragment on Pet {
            name
          }
        "#,
        );
    }

    #[test]
    fn on_object_inline() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment validFragment on Pet {
            ... on Dog {
              barks
            }
          }
        "#,
        );
    }

    #[test]
    fn on_inline_without_type_cond() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment validFragment on Pet {
            ... {
              name
            }
          }
        "#,
        );
    }

    #[test]
    fn on_union() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment validFragment on CatOrDog {
            __typename
          }
        "#,
        );
    }

    #[test]
    fn not_on_scalar() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment scalarFragment on Boolean {
            bad
          }
        "#,
            &[RuleError::new(
                &error_message(Some("scalarFragment"), "Boolean"),
                &[SourcePosition::new(38, 1, 37)],
            )],
        );
    }

    #[test]
    fn not_on_enum() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment scalarFragment on FurColor {
            bad
          }
        "#,
            &[RuleError::new(
                &error_message(Some("scalarFragment"), "FurColor"),
                &[SourcePosition::new(38, 1, 37)],
            )],
        );
    }

    #[test]
    fn not_on_input_object() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment inputFragment on ComplexInput {
            stringField
          }
        "#,
            &[RuleError::new(
                &error_message(Some("inputFragment"), "ComplexInput"),
                &[SourcePosition::new(37, 1, 36)],
            )],
        );
    }

    #[test]
    fn not_on_scalar_inline() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment invalidFragment on Pet {
            ... on String {
              barks
            }
          }
        "#,
            &[RuleError::new(
                &error_message(None, "String"),
                &[SourcePosition::new(64, 2, 19)],
            )],
        );
    }
}
