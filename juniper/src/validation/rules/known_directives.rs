use crate::{
    ast::{
        Directive, Field, Fragment, FragmentSpread, InlineFragment, Operation, OperationType,
        VariableDefinition,
    },
    parser::Spanning,
    schema::model::DirectiveLocation,
    validation::{ValidatorContext, Visitor},
    value::ScalarValue,
};

pub struct KnownDirectives {
    location_stack: Vec<DirectiveLocation>,
}

pub fn factory() -> KnownDirectives {
    KnownDirectives {
        location_stack: Vec::new(),
    }
}

impl<'a, S> Visitor<'a, S> for KnownDirectives
where
    S: ScalarValue,
{
    fn enter_operation_definition(
        &mut self,
        _: &mut ValidatorContext<'a, S>,
        op: &'a Spanning<Operation<S>>,
    ) {
        self.location_stack.push(match op.item.operation_type {
            OperationType::Query => DirectiveLocation::Query,
            OperationType::Mutation => DirectiveLocation::Mutation,
            OperationType::Subscription => DirectiveLocation::Subscription,
        });
    }

    fn exit_operation_definition(
        &mut self,
        _: &mut ValidatorContext<'a, S>,
        _: &'a Spanning<Operation<S>>,
    ) {
        let top = self.location_stack.pop();
        assert!(
            top == Some(DirectiveLocation::Query)
                || top == Some(DirectiveLocation::Mutation)
                || top == Some(DirectiveLocation::Subscription)
        );
    }

    fn enter_field(&mut self, _: &mut ValidatorContext<'a, S>, _: &'a Spanning<Field<S>>) {
        self.location_stack.push(DirectiveLocation::Field);
    }

    fn exit_field(&mut self, _: &mut ValidatorContext<'a, S>, _: &'a Spanning<Field<S>>) {
        let top = self.location_stack.pop();
        assert_eq!(top, Some(DirectiveLocation::Field));
    }

    fn enter_fragment_definition(
        &mut self,
        _: &mut ValidatorContext<'a, S>,
        _: &'a Spanning<Fragment<S>>,
    ) {
        self.location_stack
            .push(DirectiveLocation::FragmentDefinition);
    }

    fn exit_fragment_definition(
        &mut self,
        _: &mut ValidatorContext<'a, S>,
        _: &'a Spanning<Fragment<S>>,
    ) {
        let top = self.location_stack.pop();
        assert_eq!(top, Some(DirectiveLocation::FragmentDefinition));
    }

    fn enter_fragment_spread(
        &mut self,
        _: &mut ValidatorContext<'a, S>,
        _: &'a Spanning<FragmentSpread<S>>,
    ) {
        self.location_stack.push(DirectiveLocation::FragmentSpread);
    }

    fn exit_fragment_spread(
        &mut self,
        _: &mut ValidatorContext<'a, S>,
        _: &'a Spanning<FragmentSpread<S>>,
    ) {
        let top = self.location_stack.pop();
        assert_eq!(top, Some(DirectiveLocation::FragmentSpread));
    }

    fn enter_inline_fragment(
        &mut self,
        _: &mut ValidatorContext<'a, S>,
        _: &'a Spanning<InlineFragment<S>>,
    ) {
        self.location_stack.push(DirectiveLocation::InlineFragment);
    }

    fn exit_inline_fragment(
        &mut self,
        _: &mut ValidatorContext<'a, S>,
        _: &'a Spanning<InlineFragment<S>>,
    ) {
        let top = self.location_stack.pop();
        assert_eq!(top, Some(DirectiveLocation::InlineFragment));
    }

    fn enter_variable_definition(
        &mut self,
        _: &mut ValidatorContext<'a, S>,
        _: &'a (Spanning<&'a str>, VariableDefinition<S>),
    ) {
        self.location_stack
            .push(DirectiveLocation::VariableDefinition);
    }

    fn exit_variable_definition(
        &mut self,
        _: &mut ValidatorContext<'a, S>,
        _: &'a (Spanning<&'a str>, VariableDefinition<S>),
    ) {
        let top = self.location_stack.pop();
        assert_eq!(top, Some(DirectiveLocation::VariableDefinition));
    }

    fn enter_directive(
        &mut self,
        ctx: &mut ValidatorContext<'a, S>,
        directive: &'a Spanning<Directive<S>>,
    ) {
        let directive_name = &directive.item.name.item;

        if let Some(directive_type) = ctx.schema.directive_by_name(directive_name) {
            if let Some(current_location) = self.location_stack.last() {
                if !directive_type
                    .locations
                    .iter()
                    .any(|l| l == current_location)
                {
                    ctx.report_error(
                        &misplaced_error_message(directive_name, current_location),
                        &[directive.span.start],
                    );
                }
            }
        } else {
            ctx.report_error(
                &unknown_error_message(directive_name),
                &[directive.span.start],
            );
        }
    }
}

