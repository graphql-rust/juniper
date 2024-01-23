use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    fmt,
};

use crate::{
    ast::{Document, Fragment, FragmentSpread, Operation, Type, VariableDefinition},
    parser::Spanning,
    validation::{ValidatorContext, Visitor},
    value::ScalarValue,
    Span,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Scope<'a> {
    Operation(Option<&'a str>),
    Fragment(&'a str),
}

pub fn factory<'a, S: fmt::Debug>() -> VariableInAllowedPosition<'a, S> {
    VariableInAllowedPosition {
        spreads: HashMap::new(),
        variable_usages: HashMap::new(),
        variable_defs: HashMap::new(),
        current_scope: None,
    }
}

pub struct VariableInAllowedPosition<'a, S: fmt::Debug + 'a> {
    spreads: HashMap<Scope<'a>, HashSet<&'a str>>,
    variable_usages: HashMap<Scope<'a>, Vec<(SpannedInput<'a, String>, Type<'a>)>>,
    #[allow(clippy::type_complexity)]
    variable_defs: HashMap<Scope<'a>, Vec<&'a (Spanning<&'a str>, VariableDefinition<'a, S>)>>,
    current_scope: Option<Scope<'a>>,
}

impl<'a, S: fmt::Debug> VariableInAllowedPosition<'a, S> {
    fn collect_incorrect_usages<'me>(
        &'me self,
        from: &Scope<'a>,
        var_defs: &[&'a (Spanning<&'a str>, VariableDefinition<S>)],
        ctx: &mut ValidatorContext<'a, S>,
        visited: &mut HashSet<Scope<'a>>,
    ) {
        let mut to_visit = Vec::new();
        if let Some(spreads) = self.collect_incorrect_usages_inner(from, var_defs, ctx, visited) {
            to_visit.push(spreads);
        }

        while let Some(spreads) = to_visit.pop() {
            for spread in spreads {
                if let Some(spreads) = self.collect_incorrect_usages_inner(
                    &Scope::Fragment(spread),
                    var_defs,
                    ctx,
                    visited,
                ) {
                    to_visit.push(spreads);
                }
            }
        }
    }

    /// This function should be called only inside
    /// [`Self::collect_incorrect_usages()`], as it's a recursive function using
    /// heap instead of a stack. So, instead of the recursive call, we return a
    /// [`Vec`] that is visited inside [`Self::collect_incorrect_usages()`].
    fn collect_incorrect_usages_inner<'me>(
        &'me self,
        from: &Scope<'a>,
        var_defs: &[&'a (Spanning<&'a str>, VariableDefinition<S>)],
        ctx: &mut ValidatorContext<'a, S>,
        visited: &mut HashSet<Scope<'a>>,
    ) -> Option<&'me HashSet<&'a str>> {
        if visited.contains(from) {
            return None;
        }

        visited.insert(from.clone());

        if let Some(usages) = self.variable_usages.get(from) {
            for (var_name, var_type) in usages {
                if let Some(&(var_def_name, var_def)) =
                    var_defs.iter().find(|&&(n, _)| n.item == var_name.item)
                {
                    let expected_type = match (&var_def.default_value, &var_def.var_type.item) {
                        (&Some(_), Type::List(inner, expected_size)) => {
                            Type::NonNullList(inner.clone(), *expected_size)
                        }
                        (&Some(_), Type::Named(inner)) => Type::NonNullNamed(Cow::Borrowed(inner)),
                        (_, t) => t.clone(),
                    };

                    if !ctx.schema.is_subtype(&expected_type, var_type) {
                        ctx.report_error(
                            &error_message(var_name.item, expected_type, var_type),
                            &[var_def_name.span.start, var_name.span.start],
                        );
                    }
                }
            }
        }

        self.spreads.get(from)
    }
}

