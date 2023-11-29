use std::collections::hash_map::{Entry, HashMap};

use crate::{
    ast::{Directive, Field, InputValue},
    parser::{SourcePosition, Spanning},
    validation::{ValidatorContext, Visitor},
    value::ScalarValue,
};

pub struct UniqueArgumentNames<'a> {
    known_names: HashMap<&'a str, SourcePosition>,
}

pub fn factory<'a>() -> UniqueArgumentNames<'a> {
    UniqueArgumentNames {
        known_names: HashMap::new(),
    }
}

impl<'a, S> Visitor<'a, S> for UniqueArgumentNames<'a>
where
    S: ScalarValue,
{
    fn enter_directive(&mut self, _: &mut ValidatorContext<'a, S>, _: &'a Spanning<Directive<S>>) {
        self.known_names = HashMap::new();
    }

    fn enter_field(&mut self, _: &mut ValidatorContext<'a, S>, _: &'a Spanning<Field<S>>) {
        self.known_names = HashMap::new();
    }

    fn enter_argument(
        &mut self,
        ctx: &mut ValidatorContext<'a, S>,
        (arg_name, _): &'a (Spanning<&'a str>, Spanning<InputValue<S>>),
    ) {
        match self.known_names.entry(arg_name.item) {
            Entry::Occupied(e) => {
                ctx.report_error(
                    &error_message(arg_name.item),
                    &[*e.get(), arg_name.span.start],
                );
            }
            Entry::Vacant(e) => {
                e.insert(arg_name.span.start);
            }
        }
    }
}

fn error_message(arg_name: &str) -> String {
    format!("There can only be one argument named \"{arg_name}\"")
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
    fn no_arguments_on_field() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          {
            field
          }
        "#,
        );
    }

    #[test]
    fn no_arguments_on_directive() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          {
            dog @directive
          }
        "#,
        );
    }

    #[test]
    fn argument_on_field() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          {
            field(arg: "value")
          }
        "#,
        );
    }

    #[test]
    fn argument_on_directive() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          {
            dog @directive(arg: "value")
          }
        "#,
        );
    }

    #[test]
    fn same_argument_on_two_fields() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          {
            one: field(arg: "value")
            two: field(arg: "value")
          }
        "#,
        );
    }

    #[test]
    fn same_argument_on_field_and_directive() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          {
            field(arg: "value") @directive(arg: "value")
          }
        "#,
        );
    }

    #[test]
    fn same_argument_on_two_directives() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          {
            field @directive1(arg: "value") @directive2(arg: "value")
          }
        "#,
        );
    }

    #[test]
    fn multiple_field_arguments() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          {
            field(arg1: "value", arg2: "value", arg3: "value")
          }
        "#,
        );
    }

    #[test]
    fn multiple_directive_arguments() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          {
            field @directive(arg1: "value", arg2: "value", arg3: "value")
          }
        "#,
        );
    }

    #[test]
    fn duplicate_field_arguments() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          {
            field(arg1: "value", arg1: "value")
          }
        "#,
            &[RuleError::new(
                &error_message("arg1"),
                &[
                    SourcePosition::new(31, 2, 18),
                    SourcePosition::new(46, 2, 33),
                ],
            )],
        );
    }

    #[test]
    fn many_duplicate_field_arguments() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          {
            field(arg1: "value", arg1: "value", arg1: "value")
          }
        "#,
            &[
                RuleError::new(
                    &error_message("arg1"),
                    &[
                        SourcePosition::new(31, 2, 18),
                        SourcePosition::new(46, 2, 33),
                    ],
                ),
                RuleError::new(
                    &error_message("arg1"),
                    &[
                        SourcePosition::new(31, 2, 18),
                        SourcePosition::new(61, 2, 48),
                    ],
                ),
            ],
        );
    }

    #[test]
    fn duplicate_directive_arguments() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          {
            field @directive(arg1: "value", arg1: "value")
          }
        "#,
            &[RuleError::new(
                &error_message("arg1"),
                &[
                    SourcePosition::new(42, 2, 29),
                    SourcePosition::new(57, 2, 44),
                ],
            )],
        );
    }

    #[test]
    fn many_duplicate_directive_arguments() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          {
            field @directive(arg1: "value", arg1: "value", arg1: "value")
          }
        "#,
            &[
                RuleError::new(
                    &error_message("arg1"),
                    &[
                        SourcePosition::new(42, 2, 29),
                        SourcePosition::new(57, 2, 44),
                    ],
                ),
                RuleError::new(
                    &error_message("arg1"),
                    &[
                        SourcePosition::new(42, 2, 29),
                        SourcePosition::new(72, 2, 59),
                    ],
                ),
            ],
        );
    }
}