fn unknown_error_message(directive_name: &str) -> String {
    format!(r#"Unknown directive "{directive_name}""#)
}

fn misplaced_error_message(directive_name: &str, location: &DirectiveLocation) -> String {
    format!(r#"Directive "{directive_name}" may not be used on {location}"#)
}

#[cfg(test)]
mod tests {
    use super::{factory, misplaced_error_message, unknown_error_message};

    use crate::{
        parser::SourcePosition,
        schema::model::DirectiveLocation,
        validation::{expect_fails_rule, expect_passes_rule, RuleError},
        value::DefaultScalarValue,
    };

    #[test]
    fn with_no_directives() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          query Foo {
            name
            ...Frag
          }

          fragment Frag on Dog {
            name
          }
        "#,
        );
    }

    #[test]
    fn with_known_directives() {
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
    fn with_unknown_directive() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          {
            dog @unknown(directive: "value") {
              name
            }
          }
        "#,
            &[RuleError::new(
                &unknown_error_message("unknown"),
                &[SourcePosition::new(29, 2, 16)],
            )],
        );
    }

    #[test]
    fn with_unknown_directive_on_var_definition() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"query Foo(
                $var1: Int = 1 @skip(if: true) @unknown,
                $var2: String @deprecated
            ) {
                name
            }"#,
            &[
                RuleError::new(
                    &misplaced_error_message("skip", &DirectiveLocation::VariableDefinition),
                    &[SourcePosition::new(42, 1, 31)],
                ),
                RuleError::new(
                    &unknown_error_message("unknown"),
                    &[SourcePosition::new(58, 1, 47)],
                ),
                RuleError::new(
                    &misplaced_error_message("deprecated", &DirectiveLocation::VariableDefinition),
                    &[SourcePosition::new(98, 2, 30)],
                ),
            ],
        );
    }

    #[test]
    fn with_many_unknown_directives() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          {
            dog @unknown(directive: "value") {
              name
            }
            human @unknown(directive: "value") {
              name
              pets @unknown(directive: "value") {
                name
              }
            }
          }
        "#,
            &[
                RuleError::new(
                    &unknown_error_message("unknown"),
                    &[SourcePosition::new(29, 2, 16)],
                ),
                RuleError::new(
                    &unknown_error_message("unknown"),
                    &[SourcePosition::new(111, 5, 18)],
                ),
                RuleError::new(
                    &unknown_error_message("unknown"),
                    &[SourcePosition::new(180, 7, 19)],
                ),
            ],
        );
    }

    #[test]
    fn with_well_placed_directives() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          query Foo @onQuery {
            name @include(if: true)
            ...Frag @include(if: true)
            skippedField @skip(if: true)
            ...SkippedFrag @skip(if: true)
          }

          mutation Bar @onMutation {
            someField
          }
        "#,
        );
    }

    #[test]
    fn with_misplaced_directives() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          query Foo @include(if: true) {
            name @onQuery
            ...Frag @onQuery
          }

          mutation Bar @onQuery {
            someField
          }
        "#,
            &[
                RuleError::new(
                    &misplaced_error_message("include", &DirectiveLocation::Query),
                    &[SourcePosition::new(21, 1, 20)],
                ),
                RuleError::new(
                    &misplaced_error_message("onQuery", &DirectiveLocation::Field),
                    &[SourcePosition::new(59, 2, 17)],
                ),
                RuleError::new(
                    &misplaced_error_message("onQuery", &DirectiveLocation::FragmentSpread),
                    &[SourcePosition::new(88, 3, 20)],
                ),
                RuleError::new(
                    &misplaced_error_message("onQuery", &DirectiveLocation::Mutation),
                    &[SourcePosition::new(133, 6, 23)],
                ),
            ],
        );
    }
}