impl<'a, S> Visitor<'a, S> for VariableInAllowedPosition<'a, S>
where
    S: ScalarValue,
{
    fn exit_document(&mut self, ctx: &mut ValidatorContext<'a, S>, _: &'a Document<S>) {
        for (op_scope, var_defs) in &self.variable_defs {
            self.collect_incorrect_usages(op_scope, var_defs, ctx, &mut HashSet::new());
        }
    }

    fn enter_fragment_definition(
        &mut self,
        _: &mut ValidatorContext<'a, S>,
        fragment: &'a Spanning<Fragment<S>>,
    ) {
        self.current_scope = Some(Scope::Fragment(fragment.item.name.item));
    }

    fn enter_operation_definition(
        &mut self,
        _: &mut ValidatorContext<'a, S>,
        op: &'a Spanning<Operation<S>>,
    ) {
        self.current_scope = Some(Scope::Operation(op.item.name.as_ref().map(|s| s.item)));
    }

    fn enter_fragment_spread(
        &mut self,
        _: &mut ValidatorContext<'a, S>,
        spread: &'a Spanning<FragmentSpread<S>>,
    ) {
        if let Some(ref scope) = self.current_scope {
            self.spreads
                .entry(scope.clone())
                .or_default()
                .insert(spread.item.name.item);
        }
    }

    fn enter_variable_definition(
        &mut self,
        _: &mut ValidatorContext<'a, S>,
        def: &'a (Spanning<&'a str>, VariableDefinition<S>),
    ) {
        if let Some(ref scope) = self.current_scope {
            self.variable_defs
                .entry(scope.clone())
                .or_default()
                .push(def);
        }
    }

    fn enter_variable_value(
        &mut self,
        ctx: &mut ValidatorContext<'a, S>,
        var_name: SpannedInput<'a, String>,
    ) {
        if let (Some(scope), Some(input_type)) =
            (&self.current_scope, ctx.current_input_type_literal())
        {
            self.variable_usages
                .entry(scope.clone())
                .or_default()
                .push((var_name, input_type.clone()));
        }
    }
}

fn error_message(
    var_name: impl fmt::Display,
    type_name: impl fmt::Display,
    expected_type_name: impl fmt::Display,
) -> String {
    format!(
        "Variable \"{var_name}\" of type \"{type_name}\" used in position expecting type \"{expected_type_name}\"",
    )
}

type SpannedInput<'a, T> = Spanning<&'a T, &'a Span>;

#[cfg(test)]
mod tests {
    use super::{error_message, factory};

    use crate::{
        parser::SourcePosition,
        validation::{expect_fails_rule, expect_passes_rule, RuleError},
        value::DefaultScalarValue,
    };

