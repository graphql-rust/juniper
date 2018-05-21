use ast::Field;
use parser::Spanning;
use schema::meta::MetaType;
use validation::{ValidatorContext, Visitor};

pub struct FieldsOnCorrectType {}

pub fn factory() -> FieldsOnCorrectType {
    FieldsOnCorrectType {}
}

impl<'a> Visitor<'a> for FieldsOnCorrectType {
    fn enter_field(&mut self, context: &mut ValidatorContext<'a>, field: &'a Spanning<Field>) {
        {
            if let Some(parent_type) = context.parent_type() {
                let field_name = &field.item.name;
                let type_name = parent_type.name().unwrap_or("<unknown>");

                if parent_type.field_by_name(field_name.item).is_none() {
                    match *parent_type {
                        MetaType::Union(..) => {
                            // You can query for `__typename` on a union,
                            // but it isn't a field on the union...it is
                            // instead on the resulting object returned.
                            if field_name.item == "__typename" {
                                return;
                            }
                        }
                        _ => {}
                    }

                    context.report_error(
                        &error_message(field_name.item, type_name),
                        &[field_name.start.clone()],
                    );
                }
            }
        }
    }
}

fn error_message(field: &str, type_name: &str) -> String {
    format!(r#"Unknown field "{}" on type "{}""#, field, type_name)
}

#[cfg(test)]
mod tests {
    use super::{error_message, factory};

    use parser::SourcePosition;
    use validation::{expect_fails_rule, expect_passes_rule, RuleError};

    #[test]
    fn selection_on_object() {
        expect_passes_rule(
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
        expect_passes_rule(
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
        expect_passes_rule(
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
        expect_passes_rule(
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
        expect_passes_rule(
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
        expect_passes_rule(
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
        expect_fails_rule(
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
        expect_fails_rule(
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
        expect_fails_rule(
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
        expect_fails_rule(
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
        expect_fails_rule(
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
        expect_fails_rule(
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
        expect_fails_rule(
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
        expect_fails_rule(
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
        expect_fails_rule(
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
        expect_passes_rule(
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
        expect_fails_rule(
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
        expect_passes_rule(
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
        expect_passes_rule(
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

}
