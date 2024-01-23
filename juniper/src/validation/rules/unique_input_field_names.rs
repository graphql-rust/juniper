use std::collections::hash_map::{Entry, HashMap};

use crate::{
    ast::InputValue,
    parser::{SourcePosition, Spanning},
    validation::{ValidatorContext, Visitor},
    value::ScalarValue,
    Span,
};

pub struct UniqueInputFieldNames<'a> {
    known_name_stack: Vec<HashMap<&'a str, SourcePosition>>,
}

pub fn factory<'a>() -> UniqueInputFieldNames<'a> {
    UniqueInputFieldNames {
        known_name_stack: Vec::new(),
    }
}

impl<'a, S> Visitor<'a, S> for UniqueInputFieldNames<'a>
where
    S: ScalarValue,
{
    fn enter_object_value(&mut self, _: &mut ValidatorContext<'a, S>, _: SpannedObject<'a, S>) {
        self.known_name_stack.push(HashMap::new());
    }

    fn exit_object_value(&mut self, _: &mut ValidatorContext<'a, S>, _: SpannedObject<'a, S>) {
        self.known_name_stack.pop();
    }

    fn enter_object_field(
        &mut self,
        ctx: &mut ValidatorContext<'a, S>,
        (field_name, _): (SpannedInput<'a, String>, SpannedInput<InputValue<S>>),
    ) {
        if let Some(ref mut known_names) = self.known_name_stack.last_mut() {
            match known_names.entry(field_name.item) {
                Entry::Occupied(e) => {
                    ctx.report_error(
                        &error_message(field_name.item),
                        &[*e.get(), field_name.span.start],
                    );
                }
                Entry::Vacant(e) => {
                    e.insert(field_name.span.start);
                }
            }
        }
    }
}

type SpannedInput<'a, T> = Spanning<&'a T, &'a Span>;
type SpannedObject<'a, S> = SpannedInput<'a, Vec<(Spanning<String>, Spanning<InputValue<S>>)>>;

fn error_message(field_name: &str) -> String {
    format!("There can only be one input field named \"{field_name}\"")
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
    fn input_object_with_fields() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          {
            field(arg: { f: true })
          }
        "#,
        );
    }

    #[test]
    fn same_input_object_within_two_args() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          {
            field(arg1: { f: true }, arg2: { f: true })
          }
        "#,
        );
    }

    #[test]
    fn multiple_input_object_fields() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          {
            field(arg: { f1: "value", f2: "value", f3: "value" })
          }
        "#,
        );
    }

    #[test]
    fn allows_for_nested_input_objects_with_similar_fields() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          {
            field(arg: {
              deep: {
                deep: {
                  id: 1
                }
                id: 1
              }
              id: 1
            })
          }
        "#,
        );
    }

    #[test]
    fn duplicate_input_object_fields() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          {
            field(arg: { f1: "value", f1: "value" })
          }
        "#,
            &[RuleError::new(
                &error_message("f1"),
                &[
                    SourcePosition::new(38, 2, 25),
                    SourcePosition::new(51, 2, 38),
                ],
            )],
        );
    }

    #[test]
    fn many_duplicate_input_object_fields() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          {
            field(arg: { f1: "value", f1: "value", f1: "value" })
          }
        "#,
            &[
                RuleError::new(
                    &error_message("f1"),
                    &[
                        SourcePosition::new(38, 2, 25),
                        SourcePosition::new(51, 2, 38),
                    ],
                ),
                RuleError::new(
                    &error_message("f1"),
                    &[
                        SourcePosition::new(38, 2, 25),
                        SourcePosition::new(64, 2, 51),
                    ],
                ),
            ],
        );
    }
}
