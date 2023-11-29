use crate::{
    ast::Field,
    parser::Spanning,
    schema::meta::MetaType,
    validation::{ValidatorContext, Visitor},
    value::ScalarValue,
    Operation, OperationType, Selection,
};

pub struct FieldsOnCorrectType;

pub fn factory() -> FieldsOnCorrectType {
    FieldsOnCorrectType
}

impl<'a, S> Visitor<'a, S> for FieldsOnCorrectType
where
    S: ScalarValue,
{
    fn enter_operation_definition(
        &mut self,
        context: &mut ValidatorContext<'a, S>,
        operation: &'a Spanning<Operation<S>>,
    ) {
        // https://spec.graphql.org/October2021/#note-bc213
        if let OperationType::Subscription = operation.item.operation_type {
            for selection in &operation.item.selection_set {
                if let Selection::Field(field) = selection {
                    if field.item.name.item == "__typename" {
                        context.report_error(
                            "`__typename` may not be included as a root \
                             field in a subscription operation",
                            &[field.item.name.span.start],
                        );
                    }
                }
            }
        }
    }

    fn enter_field(
        &mut self,
        context: &mut ValidatorContext<'a, S>,
        field: &'a Spanning<Field<S>>,
    ) {
        {
            if let Some(parent_type) = context.parent_type() {
                let field_name = &field.item.name;
                let type_name = parent_type.name().unwrap_or("<unknown>");

                if parent_type.field_by_name(field_name.item).is_none() {
                    if let MetaType::Union(..) = *parent_type {
                        // You can query for `__typename` on a union,
                        // but it isn't a field on the union...it is
                        // instead on the resulting object returned.
                        if field_name.item == "__typename" {
                            return;
                        }
                    }

                    context.report_error(
                        &error_message(field_name.item, type_name),
                        &[field_name.span.start],
                    );
                }
            }
        }
    }
}

fn error_message(field: &str, type_name: &str) -> String {
    format!(r#"Unknown field "{field}" on type "{type_name}""#)
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
    fn selection_on_object() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment objectFieldSelection on Dog {
            __typename
            name
          }
        "#,
        );
    }

    #[test]
    fn aliased_selection_on_object() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment aliasedObjectFieldSelection on Dog {
            tn : __typename
            otherName : name
          }
        "#,
        );
    }

    #[test]
    fn selection_on_interface() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment interfaceFieldSelection on Pet {
            __typename
            name
          }
        "#,
        );
    }

    #[test]
    fn aliased_selection_on_interface() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment interfaceFieldSelection on Pet {
            otherName : name
          }
        "#,
        );
    }

    #[test]
    fn lying_alias_selection() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment lyingAliasSelection on Dog {
            name : nickname
          }
        "#,
        );
    }

    #[test]
    fn ignores_unknown_type() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment unknownSelection on UnknownType {
            unknownField
          }
        "#,
        );
    }

    #[test]
    fn nested_unknown_fields() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment typeKnownAgain on Pet {
            unknown_pet_field {
              ... on Cat {
                unknown_cat_field
              }
            }
          }
        "#,
            &[
                RuleError::new(
                    &error_message("unknown_pet_field", "Pet"),
                    &[SourcePosition::new(56, 2, 12)],
                ),
                RuleError::new(
                    &error_message("unknown_cat_field", "Cat"),
                    &[SourcePosition::new(119, 4, 16)],
                ),
            ],
        );
    }

    #[test]
    fn unknown_field_on_fragment() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment fieldNotDefined on Dog {
            meowVolume
          }
        "#,
            &[RuleError::new(
                &error_message("meowVolume", "Dog"),
                &[SourcePosition::new(57, 2, 12)],
            )],
        );
    }

    #[test]
    fn ignores_deeply_unknown_field() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment deepFieldNotDefined on Dog {
            unknown_field {
              deeper_unknown_field
            }
          }
        "#,
            &[RuleError::new(
                &error_message("unknown_field", "Dog"),
                &[SourcePosition::new(61, 2, 12)],
            )],
        );
    }

    #[test]
    fn unknown_subfield() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment subFieldNotDefined on Human {
            pets {
              unknown_field
            }
          }
        "#,
            &[RuleError::new(
                &error_message("unknown_field", "Pet"),
                &[SourcePosition::new(83, 3, 14)],
            )],
        );
    }

    #[test]
    fn unknown_field_on_inline_fragment() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment fieldNotDefined on Pet {
            ... on Dog {
              meowVolume
            }
          }
        "#,
            &[RuleError::new(
                &error_message("meowVolume", "Dog"),
                &[SourcePosition::new(84, 3, 14)],
            )],
        );
    }

    #[test]
    fn unknown_aliased_target() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment aliasedFieldTargetNotDefined on Dog {
            volume : mooVolume
          }
        "#,
            &[RuleError::new(
                &error_message("mooVolume", "Dog"),
                &[SourcePosition::new(79, 2, 21)],
            )],
        );
    }

    #[test]
    fn unknown_aliased_lying_field_target() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment aliasedLyingFieldTargetNotDefined on Dog {
            barkVolume : kawVolume
          }
        "#,
            &[RuleError::new(
                &error_message("kawVolume", "Dog"),
                &[SourcePosition::new(88, 2, 25)],
            )],
        );
    }

    #[test]
    fn not_defined_on_interface() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment notDefinedOnInterface on Pet {
            tailLength
          }
        "#,
            &[RuleError::new(
                &error_message("tailLength", "Pet"),
                &[SourcePosition::new(63, 2, 12)],
            )],
        );
    }

    #[test]
    fn defined_in_concrete_types_but_not_interface() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment definedOnImplementorsButNotInterface on Pet {
            nickname
          }
        "#,
            &[RuleError::new(
                &error_message("nickname", "Pet"),
                &[SourcePosition::new(78, 2, 12)],
            )],
        );
    }

    #[test]
    fn meta_field_on_union() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment definedOnImplementorsButNotInterface on Pet {
            __typename
          }
        "#,
        );
    }

    #[test]
    fn fields_on_union() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment definedOnImplementorsQueriedOnUnion on CatOrDog {
            name
          }
        "#,
            &[RuleError::new(
                &error_message("name", "CatOrDog"),
                &[SourcePosition::new(82, 2, 12)],
            )],
        );
    }

    #[test]
    fn typename_on_union() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment objectFieldSelection on Pet {
            __typename
            ... on Dog {
              name
            }
            ... on Cat {
              name
            }
          }
        "#,
        );
    }

    #[test]
    fn valid_field_in_inline_fragment() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment objectFieldSelection on Pet {
            ... on Dog {
              name
            }
            ... {
              name
            }
          }
        "#,
        );
    }

    #[test]
    fn forbids_typename_on_subscription() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"subscription { __typename }"#,
            &[RuleError::new(
                "`__typename` may not be included as a root field in a \
                 subscription operation",
                &[SourcePosition::new(15, 0, 15)],
            )],
        );
    }

    #[test]
    fn forbids_typename_on_explicit_subscription() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"subscription SubscriptionRoot { __typename }"#,
            &[RuleError::new(
                "`__typename` may not be included as a root field in a \
                 subscription operation",
                &[SourcePosition::new(32, 0, 32)],
            )],
        );
    }
}
