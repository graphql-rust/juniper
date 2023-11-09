use crate::{
    ast::{Directive, Field, InputValue},
    parser::Spanning,
    schema::meta::Argument,
    validation::{ValidatorContext, Visitor},
    value::ScalarValue,
};
use std::fmt::Debug;

#[derive(Debug)]
enum ArgumentPosition<'a> {
    Directive(&'a str),
    Field(&'a str, &'a str),
}

pub struct KnownArgumentNames<'a, S: Debug + 'a> {
    current_args: Option<(ArgumentPosition<'a>, &'a Vec<Argument<'a, S>>)>,
}

pub fn factory<'a, S: Debug>() -> KnownArgumentNames<'a, S> {
    KnownArgumentNames { current_args: None }
}

impl<'a, S> Visitor<'a, S> for KnownArgumentNames<'a, S>
where
    S: ScalarValue,
{
    fn enter_directive(
        &mut self,
        ctx: &mut ValidatorContext<'a, S>,
        directive: &'a Spanning<Directive<S>>,
    ) {
        self.current_args = ctx
            .schema
            .directive_by_name(directive.item.name.item)
            .map(|d| {
                (
                    ArgumentPosition::Directive(directive.item.name.item),
                    &d.arguments,
                )
            });
    }

    fn exit_directive(&mut self, _: &mut ValidatorContext<'a, S>, _: &'a Spanning<Directive<S>>) {
        self.current_args = None;
    }

    fn enter_field(&mut self, ctx: &mut ValidatorContext<'a, S>, field: &'a Spanning<Field<S>>) {
        self.current_args = ctx
            .parent_type()
            .and_then(|t| t.field_by_name(field.item.name.item))
            .and_then(|f| f.arguments.as_ref())
            .map(|args| {
                (
                    ArgumentPosition::Field(
                        field.item.name.item,
                        ctx.parent_type()
                            .expect("Parent type should exist")
                            .name()
                            .expect("Parent type should be named"),
                    ),
                    args,
                )
            });
    }

    fn exit_field(&mut self, _: &mut ValidatorContext<'a, S>, _: &'a Spanning<Field<S>>) {
        self.current_args = None;
    }

    fn enter_argument(
        &mut self,
        ctx: &mut ValidatorContext<'a, S>,
        (arg_name, _): &'a (Spanning<&'a str>, Spanning<InputValue<S>>),
    ) {
        if let Some((ref pos, args)) = self.current_args {
            if !args.iter().any(|a| a.name == arg_name.item) {
                let message = match *pos {
                    ArgumentPosition::Field(field_name, type_name) => {
                        field_error_message(arg_name.item, field_name, type_name)
                    }
                    ArgumentPosition::Directive(directive_name) => {
                        directive_error_message(arg_name.item, directive_name)
                    }
                };

                ctx.report_error(&message, &[arg_name.span.start]);
            }
        }
    }
}

fn field_error_message(arg_name: &str, field_name: &str, type_name: &str) -> String {
    format!(r#"Unknown argument "{arg_name}" on field "{field_name}" of type "{type_name}""#)
}

fn directive_error_message(arg_name: &str, directive_name: &str) -> String {
    format!(r#"Unknown argument "{arg_name}" on directive "{directive_name}""#)
}

#[cfg(test)]
mod tests {
    use super::{directive_error_message, factory, field_error_message};

    use crate::{
        parser::SourcePosition,
        validation::{expect_fails_rule, expect_passes_rule, RuleError},
        value::DefaultScalarValue,
    };

    #[test]
    fn single_arg_is_known() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment argOnRequiredArg on Dog {
            doesKnowCommand(dogCommand: SIT)
          }
        "#,
        );
    }

    #[test]
    fn multiple_args_are_known() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment multipleArgs on ComplicatedArgs {
            multipleReqs(req1: 1, req2: 2)
          }
        "#,
        );
    }

    #[test]
    fn ignores_args_of_unknown_fields() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment argOnUnknownField on Dog {
            unknownField(unknownArg: SIT)
          }
        "#,
        );
    }

    #[test]
    fn multiple_args_in_reverse_order_are_known() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment multipleArgsReverseOrder on ComplicatedArgs {
            multipleReqs(req2: 2, req1: 1)
          }
        "#,
        );
    }

    #[test]
    fn no_args_on_optional_arg() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment noArgOnOptionalArg on Dog {
            isHousetrained
          }
        "#,
        );
    }

    #[test]
    fn args_are_known_deeply() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          {
            dog {
              doesKnowCommand(dogCommand: SIT)
            }
            human {
              pet {
                ... on Dog {
                  doesKnowCommand(dogCommand: SIT)
                }
              }
            }
          }
        "#,
        );
    }

    #[test]
    fn directive_args_are_known() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          {
            dog @skip(if: true)
          }
        "#,
        );
    }

    #[test]
    fn undirective_args_are_invalid() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          {
            dog @skip(unless: true)
          }
        "#,
            &[RuleError::new(
                &directive_error_message("unless", "skip"),
                &[SourcePosition::new(35, 2, 22)],
            )],
        );
    }

    #[test]
    fn invalid_arg_name() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment invalidArgName on Dog {
            doesKnowCommand(unknown: true)
          }
        "#,
            &[RuleError::new(
                &field_error_message("unknown", "doesKnowCommand", "Dog"),
                &[SourcePosition::new(72, 2, 28)],
            )],
        );
    }

    #[test]
    fn unknown_args_amongst_known_args() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment oneGoodArgOneInvalidArg on Dog {
            doesKnowCommand(whoknows: 1, dogCommand: SIT, unknown: true)
          }
        "#,
            &[
                RuleError::new(
                    &field_error_message("whoknows", "doesKnowCommand", "Dog"),
                    &[SourcePosition::new(81, 2, 28)],
                ),
                RuleError::new(
                    &field_error_message("unknown", "doesKnowCommand", "Dog"),
                    &[SourcePosition::new(111, 2, 58)],
                ),
            ],
        );
    }

    #[test]
    fn unknown_args_deeply() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          {
            dog {
              doesKnowCommand(unknown: true)
            }
            human {
              pet {
                ... on Dog {
                  doesKnowCommand(unknown: true)
                }
              }
            }
          }
        "#,
            &[
                RuleError::new(
                    &field_error_message("unknown", "doesKnowCommand", "Dog"),
                    &[SourcePosition::new(61, 3, 30)],
                ),
                RuleError::new(
                    &field_error_message("unknown", "doesKnowCommand", "Dog"),
                    &[SourcePosition::new(193, 8, 34)],
                ),
            ],
        );
    }
}
