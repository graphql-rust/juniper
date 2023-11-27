//! Validation rule checking whether a GraphQL operation contains introspection (`__schema` or
//! `__type` fields).

use crate::{
    ast::Field,
    parser::Spanning,
    validation::{ValidatorContext, Visitor},
    value::ScalarValue,
};

/// Validation rule checking whether a GraphQL operation contains introspection (`__schema` or
/// `__type` fields).
pub struct DisableIntrospection;

/// Produces a new [`DisableIntrospection`] validation rule.
#[inline]
#[must_use]
pub fn factory() -> DisableIntrospection {
    DisableIntrospection
}

impl<'a, S> Visitor<'a, S> for DisableIntrospection
where
    S: ScalarValue,
{
    fn enter_field(
        &mut self,
        context: &mut ValidatorContext<'a, S>,
        field: &'a Spanning<Field<S>>,
    ) {
        let field_name = field.item.name.item;
        if matches!(field_name, "__schema" | "__type") {
            context.report_error(&error_message(field_name), &[field.item.name.span.start]);
        }
    }
}

fn error_message(field_name: &str) -> String {
    format!("GraphQL introspection is not allowed, but the operation contained `{field_name}`")
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
    fn allows_regular_fields() {
        // language=GraphQL
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
            query {
                user {
                    name
                    ... on User {
                        email
                    }
                    alias: email
                    ... {
                        typeless
                    }
                    friends {
                        name
                    }
                }
            }
            "#,
        );
    }

    #[test]
    fn allows_typename_field() {
        // language=GraphQL
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
            query {
                __typename
                user {
                    __typename
                    ... on User {
                        __typename
                    }
                    ... {
                        __typename
                    }
                    friends {
                        __typename
                    }
                }
            }
            "#,
        );
    }

    #[test]
    fn forbids_query_schema() {
        // language=GraphQL
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
            query {
                __schema {
                    queryType {
                       name
                    }
                }
            }
            "#,
            &[RuleError::new(
                &error_message("__schema"),
                &[SourcePosition::new(37, 2, 16)],
            )],
        );
    }

    #[test]
    fn forbids_query_type() {
        // language=GraphQL
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
            query {
                __type(
                   name: "Query"
                ) {
                   name
                }
            }
            "#,
            &[RuleError::new(
                &error_message("__type"),
                &[SourcePosition::new(37, 2, 16)],
            )],
        );
    }

    #[test]
    fn forbids_field_type() {
        // language=GraphQL
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
            query {
                user {
                    name
                    ... on User {
                        email
                    }
                    alias: email
                    ... {
                        typeless
                    }
                    friends {
                        name
                    }
                    __type
                }
            }
            "#,
            &[RuleError::new(
                &error_message("__type"),
                &[SourcePosition::new(370, 14, 20)],
            )],
        );
    }

    #[test]
    fn forbids_field_schema() {
        // language=GraphQL
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
            query {
                user {
                    name
                    ... on User {
                        email
                    }
                    alias: email
                    ... {
                        typeless
                    }
                    friends {
                        name
                    }
                    __schema
                }
            }
            "#,
            &[RuleError::new(
                &error_message("__schema"),
                &[SourcePosition::new(370, 14, 20)],
            )],
        );
    }
}
