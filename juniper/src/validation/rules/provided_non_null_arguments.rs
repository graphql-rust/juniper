use std::fmt;

use crate::{
    ast::{Directive, Field},
    parser::Spanning,
    schema::{meta::Field as FieldType, model::DirectiveType},
    validation::{ValidatorContext, Visitor},
    value::ScalarValue,
};

pub struct ProvidedNonNullArguments;

pub fn factory() -> ProvidedNonNullArguments {
    ProvidedNonNullArguments
}

impl<'a, S> Visitor<'a, S> for ProvidedNonNullArguments
where
    S: ScalarValue,
{
    fn enter_field(&mut self, ctx: &mut ValidatorContext<'a, S>, field: &'a Spanning<Field<S>>) {
        let field_name = &field.item.name.item;

        if let Some(&FieldType {
            arguments: Some(ref meta_args),
            ..
        }) = ctx.parent_type().and_then(|t| t.field_by_name(field_name))
        {
            for meta_arg in meta_args {
                if meta_arg.arg_type.is_non_null()
                    && meta_arg.default_value.is_none()
                    && field
                        .item
                        .arguments
                        .as_ref()
                        .and_then(|args| args.item.get(&meta_arg.name))
                        .is_none()
                {
                    ctx.report_error(
                        &field_error_message(field_name, &meta_arg.name, &meta_arg.arg_type),
                        &[field.span.start],
                    );
                }
            }
        }
    }

    fn enter_directive(
        &mut self,
        ctx: &mut ValidatorContext<'a, S>,
        directive: &'a Spanning<Directive<S>>,
    ) {
        let directive_name = &directive.item.name.item;

        if let Some(DirectiveType {
            arguments: meta_args,
            ..
        }) = ctx.schema.directive_by_name(directive_name)
        {
            for meta_arg in meta_args {
                if meta_arg.arg_type.is_non_null()
                    && directive
                        .item
                        .arguments
                        .as_ref()
                        .and_then(|args| args.item.get(&meta_arg.name))
                        .is_none()
                {
                    ctx.report_error(
                        &directive_error_message(
                            directive_name,
                            &meta_arg.name,
                            &meta_arg.arg_type,
                        ),
                        &[directive.span.start],
                    );
                }
            }
        }
    }
}

fn field_error_message(
    field_name: impl fmt::Display,
    arg_name: impl fmt::Display,
    type_name: impl fmt::Display,
) -> String {
    format!(
        r#"Field "{field_name}" argument "{arg_name}" of type "{type_name}" is required but not provided"#,
    )
}

fn directive_error_message(
    directive_name: impl fmt::Display,
    arg_name: impl fmt::Display,
    type_name: impl fmt::Display,
) -> String {
    format!(
        r#"Directive "@{directive_name}" argument "{arg_name}" of type "{type_name}" is required but not provided"#,
    )
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
    fn ignores_unknown_arguments() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          {
            dog {
              isHousetrained(unknownArgument: true)
            }
          }
        "#,
        );
    }

    #[test]
    fn arg_on_optional_arg() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
            {
              dog {
                isHousetrained(atOtherHomes: true)
              }
            }
        "#,
        );
    }

    #[test]
    fn no_arg_on_optional_arg() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
            {
              dog {
                isHousetrained
              }
            }
        "#,
        );
    }

    #[test]
    fn multiple_args() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
            {
              complicatedArgs {
                multipleReqs(req1: 1, req2: 2)
              }
            }
        "#,
        );
    }

    #[test]
    fn multiple_args_reverse_order() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
            {
              complicatedArgs {
                multipleReqs(req2: 2, req1: 1)
              }
            }
        "#,
        );
    }

    #[test]
    fn no_args_on_multiple_optional() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
            {
              complicatedArgs {
                multipleOpts
              }
            }
        "#,
        );
    }

    #[test]
    fn one_arg_on_multiple_optional() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
            {
              complicatedArgs {
                multipleOpts(opt1: 1)
              }
            }
        "#,
        );
    }

    #[test]
    fn second_arg_on_multiple_optional() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
            {
              complicatedArgs {
                multipleOpts(opt2: 1)
              }
            }
        "#,
        );
    }

    #[test]
    fn muliple_reqs_on_mixed_list() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
            {
              complicatedArgs {
                multipleOptAndReq(req1: 3, req2: 4)
              }
            }
        "#,
        );
    }

    #[test]
    fn multiple_reqs_and_one_opt_on_mixed_list() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
            {
              complicatedArgs {
                multipleOptAndReq(req1: 3, req2: 4, opt1: 5)
              }
            }
        "#,
        );
    }

    #[test]
    fn all_reqs_on_opts_on_mixed_list() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
            {
              complicatedArgs {
                multipleOptAndReq(req1: 3, req2: 4, opt1: 5, opt2: 6)
              }
            }
        "#,
        );
    }

    #[test]
    fn missing_one_non_nullable_argument() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
            {
              complicatedArgs {
                multipleReqs(req2: 2)
              }
            }
        "#,
            &[RuleError::new(
                &field_error_message("multipleReqs", "req1", "Int!"),
                &[SourcePosition::new(63, 3, 16)],
            )],
        );
    }

    #[test]
    fn missing_multiple_non_nullable_arguments() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
            {
              complicatedArgs {
                multipleReqs
              }
            }
        "#,
            &[
                RuleError::new(
                    &field_error_message("multipleReqs", "req1", "Int!"),
                    &[SourcePosition::new(63, 3, 16)],
                ),
                RuleError::new(
                    &field_error_message("multipleReqs", "req2", "Int!"),
                    &[SourcePosition::new(63, 3, 16)],
                ),
            ],
        );
    }

    #[test]
    fn incorrect_value_and_missing_argument() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
            {
              complicatedArgs {
                multipleReqs(req1: "one")
              }
            }
        "#,
            &[RuleError::new(
                &field_error_message("multipleReqs", "req2", "Int!"),
                &[SourcePosition::new(63, 3, 16)],
            )],
        );
    }

    #[test]
    fn ignores_unknown_directives() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
            {
              dog @unknown
            }
        "#,
        );
    }

    #[test]
    fn with_directives_of_valid_types() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
            {
              dog @include(if: true) {
                name
              }
              human @skip(if: false) {
                name
              }
            }
        "#,
        );
    }

    #[test]
    fn with_directive_with_missing_types() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
            {
              dog @include {
                name @skip
              }
            }
        "#,
            &[
                RuleError::new(
                    &directive_error_message("include", "if", "Boolean!"),
                    &[SourcePosition::new(33, 2, 18)],
                ),
                RuleError::new(
                    &directive_error_message("skip", "if", "Boolean!"),
                    &[SourcePosition::new(65, 3, 21)],
                ),
            ],
        );
    }
}
