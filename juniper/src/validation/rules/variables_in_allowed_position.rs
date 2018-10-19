use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;

use ast::{Document, Fragment, FragmentSpread, Operation, Type, VariableDefinition};
use parser::Spanning;
use validation::{ValidatorContext, Visitor};
use value::ScalarValue;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Scope<'a> {
    Operation(Option<&'a str>),
    Fragment(&'a str),
}

pub struct VariableInAllowedPosition<'a, S: Debug + 'a> {
    spreads: HashMap<Scope<'a>, HashSet<&'a str>>,
    variable_usages: HashMap<Scope<'a>, Vec<(Spanning<&'a String>, Type<'a>)>>,
    variable_defs: HashMap<Scope<'a>, Vec<&'a (Spanning<&'a str>, VariableDefinition<'a, S>)>>,
    current_scope: Option<Scope<'a>>,
}

pub fn factory<'a, S: Debug>() -> VariableInAllowedPosition<'a, S> {
    VariableInAllowedPosition {
        spreads: HashMap::new(),
        variable_usages: HashMap::new(),
        variable_defs: HashMap::new(),
        current_scope: None,
    }
}

impl<'a, S: Debug> VariableInAllowedPosition<'a, S> {
    fn collect_incorrect_usages(
        &self,
        from: &Scope<'a>,
        var_defs: &Vec<&'a (Spanning<&'a str>, VariableDefinition<S>)>,
        ctx: &mut ValidatorContext<'a, S>,
        visited: &mut HashSet<Scope<'a>>,
    ) {
        if visited.contains(from) {
            return;
        }

        visited.insert(from.clone());

        if let Some(usages) = self.variable_usages.get(from) {
            for &(ref var_name, ref var_type) in usages {
                if let Some(&&(ref var_def_name, ref var_def)) = var_defs
                    .iter()
                    .find(|&&&(ref n, _)| &n.item == var_name.item)
                {
                    let expected_type = match (&var_def.default_value, &var_def.var_type.item) {
                        (&Some(_), &Type::List(ref inner)) => Type::NonNullList(inner.clone()),
                        (&Some(_), &Type::Named(ref inner)) => {
                            Type::NonNullNamed(Cow::Borrowed(inner))
                        }
                        (_, t) => t.clone(),
                    };

                    if !ctx.schema.is_subtype(&expected_type, var_type) {
                        ctx.report_error(
                            &error_message(
                                var_name.item,
                                &format!("{}", expected_type),
                                &format!("{}", var_type),
                            ),
                            &[var_def_name.start.clone(), var_name.start.clone()],
                        );
                    }
                }
            }
        }

        if let Some(spreads) = self.spreads.get(from) {
            for spread in spreads {
                self.collect_incorrect_usages(&Scope::Fragment(spread), var_defs, ctx, visited);
            }
        }
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
                .or_insert_with(HashSet::new)
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
                .or_insert_with(Vec::new)
                .push(def);
        }
    }

    fn enter_variable_value(
        &mut self,
        ctx: &mut ValidatorContext<'a, S>,
        var_name: Spanning<&'a String>,
    ) {
        if let (&Some(ref scope), Some(input_type)) =
            (&self.current_scope, ctx.current_input_type_literal())
        {
            self.variable_usages
                .entry(scope.clone())
                .or_insert_with(Vec::new)
                .push((
                    Spanning::start_end(&var_name.start, &var_name.end, var_name.item),
                    input_type.clone(),
                ));
        }
    }
}

fn error_message(var_name: &str, type_name: &str, expected_type_name: &str) -> String {
    format!(
        "Variable \"{}\" of type \"{}\" used in position expecting type \"{}\"",
        var_name, type_name, expected_type_name
    )
}

#[cfg(test)]
mod tests {
    use super::{error_message, factory};

    use parser::SourcePosition;
    use validation::{expect_fails_rule, expect_passes_rule, RuleError};
    use value::DefaultScalarValue;

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
