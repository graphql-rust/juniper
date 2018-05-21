use ast::{Fragment, InlineFragment};
use parser::Spanning;
use validation::{ValidatorContext, Visitor};

pub struct FragmentsOnCompositeTypes {}

pub fn factory() -> FragmentsOnCompositeTypes {
    FragmentsOnCompositeTypes {}
}

impl<'a> Visitor<'a> for FragmentsOnCompositeTypes {
    fn enter_fragment_definition(
        &mut self,
        context: &mut ValidatorContext<'a>,
        f: &'a Spanning<Fragment>,
    ) {
        {
            if let Some(current_type) = context.current_type() {
                if !current_type.is_composite() {
                    let type_name = current_type.name().unwrap_or("<unknown>");
                    let type_cond = &f.item.type_condition;

                    context.report_error(
                        &error_message(Some(f.item.name.item), type_name),
                        &[type_cond.start.clone()],
                    );
                }
            }
        }
    }

    fn enter_inline_fragment(
        &mut self,
        context: &mut ValidatorContext<'a>,
        f: &'a Spanning<InlineFragment>,
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
                    context.report_error(&error_message(None, name), &[type_cond.start.clone()]);
                }
            }
        }
    }
}

fn error_message(fragment_name: Option<&str>, on_type: &str) -> String {
    if let Some(name) = fragment_name {
        format!(
            r#"Fragment "{}" cannot condition non composite type "{}"#,
            name, on_type
        )
    } else {
        format!(
            r#"Fragment cannot condition on non composite type "{}""#,
            on_type
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{error_message, factory};

    use parser::SourcePosition;
    use validation::{expect_fails_rule, expect_passes_rule, RuleError};

    #[test]
    fn on_object() {
        expect_passes_rule(
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
        expect_passes_rule(
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
        expect_passes_rule(
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
        expect_passes_rule(
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
        expect_passes_rule(
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
        expect_fails_rule(
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
        expect_fails_rule(
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
        expect_fails_rule(
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
        expect_fails_rule(
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