    #[test]
    fn boolean_into_boolean() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          query Query($booleanArg: Boolean)
          {
            complicatedArgs {
              booleanArgField(booleanArg: $booleanArg)
            }
          }
        "#,
        );
    }

    #[test]
    fn boolean_into_boolean_within_fragment() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment booleanArgFrag on ComplicatedArgs {
            booleanArgField(booleanArg: $booleanArg)
          }
          query Query($booleanArg: Boolean)
          {
            complicatedArgs {
              ...booleanArgFrag
            }
          }
        "#,
        );

        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          query Query($booleanArg: Boolean)
          {
            complicatedArgs {
              ...booleanArgFrag
            }
          }
          fragment booleanArgFrag on ComplicatedArgs {
            booleanArgField(booleanArg: $booleanArg)
          }
        "#,
        );
    }

    #[test]
    fn non_null_boolean_into_boolean() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          query Query($nonNullBooleanArg: Boolean!)
          {
            complicatedArgs {
              booleanArgField(booleanArg: $nonNullBooleanArg)
            }
          }
        "#,
        );
    }

    #[test]
    fn non_null_boolean_into_boolean_within_fragment() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment booleanArgFrag on ComplicatedArgs {
            booleanArgField(booleanArg: $nonNullBooleanArg)
          }

          query Query($nonNullBooleanArg: Boolean!)
          {
            complicatedArgs {
              ...booleanArgFrag
            }
          }
        "#,
        );
    }

    #[test]
    fn int_into_non_null_int_with_default() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          query Query($intArg: Int = 1)
          {
            complicatedArgs {
              nonNullIntArgField(nonNullIntArg: $intArg)
            }
          }
        "#,
        );
    }

    #[test]
    fn string_list_into_string_list() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          query Query($stringListVar: [String])
          {
            complicatedArgs {
              stringListArgField(stringListArg: $stringListVar)
            }
          }
        "#,
        );
    }

    #[test]
    fn non_null_string_list_into_string_list() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          query Query($stringListVar: [String!])
          {
            complicatedArgs {
              stringListArgField(stringListArg: $stringListVar)
            }
          }
        "#,
        );
    }

    #[test]
    fn string_into_string_list_in_item_position() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          query Query($stringVar: String)
          {
            complicatedArgs {
              stringListArgField(stringListArg: [$stringVar])
            }
          }
        "#,
        );
    }

    #[test]
    fn non_null_string_into_string_list_in_item_position() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          query Query($stringVar: String!)
          {
            complicatedArgs {
              stringListArgField(stringListArg: [$stringVar])
            }
          }
        "#,
        );
    }

    #[test]
    fn complex_input_into_complex_input() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          query Query($complexVar: ComplexInput)
          {
            complicatedArgs {
              complexArgField(complexArg: $complexVar)
            }
          }
        "#,
        );
    }

    #[test]
    fn complex_input_into_complex_input_in_field_position() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          query Query($boolVar: Boolean = false)
          {
            complicatedArgs {
              complexArgField(complexArg: {requiredArg: $boolVar})
            }
          }
        "#,
        );
    }

    #[test]
    fn non_null_boolean_into_non_null_boolean_in_directive() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          query Query($boolVar: Boolean!)
          {
            dog @include(if: $boolVar)
          }
        "#,
        );
    }

    #[test]
    fn boolean_in_non_null_in_directive_with_default() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          query Query($boolVar: Boolean = false)
          {
            dog @include(if: $boolVar)
          }
        "#,
        );
    }

    #[test]
    fn int_into_non_null_int() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          query Query($intArg: Int) {
            complicatedArgs {
              nonNullIntArgField(nonNullIntArg: $intArg)
            }
          }
        "#,
            &[RuleError::new(
                &error_message("intArg", "Int", "Int!"),
                &[
                    SourcePosition::new(23, 1, 22),
                    SourcePosition::new(117, 3, 48),
                ],
            )],
        );
    }

    #[test]
    fn int_into_non_null_int_within_fragment() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment nonNullIntArgFieldFrag on ComplicatedArgs {
            nonNullIntArgField(nonNullIntArg: $intArg)
          }

          query Query($intArg: Int) {
            complicatedArgs {
              ...nonNullIntArgFieldFrag
            }
          }
        "#,
            &[RuleError::new(
                &error_message("intArg", "Int", "Int!"),
                &[
                    SourcePosition::new(154, 5, 22),
                    SourcePosition::new(110, 2, 46),
                ],
            )],
        );
    }

    #[test]
    fn int_into_non_null_int_within_nested_fragment() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment outerFrag on ComplicatedArgs {
            ...nonNullIntArgFieldFrag
          }

          fragment nonNullIntArgFieldFrag on ComplicatedArgs {
            nonNullIntArgField(nonNullIntArg: $intArg)
          }

          query Query($intArg: Int) {
            complicatedArgs {
              ...outerFrag
            }
          }
        "#,
            &[RuleError::new(
                &error_message("intArg", "Int", "Int!"),
                &[
                    SourcePosition::new(255, 9, 22),
                    SourcePosition::new(211, 6, 46),
                ],
            )],
        );
    }

    #[test]
    fn string_over_boolean() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          query Query($stringVar: String) {
            complicatedArgs {
              booleanArgField(booleanArg: $stringVar)
            }
          }
        "#,
            &[RuleError::new(
                &error_message("stringVar", "String", "Boolean"),
                &[
                    SourcePosition::new(23, 1, 22),
                    SourcePosition::new(117, 3, 42),
                ],
            )],
        );
    }

    #[test]
    fn string_into_string_list() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          query Query($stringVar: String) {
            complicatedArgs {
              stringListArgField(stringListArg: $stringVar)
            }
          }
        "#,
            &[RuleError::new(
                &error_message("stringVar", "String", "[String]"),
                &[
                    SourcePosition::new(23, 1, 22),
                    SourcePosition::new(123, 3, 48),
                ],
            )],
        );
    }

    #[test]
    fn boolean_into_non_null_boolean_in_directive() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          query Query($boolVar: Boolean) {
            dog @include(if: $boolVar)
          }
        "#,
            &[RuleError::new(
                &error_message("boolVar", "Boolean", "Boolean!"),
                &[
                    SourcePosition::new(23, 1, 22),
                    SourcePosition::new(73, 2, 29),
                ],
            )],
        );
    }

    #[test]
    fn string_into_non_null_boolean_in_directive() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          query Query($stringVar: String) {
            dog @include(if: $stringVar)
          }
        "#,
            &[RuleError::new(
                &error_message("stringVar", "String", "Boolean!"),
                &[
                    SourcePosition::new(23, 1, 22),
                    SourcePosition::new(74, 2, 29),
                ],
            )],
        );
    }
}
