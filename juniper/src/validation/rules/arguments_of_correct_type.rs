use std::fmt;

use crate::{
    ast::{Directive, Field, InputValue},
    parser::Spanning,
    schema::meta::Argument,
    types::utilities::validate_literal_value,
    validation::{ValidatorContext, Visitor},
    value::ScalarValue,
};

pub struct ArgumentsOfCorrectType<'a, S: fmt::Debug + 'a> {
    current_args: Option<&'a Vec<Argument<S>>>,
}

pub fn factory<'a, S: fmt::Debug>() -> ArgumentsOfCorrectType<'a, S> {
    ArgumentsOfCorrectType { current_args: None }
}

impl<'a, S> Visitor<'a, S> for ArgumentsOfCorrectType<'a, S>
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
            .map(|d| &d.arguments);
    }

    fn exit_directive(&mut self, _: &mut ValidatorContext<'a, S>, _: &'a Spanning<Directive<S>>) {
        self.current_args = None;
    }

    fn enter_field(&mut self, ctx: &mut ValidatorContext<'a, S>, field: &'a Spanning<Field<S>>) {
        self.current_args = ctx
            .parent_type()
            .and_then(|t| t.field_by_name(field.item.name.item))
            .and_then(|f| f.arguments.as_ref());
    }

    fn exit_field(&mut self, _: &mut ValidatorContext<'a, S>, _: &'a Spanning<Field<S>>) {
        self.current_args = None;
    }

    fn enter_argument(
        &mut self,
        ctx: &mut ValidatorContext<'a, S>,
        (arg_name, arg_value): &'a (Spanning<&'a str>, Spanning<InputValue<S>>),
    ) {
        if let Some(argument_meta) = self
            .current_args
            .and_then(|args| args.iter().find(|a| a.name == arg_name.item))
        {
            let meta_type = ctx.schema.make_type(&argument_meta.arg_type);

            if let Some(err) = validate_literal_value(ctx.schema, &meta_type, &arg_value.item) {
                ctx.report_error(&error_message(arg_name.item, err), &[arg_value.span.start]);
            }
        }
    }
}

fn error_message(arg_name: impl fmt::Display, msg: impl fmt::Display) -> String {
    format!("Invalid value for argument \"{arg_name}\", reason: {msg}")
}

#[cfg(test)]
mod tests {
    use super::{error_message, factory};

    use crate::{
        parser::SourcePosition,
        types::utilities::error,
        validation::{RuleError, expect_fails_rule, expect_passes_rule},
        value::DefaultScalarValue,
    };

