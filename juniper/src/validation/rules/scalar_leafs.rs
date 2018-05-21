use ast::Field;
use parser::Spanning;
use validation::{RuleError, ValidatorContext, Visitor};

pub struct ScalarLeafs {}

pub fn factory() -> ScalarLeafs {
    ScalarLeafs {}
}

impl<'a> Visitor<'a> for ScalarLeafs {
    fn enter_field(&mut self, ctx: &mut ValidatorContext<'a>, field: &'a Spanning<Field>) {
        let field_name = &field.item.name.item;

        let error = if let (Some(field_type), Some(field_type_literal)) =
            (ctx.current_type(), ctx.current_type_literal())
        {
            match (field_type.is_leaf(), &field.item.selection_set) {
                (true, &Some(_)) => Some(RuleError::new(
                    &no_allowed_error_message(field_name, &format!("{}", field_type_literal)),
                    &[field.start.clone()],
                )),
                (false, &None) => Some(RuleError::new(
                    &required_error_message(field_name, &format!("{}", field_type_literal)),
                    &[field.start.clone()],
                )),
                _ => None,
            }
        } else {
            None
        };

        if let Some(error) = error {
            ctx.append_errors(vec![error]);
        }
    }
}

fn no_allowed_error_message(field_name: &str, type_name: &str) -> String {
    format!(
        r#"Field "{}" must not have a selection since type {} has no subfields"#,
        field_name, type_name
    )
}

fn required_error_message(field_name: &str, type_name: &str) -> String {
    format!(
        r#"Field "{}" of type "{}" must have a selection of subfields. Did you mean "{} {{ ... }}"?"#,
        field_name, type_name, field_name)
}

#[cfg(test)]
mod tests {
    use super::{factory, no_allowed_error_message, required_error_message};

    use parser::SourcePosition;
    use validation::{expect_fails_rule, expect_passes_rule, RuleError};

    #[test]
    fn valid_scalar_selection() {
        expect_passes_rule(
            factory,
            r#"
          fragment scalarSelection on Dog {
            barks
          }
        "#,
        );
    }

    #[test]
    fn object_type_missing_selection() {
        expect_fails_rule(
            factory,
            r#"
          query directQueryOnObjectWithoutSubFields {
            human
          }
        "#,
            &[RuleError::new(
                &required_error_message("human", "Human"),
                &[SourcePosition::new(67, 2, 12)],
            )],
        );
    }

    #[test]
    fn interface_type_missing_selection() {
        expect_fails_rule(
            factory,
            r#"
          {
            human { pets }
          }
        "#,
            &[RuleError::new(
                &required_error_message("pets", "[Pet]"),
                &[SourcePosition::new(33, 2, 20)],
            )],
        );
    }

    #[test]
    fn valid_scalar_selection_with_args() {
        expect_passes_rule(
            factory,
            r#"
          fragment scalarSelectionWithArgs on Dog {
            doesKnowCommand(dogCommand: SIT)
          }
        "#,
        );
    }

    #[test]
    fn scalar_selection_not_allowed_on_boolean() {
        expect_fails_rule(
            factory,
            r#"
          fragment scalarSelectionsNotAllowedOnBoolean on Dog {
            barks { sinceWhen }
          }
        "#,
            &[RuleError::new(
                &no_allowed_error_message("barks", "Boolean"),
                &[SourcePosition::new(77, 2, 12)],
            )],
        );
    }

    #[test]
    fn scalar_selection_not_allowed_on_enum() {
        expect_fails_rule(
            factory,
            r#"
          fragment scalarSelectionsNotAllowedOnEnum on Cat {
            furColor { inHexdec }
          }
        "#,
            &[RuleError::new(
                &no_allowed_error_message("furColor", "FurColor"),
                &[SourcePosition::new(74, 2, 12)],
            )],
        );
    }

    #[test]
    fn scalar_selection_not_allowed_with_args() {
        expect_fails_rule(
            factory,
            r#"
          fragment scalarSelectionsNotAllowedWithArgs on Dog {
            doesKnowCommand(dogCommand: SIT) { sinceWhen }
          }
        "#,
            &[RuleError::new(
                &no_allowed_error_message("doesKnowCommand", "Boolean"),
                &[SourcePosition::new(76, 2, 12)],
            )],
        );
    }

    #[test]
    fn scalar_selection_not_allowed_with_directives() {
        expect_fails_rule(
            factory,
            r#"
          fragment scalarSelectionsNotAllowedWithDirectives on Dog {
            name @include(if: true) { isAlsoHumanName }
          }
        "#,
            &[RuleError::new(
                &no_allowed_error_message("name", "String"),
                &[SourcePosition::new(82, 2, 12)],
            )],
        );
    }

    #[test]
    fn scalar_selection_not_allowed_with_directives_and_args() {
        expect_fails_rule(
            factory,
            r#"
          fragment scalarSelectionsNotAllowedWithDirectivesAndArgs on Dog {
            doesKnowCommand(dogCommand: SIT) @include(if: true) { sinceWhen }
          }
        "#,
            &[RuleError::new(
                &no_allowed_error_message("doesKnowCommand", "Boolean"),
                &[SourcePosition::new(89, 2, 12)],
            )],
        );
    }

}
