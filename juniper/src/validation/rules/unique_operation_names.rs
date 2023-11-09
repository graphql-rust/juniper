use std::collections::hash_map::{Entry, HashMap};

use crate::{
    ast::Operation,
    parser::{SourcePosition, Spanning},
    validation::{ValidatorContext, Visitor},
    value::ScalarValue,
};

pub struct UniqueOperationNames<'a> {
    names: HashMap<&'a str, SourcePosition>,
}

pub fn factory<'a>() -> UniqueOperationNames<'a> {
    UniqueOperationNames {
        names: HashMap::new(),
    }
}

impl<'a, S> Visitor<'a, S> for UniqueOperationNames<'a>
where
    S: ScalarValue,
{
    fn enter_operation_definition(
        &mut self,
        ctx: &mut ValidatorContext<'a, S>,
        op: &'a Spanning<Operation<S>>,
    ) {
        if let Some(ref op_name) = op.item.name {
            match self.names.entry(op_name.item) {
                Entry::Occupied(e) => {
                    ctx.report_error(&error_message(op_name.item), &[*e.get(), op.span.start]);
                }
                Entry::Vacant(e) => {
                    e.insert(op.span.start);
                }
            }
        }
    }
}

fn error_message(op_name: &str) -> String {
    format!("There can only be one operation named {op_name}")
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
    fn no_operations() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment fragA on Dog {
            name
          }
        "#,
        );
    }

    #[test]
    fn one_anon_operation() {
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
    fn one_named_operation() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          query Foo {
            field
          }
        "#,
        );
    }

    #[test]
    fn multiple_operations() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          query Foo {
            dog {
              name
            }
          }

          query Bar {
            dog {
              name
            }
          }
        "#,
        );
    }

    #[test]
    fn multiple_operations_of_different_types() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          query Foo {
            field
          }

          mutation Bar {
            field
          }
        "#,
        );
    }

    #[test]
    fn fragment_and_operation_named_the_same() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          query Foo {
            dog {
              ...Foo
            }
          }
          fragment Foo on Dog {
            name
          }
        "#,
        );
    }

    #[test]
    fn multiple_operations_of_same_name() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          query Foo {
            dog {
              name
            }
          }
          query Foo {
            human {
              name
            }
          }
        "#,
            &[RuleError::new(
                &error_message("Foo"),
                &[
                    SourcePosition::new(11, 1, 10),
                    SourcePosition::new(96, 6, 10),
                ],
            )],
        );
    }

    #[test]
    fn multiple_ops_of_same_name_of_different_types() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          query Foo {
            dog {
              name
            }
          }
          mutation Foo {
            testInput
          }
        "#,
            &[RuleError::new(
                &error_message("Foo"),
                &[
                    SourcePosition::new(11, 1, 10),
                    SourcePosition::new(96, 6, 10),
                ],
            )],
        );
    }
}
