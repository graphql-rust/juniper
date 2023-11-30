use std::fmt;

use crate::{
    ast::VariableDefinition,
    parser::Spanning,
    types::utilities::validate_literal_value,
    validation::{ValidatorContext, Visitor},
    value::ScalarValue,
};

pub struct DefaultValuesOfCorrectType;

pub fn factory() -> DefaultValuesOfCorrectType {
    DefaultValuesOfCorrectType
}

impl<'a, S> Visitor<'a, S> for DefaultValuesOfCorrectType
where
    S: ScalarValue,
{
    fn enter_variable_definition(
        &mut self,
        ctx: &mut ValidatorContext<'a, S>,
        (var_name, var_def): &'a (Spanning<&'a str>, VariableDefinition<S>),
    ) {
        if let Some(Spanning {
            item: ref var_value,
            ref span,
        }) = var_def.default_value
        {
            if var_def.var_type.item.is_non_null() {
                ctx.report_error(
                    &non_null_error_message(var_name.item, &var_def.var_type.item),
                    &[span.start],
                )
            } else {
                let meta_type = ctx.schema.make_type(&var_def.var_type.item);

                if let Some(err) = validate_literal_value(ctx.schema, &meta_type, var_value) {
                    ctx.report_error(
                        &type_error_message(var_name.item, &var_def.var_type.item, err),
                        &[span.start],
                    );
                }
            }
        }
    }
}

fn type_error_message(
    arg_name: impl fmt::Display,
    type_name: impl fmt::Display,
    reason: impl fmt::Display,
) -> String {
    format!(
        "Invalid default value for argument \"{arg_name}\", expected type \"{type_name}\", \
         reason: {reason}",
    )
}

fn non_null_error_message(arg_name: impl fmt::Display, type_name: impl fmt::Display) -> String {
    format!(
        "Argument \"{arg_name}\" has type \"{type_name}\" and is not nullable, \
         so it can't have a default value",
    )
}

#[cfg(test)]
mod tests {
    use super::{factory, non_null_error_message, type_error_message};

    use crate::{
        parser::SourcePosition,
        types::utilities::error,
        validation::{expect_fails_rule, expect_passes_rule, RuleError},
        value::DefaultScalarValue,
    };

    #[test]
    fn variables_with_no_default_values() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
            query NullableValues($a: Int, $b: String, $c: ComplexInput) {
                dog { name }
            }
            "#,
        );
    }

    #[test]
    fn required_variables_without_default_values() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
            query RequiredValues($a: Int!, $b: String!) {
                dog { name }
            }
            "#,
        );
    }

    #[test]
    fn variables_with_valid_default_values() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
            query WithDefaultValues(
                $a: Int = 1,
                $b: String = "ok",
                $c: ComplexInput = { requiredField: true, intField: 3 }
            ) {
                dog { name }
            }
            "#,
        );
    }

    #[test]
    fn no_required_variables_with_default_values() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
            query UnreachableDefaultValues($a: Int! = 3, $b: String! = "default") {
                dog { name }
            }
            "#,
            &[
                RuleError::new(
                    &non_null_error_message("a", "Int!"),
                    &[SourcePosition::new(55, 1, 54)],
                ),
                RuleError::new(
                    &non_null_error_message("b", "String!"),
                    &[SourcePosition::new(72, 1, 71)],
                ),
            ],
        );
    }

    #[test]
    fn variables_with_invalid_default_values() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
            query InvalidDefaultValues(
                $a: Int = "one",
                $b: String = 4,
                $c: ComplexInput = "notverycomplex"
            ) {
                dog { name }
            }
            "#,
            &[
                RuleError::new(
                    &type_error_message("a", "Int", error::type_value("\"one\"", "Int")),
                    &[SourcePosition::new(67, 2, 26)],
                ),
                RuleError::new(
                    &type_error_message("b", "String", error::type_value("4", "String")),
                    &[SourcePosition::new(103, 3, 29)],
                ),
                RuleError::new(
                    &type_error_message(
                        "c",
                        "ComplexInput",
                        error::type_value("\"notverycomplex\"", "ComplexInput"),
                    ),
                    &[SourcePosition::new(141, 4, 35)],
                ),
            ],
        );
    }

    #[test]
    fn complex_variables_missing_required_field() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
            query MissingRequiredField($a: ComplexInput = {intField: 3}) {
                dog { name }
            }
            "#,
            &[RuleError::new(
                &type_error_message(
                    "a",
                    "ComplexInput",
                    error::missing_fields("ComplexInput", "\"requiredField\""),
                ),
                &[SourcePosition::new(59, 1, 58)],
            )],
        );
    }

    #[test]
    fn list_variables_with_invalid_item() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
            query InvalidItem($a: [String] = ["one", 2]) {
                dog { name }
            }
            "#,
            &[RuleError::new(
                &type_error_message("a", "[String]", error::type_value("2", "String")),
                &[SourcePosition::new(46, 1, 45)],
            )],
        );
    }
}
