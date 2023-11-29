use crate::{
    ast::{Definition, Document, Operation},
    parser::Spanning,
    validation::{ValidatorContext, Visitor},
    value::ScalarValue,
};

pub struct LoneAnonymousOperation {
    operation_count: Option<usize>,
}

pub fn factory() -> LoneAnonymousOperation {
    LoneAnonymousOperation {
        operation_count: None,
    }
}

impl<'a, S> Visitor<'a, S> for LoneAnonymousOperation
where
    S: ScalarValue,
{
    fn enter_document(&mut self, _: &mut ValidatorContext<'a, S>, doc: &'a Document<S>) {
        self.operation_count = Some(
            doc.iter()
                .filter(|d| match **d {
                    Definition::Operation(_) => true,
                    Definition::Fragment(_) => false,
                })
                .count(),
        );
    }

    fn enter_operation_definition(
        &mut self,
        ctx: &mut ValidatorContext<'a, S>,
        op: &'a Spanning<Operation<S>>,
    ) {
        if let Some(operation_count) = self.operation_count {
            if operation_count > 1 && op.item.name.is_none() {
                ctx.report_error(error_message(), &[op.span.start]);
            }
        }
    }
}

fn error_message() -> &'static str {
    "This anonymous operation must be the only defined operation"
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
          fragment fragA on Type {
            field
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
    fn multiple_named_operations() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          query Foo {
            field
          }

          query Bar {
            field
          }
        "#,
        );
    }

    #[test]
    fn anon_operation_with_fragment() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          {
            ...Foo
          }
          fragment Foo on Type {
            field
          }
        "#,
        );
    }

    #[test]
    fn multiple_anon_operations() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          {
            fieldA
          }
          {
            fieldB
          }
        "#,
            &[
                RuleError::new(error_message(), &[SourcePosition::new(11, 1, 10)]),
                RuleError::new(error_message(), &[SourcePosition::new(54, 4, 10)]),
            ],
        );
    }

    #[test]
    fn anon_operation_with_a_mutation() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          {
            fieldA
          }
          mutation Foo {
            fieldB
          }
        "#,
            &[RuleError::new(
                error_message(),
                &[SourcePosition::new(11, 1, 10)],
            )],
        );
    }
}