    #[test]
    fn null_into_nullable_int() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
            {
              complicatedArgs {
                intArgField(intArg: null)
              }
            }
            "#,
        );
    }

    #[test]
    fn null_into_nullable_list() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
            {
              complicatedArgs {
                stringListArgField(stringListArg: null)
              }
            }
            "#,
        );
    }

    #[test]
    fn null_into_int() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
            {
              complicatedArgs {
                nonNullIntArgField(nonNullIntArg: null)
              }
            }
            "#,
            &[RuleError::new(
                &error_message("nonNullIntArg", error::non_null("Int!")),
                &[SourcePosition::new(97, 3, 50)],
            )],
        );
    }

    #[test]
    fn null_into_list() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
            {
              complicatedArgs {
                nonNullStringListArgField(nonNullStringListArg: null)
              }
            }
            "#,
            &[RuleError::new(
                &error_message("nonNullStringListArg", error::non_null("[String!]!")),
                &[SourcePosition::new(111, 3, 64)],
            )],
        );
    }

    #[test]
    fn good_int_value() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
            {
              complicatedArgs {
                intArgField(intArg: 2)
              }
            }
            "#,
        );
    }

    #[test]
    fn good_boolean_value() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
            {
              complicatedArgs {
                booleanArgField(booleanArg: true)
              }
            }
            "#,
        );
    }

    #[test]
    fn good_string_value() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
            {
              complicatedArgs {
                stringArgField(stringArg: "foo")
              }
            }
            "#,
        );
    }

    #[test]
    fn good_float_value() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
            {
              complicatedArgs {
                floatArgField(floatArg: 1.1)
              }
            }
            "#,
        );
    }

    #[test]
    fn int_into_float() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
            {
              complicatedArgs {
                floatArgField(floatArg: 1)
              }
            }
            "#,
        );
    }

    #[test]
    fn int_into_id() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
            {
              complicatedArgs {
                idArgField(idArg: 1)
              }
            }
            "#,
        );
    }

    #[test]
    fn string_into_id() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
            {
              complicatedArgs {
                idArgField(idArg: "someIdString")
              }
            }
            "#,
        );
    }

    #[test]
    fn good_enum_value() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
            {
              dog {
                doesKnowCommand(dogCommand: SIT)
              }
            }
            "#,
        );
    }

    #[test]
    fn int_into_string() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
            {
              complicatedArgs {
                stringArgField(stringArg: 1)
              }
            }
            "#,
            &[RuleError::new(
                &error_message("stringArg", error::type_value("1", "String")),
                &[SourcePosition::new(89, 3, 42)],
            )],
        );
    }

    #[test]
    fn float_into_string() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
            {
              complicatedArgs {
                stringArgField(stringArg: 1.0)
              }
            }
            "#,
            &[RuleError::new(
                &error_message("stringArg", error::type_value("1", "String")),
                &[SourcePosition::new(89, 3, 42)],
            )],
        );
    }

    #[test]
    fn boolean_into_string() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
            {
              complicatedArgs {
                stringArgField(stringArg: true)
              }
            }
            "#,
            &[RuleError::new(
                &error_message("stringArg", error::type_value("true", "String")),
                &[SourcePosition::new(89, 3, 42)],
            )],
        );
    }

    #[test]
    fn unquoted_string_into_string() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
            {
              complicatedArgs {
                stringArgField(stringArg: BAR)
              }
            }
            "#,
            &[RuleError::new(
                &error_message("stringArg", error::type_value("BAR", "String")),
                &[SourcePosition::new(89, 3, 42)],
            )],
        );
    }

    #[test]
    fn string_into_int() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
            {
              complicatedArgs {
                intArgField(intArg: "3")
              }
            }
            "#,
            &[RuleError::new(
                &error_message("intArg", error::type_value("\"3\"", "Int")),
                &[SourcePosition::new(83, 3, 36)],
            )],
        );
    }

    #[test]
    fn unquoted_string_into_int() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
            {
              complicatedArgs {
                intArgField(intArg: FOO)
              }
            }
            "#,
            &[RuleError::new(
                &error_message("intArg", error::type_value("FOO", "Int")),
                &[SourcePosition::new(83, 3, 36)],
            )],
        );
    }

    #[test]
    fn simple_float_into_int() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
            {
              complicatedArgs {
                intArgField(intArg: 3.0)
              }
            }
            "#,
            &[RuleError::new(
                &error_message("intArg", error::type_value("3", "Int")),
                &[SourcePosition::new(83, 3, 36)],
            )],
        );
    }

    #[test]
    fn float_into_int() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
            {
              complicatedArgs {
                intArgField(intArg: 3.333)
              }
            }
            "#,
            &[RuleError::new(
                &error_message("intArg", error::type_value("3.333", "Int")),
                &[SourcePosition::new(83, 3, 36)],
            )],
        );
    }

    #[test]
    fn string_into_float() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
            {
              complicatedArgs {
                floatArgField(floatArg: "3.333")
              }
            }
            "#,
            &[RuleError::new(
                &error_message("floatArg", error::type_value("\"3.333\"", "Float")),
                &[SourcePosition::new(87, 3, 40)],
            )],
        );
    }

    #[test]
    fn boolean_into_float() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
            {
              complicatedArgs {
                floatArgField(floatArg: true)
              }
            }
            "#,
            &[RuleError::new(
                &error_message("floatArg", error::type_value("true", "Float")),
                &[SourcePosition::new(87, 3, 40)],
            )],
        );
    }

    #[test]
    fn unquoted_into_float() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
            {
              complicatedArgs {
                floatArgField(floatArg: FOO)
              }
            }
            "#,
            &[RuleError::new(
                &error_message("floatArg", error::type_value("FOO", "Float")),
                &[SourcePosition::new(87, 3, 40)],
            )],
        );
    }

    #[test]
    fn int_into_boolean() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
            {
              complicatedArgs {
                booleanArgField(booleanArg: 2)
              }
            }
            "#,
            &[RuleError::new(
                &error_message("booleanArg", error::type_value("2", "Boolean")),
                &[SourcePosition::new(91, 3, 44)],
            )],
        );
    }

    #[test]
    fn float_into_boolean() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
            {
              complicatedArgs {
                booleanArgField(booleanArg: 1.0)
              }
            }
            "#,
            &[RuleError::new(
                &error_message("booleanArg", error::type_value("1", "Boolean")),
                &[SourcePosition::new(91, 3, 44)],
            )],
        );
    }

    #[test]
    fn string_into_boolean() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
            {
              complicatedArgs {
                booleanArgField(booleanArg: "true")
              }
            }
            "#,
            &[RuleError::new(
                &error_message("booleanArg", error::type_value("\"true\"", "Boolean")),
                &[SourcePosition::new(91, 3, 44)],
            )],
        );
    }

    #[test]
    fn unquoted_into_boolean() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
            {
              complicatedArgs {
                booleanArgField(booleanArg: TRUE)
              }
            }
            "#,
            &[RuleError::new(
                &error_message("booleanArg", error::type_value("TRUE", "Boolean")),
                &[SourcePosition::new(91, 3, 44)],
            )],
        );
    }

    #[test]
    fn float_into_id() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
            {
              complicatedArgs {
                idArgField(idArg: 1.0)
              }
            }
            "#,
            &[RuleError::new(
                &error_message("idArg", error::type_value("1", "ID")),
                &[SourcePosition::new(81, 3, 34)],
            )],
        );
    }

    #[test]
    fn boolean_into_id() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
            {
              complicatedArgs {
                idArgField(idArg: true)
              }
            }
            "#,
            &[RuleError::new(
                &error_message("idArg", error::type_value("true", "ID")),
                &[SourcePosition::new(81, 3, 34)],
            )],
        );
    }

    #[test]
    fn unquoted_into_id() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
            {
              complicatedArgs {
                idArgField(idArg: SOMETHING)
              }
            }
            "#,
            &[RuleError::new(
                &error_message("idArg", error::type_value("SOMETHING", "ID")),
                &[SourcePosition::new(81, 3, 34)],
            )],
        );
    }

    #[test]
    fn int_into_enum() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
            {
              dog {
                doesKnowCommand(dogCommand: 2)
              }
            }
            "#,
            &[RuleError::new(
                &error_message("dogCommand", error::enum_value("2", "DogCommand")),
                &[SourcePosition::new(79, 3, 44)],
            )],
        );
    }

    #[test]
    fn float_into_enum() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
            {
              dog {
                doesKnowCommand(dogCommand: 1.0)
              }
            }
            "#,
            &[RuleError::new(
                &error_message("dogCommand", error::enum_value("1", "DogCommand")),
                &[SourcePosition::new(79, 3, 44)],
            )],
        );
    }

    #[test]
    fn string_into_enum() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
            {
              dog {
                doesKnowCommand(dogCommand: "SIT")
              }
            }
            "#,
            &[RuleError::new(
                &error_message("dogCommand", error::enum_value("\"SIT\"", "DogCommand")),
                &[SourcePosition::new(79, 3, 44)],
            )],
        );
    }

    #[test]
    fn boolean_into_enum() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
            {
              dog {
                doesKnowCommand(dogCommand: true)
              }
            }
            "#,
            &[RuleError::new(
                &error_message("dogCommand", error::enum_value("true", "DogCommand")),
                &[SourcePosition::new(79, 3, 44)],
            )],
        );
    }

    #[test]
    fn unknown_enum_value_into_enum() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
            {
              dog {
                doesKnowCommand(dogCommand: JUGGLE)
              }
            }
            "#,
            &[RuleError::new(
                &error_message("dogCommand", error::type_value("JUGGLE", "DogCommand")),
                &[SourcePosition::new(79, 3, 44)],
            )],
        );
    }

    #[test]
    fn different_case_enum_value_into_enum() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
            {
              dog {
                doesKnowCommand(dogCommand: sit)
              }
            }
            "#,
            &[RuleError::new(
                &error_message("dogCommand", error::type_value("sit", "DogCommand")),
                &[SourcePosition::new(79, 3, 44)],
            )],
        );
    }

    #[test]
    fn good_list_value() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
            {
              complicatedArgs {
                stringListArgField(stringListArg: ["one", "two"])
              }
            }
            "#,
        );
    }

    #[test]
    fn empty_list_value() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
            {
              complicatedArgs {
                stringListArgField(stringListArg: [])
              }
            }
            "#,
        );
    }

    #[test]
    fn single_value_into_list() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
            {
              complicatedArgs {
                stringListArgField(stringListArg: "one")
              }
            }
            "#,
        );
    }

    #[test]
    fn incorrect_item_type() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
            {
              complicatedArgs {
                stringListArgField(stringListArg: ["one", 2])
              }
            }
            "#,
            &[RuleError::new(
                &error_message("stringListArg", error::type_value("2", "String")),
                &[SourcePosition::new(97, 3, 50)],
            )],
        );
    }

    #[test]
    fn single_value_of_incorrect_type() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
            {
              complicatedArgs {
                stringListArgField(stringListArg: 1)
              }
            }
            "#,
            &[RuleError::new(
                &error_message("stringListArg", error::type_value("1", "String")),
                &[SourcePosition::new(97, 3, 50)],
            )],
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
    fn multiple_reqs_on_mixed_list() {
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
    fn all_reqs_and_opts_on_mixed_list() {
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
    fn incorrect_value_type() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
            {
              complicatedArgs {
                multipleReqs(req2: "two", req1: "one")
              }
            }
            "#,
            &[
                RuleError::new(
                    &error_message("req2", error::type_value("\"two\"", "Int")),
                    &[SourcePosition::new(82, 3, 35)],
                ),
                RuleError::new(
                    &error_message("req1", error::type_value("\"one\"", "Int")),
                    &[SourcePosition::new(95, 3, 48)],
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
                &error_message("req1", error::type_value("\"one\"", "Int")),
                &[SourcePosition::new(82, 3, 35)],
            )],
        );
    }

    #[test]
    fn optional_arg_despite_required_field_in_type() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
            {
              complicatedArgs {
                complexArgField
              }
            }
            "#,
        );
    }

    #[test]
    fn partial_object_only_required() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
            {
              complicatedArgs {
                complexArgField(complexArg: { requiredField: true })
              }
            }
            "#,
        );
    }

    #[test]
    fn partial_object_required_field_can_be_falsy() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
            {
              complicatedArgs {
                complexArgField(complexArg: { requiredField: false })
              }
            }
            "#,
        );
    }

    #[test]
    fn partial_object_including_required() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
            {
              complicatedArgs {
                complexArgField(complexArg: { requiredField: true, intField: 4 })
              }
            }
            "#,
        );
    }

    #[test]
    fn full_object() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
            {
              complicatedArgs {
                complexArgField(complexArg: {
                  requiredField: true,
                  intField: 4,
                  stringField: "foo",
                  booleanField: false,
                  stringListField: ["one", "two"]
                })
              }
            }
            "#,
        );
    }

    #[test]
    fn full_object_with_fields_in_different_order() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
            {
              complicatedArgs {
                complexArgField(complexArg: {
                  stringListField: ["one", "two"],
                  booleanField: false,
                  requiredField: true,
                  stringField: "foo",
                  intField: 4,
                })
              }
            }
            "#,
        );
    }

    #[test]
    fn partial_object_missing_required() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
            {
              complicatedArgs {
                complexArgField(complexArg: { intField: 4 })
              }
            }
            "#,
            &[RuleError::new(
                &error_message(
                    "complexArg",
                    error::missing_fields("ComplexInput", "\"requiredField\""),
                ),
                &[SourcePosition::new(91, 3, 44)],
            )],
        );
    }

    #[test]
    fn partial_object_invalid_field_type() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
            {
              complicatedArgs {
                complexArgField(complexArg: {
                  stringListField: ["one", 2],
                  requiredField: true,
                })
              }
            }
            "#,
            &[RuleError::new(
                &error_message(
                    "complexArg",
                    error::field(
                        "ComplexInput",
                        "stringListField",
                        error::type_value("2", "String"),
                    ),
                ),
                &[SourcePosition::new(91, 3, 44)],
            )],
        );
    }

    #[test]
    fn partial_object_unknown_field_arg() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
            {
              complicatedArgs {
                complexArgField(complexArg: {
                  requiredField: true,
                  unknownField: "value"
                })
              }
            }
            "#,
            &[RuleError::new(
                &error_message(
                    "complexArg",
                    error::unknown_field("ComplexInput", "unknownField"),
                ),
                &[SourcePosition::new(91, 3, 44)],
            )],
        );
    }

    #[test]
    fn directive_with_valid_types() {
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
    fn directive_with_incorrect_types() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
            {
              dog @include(if: "yes") {
                name @skip(if: ENUM)
              }
            }
            "#,
            &[
                RuleError::new(
                    &error_message("if", error::type_value("\"yes\"", "Boolean")),
                    &[SourcePosition::new(46, 2, 31)],
                ),
                RuleError::new(
                    &error_message("if", error::type_value("ENUM", "Boolean")),
                    &[SourcePosition::new(86, 3, 31)],
                ),
            ],
        );
    }
}
