use crate::{
    ast::{Document, Fragment, FragmentSpread, InputValue, Operation, VariableDefinition},
    parser::{SourcePosition, Spanning},
    validation::{RuleError, ValidatorContext, Visitor},
    value::ScalarValue,
    Span,
};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Scope<'a> {
    Operation(Option<&'a str>),
    Fragment(&'a str),
}

pub fn factory<'a>() -> NoUndefinedVariables<'a> {
    NoUndefinedVariables {
        defined_variables: HashMap::new(),
        used_variables: HashMap::new(),
        current_scope: None,
        spreads: HashMap::new(),
    }
}

type BorrowedSpanning<'a, T> = Spanning<&'a T, &'a Span>;

pub struct NoUndefinedVariables<'a> {
    defined_variables: HashMap<Option<&'a str>, (SourcePosition, HashSet<&'a str>)>,
    used_variables: HashMap<Scope<'a>, Vec<BorrowedSpanning<'a, str>>>,
    current_scope: Option<Scope<'a>>,
    spreads: HashMap<Scope<'a>, Vec<&'a str>>,
}

impl<'a> NoUndefinedVariables<'a> {
    fn find_undef_vars(
        &'a self,
        scope: &Scope<'a>,
        defined: &HashSet<&'a str>,
        unused: &mut Vec<BorrowedSpanning<'a, str>>,
        visited: &mut HashSet<Scope<'a>>,
    ) {
        let mut to_visit = Vec::new();
        if let Some(spreads) = self.find_undef_vars_inner(scope, defined, unused, visited) {
            to_visit.push(spreads);
        }
        while let Some(spreads) = to_visit.pop() {
            for spread in spreads {
                if let Some(spreads) =
                    self.find_undef_vars_inner(&Scope::Fragment(spread), defined, unused, visited)
                {
                    to_visit.push(spreads);
                }
            }
        }
    }

    /// This function should be called only inside [`Self::find_undef_vars()`],
    /// as it's a recursive function using heap instead of a stack. So, instead
    /// of the recursive call, we return a [`Vec`] that is visited inside
    /// [`Self::find_undef_vars()`].
    fn find_undef_vars_inner(
        &'a self,
        scope: &Scope<'a>,
        defined: &HashSet<&'a str>,
        unused: &mut Vec<BorrowedSpanning<'a, str>>,
        visited: &mut HashSet<Scope<'a>>,
    ) -> Option<&'a Vec<&'a str>> {
        if visited.contains(scope) {
            return None;
        }

        visited.insert(scope.clone());

        if let Some(used_vars) = self.used_variables.get(scope) {
            for var in used_vars {
                if !defined.contains(&var.item) {
                    unused.push(*var);
                }
            }
        }

        self.spreads.get(scope)
    }
}

impl<'a, S> Visitor<'a, S> for NoUndefinedVariables<'a>
where
    S: ScalarValue,
{
    fn exit_document(&mut self, ctx: &mut ValidatorContext<'a, S>, _: &'a Document<S>) {
        for (op_name, (pos, def_vars)) in &self.defined_variables {
            let mut unused = Vec::new();
            let mut visited = HashSet::new();
            self.find_undef_vars(
                &Scope::Operation(*op_name),
                def_vars,
                &mut unused,
                &mut visited,
            );

            ctx.append_errors(
                unused
                    .into_iter()
                    .map(|var| {
                        RuleError::new(&error_message(var.item, *op_name), &[var.span.start, *pos])
                    })
                    .collect(),
            );
        }
    }

    fn enter_operation_definition(
        &mut self,
        _: &mut ValidatorContext<'a, S>,
        op: &'a Spanning<Operation<S>>,
    ) {
        let op_name = op.item.name.as_ref().map(|s| s.item);
        self.current_scope = Some(Scope::Operation(op_name));
        self.defined_variables
            .insert(op_name, (op.span.start, HashSet::new()));
    }

    fn enter_fragment_definition(
        &mut self,
        _: &mut ValidatorContext<'a, S>,
        f: &'a Spanning<Fragment<S>>,
    ) {
        self.current_scope = Some(Scope::Fragment(f.item.name.item));
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
                .push(spread.item.name.item);
        }
    }

    fn enter_variable_definition(
        &mut self,
        _: &mut ValidatorContext<'a, S>,
        (var_name, _): &'a (Spanning<&'a str>, VariableDefinition<S>),
    ) {
        if let Some(Scope::Operation(ref name)) = self.current_scope {
            if let Some(&mut (_, ref mut vars)) = self.defined_variables.get_mut(name) {
                vars.insert(var_name.item);
            }
        }
    }

    fn enter_argument(
        &mut self,
        _: &mut ValidatorContext<'a, S>,
        (_, value): &'a (Spanning<&'a str>, Spanning<InputValue<S>>),
    ) {
        if let Some(ref scope) = self.current_scope {
            self.used_variables
                .entry(scope.clone())
                .or_default()
                .append(
                    &mut value
                        .item
                        .referenced_variables()
                        .iter()
                        .map(|&var_name| BorrowedSpanning {
                            span: &value.span,
                            item: var_name,
                        })
                        .collect(),
                );
        }
    }
}

fn error_message(var_name: &str, op_name: Option<&str>) -> String {
    if let Some(op_name) = op_name {
        format!(r#"Variable "${var_name}" is not defined by operation "{op_name}""#)
    } else {
        format!(r#"Variable "${var_name}" is not defined"#)
    }
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
    fn all_variables_defined() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          query Foo($a: String, $b: String, $c: String) {
            field(a: $a, b: $b, c: $c)
          }
        "#,
        );
    }

    #[test]
    fn all_variables_deeply_defined() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          query Foo($a: String, $b: String, $c: String) {
            field(a: $a) {
              field(b: $b) {
                field(c: $c)
              }
            }
          }
        "#,
        );
    }

    #[test]
    fn all_variables_deeply_defined_in_inline_fragments_defined() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          query Foo($a: String, $b: String, $c: String) {
            ... on Type {
              field(a: $a) {
                field(b: $b) {
                  ... on Type {
                    field(c: $c)
                  }
                }
              }
            }
          }
        "#,
        );
    }

    #[test]
    fn all_variables_in_fragments_deeply_defined() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          query Foo($a: String, $b: String, $c: String) {
            ...FragA
          }
          fragment FragA on Type {
            field(a: $a) {
              ...FragB
            }
          }
          fragment FragB on Type {
            field(b: $b) {
              ...FragC
            }
          }
          fragment FragC on Type {
            field(c: $c)
          }
        "#,
        );
    }

    #[test]
    fn variable_within_single_fragment_defined_in_multiple_operations() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          query Foo($a: String) {
            ...FragA
          }
          query Bar($a: String) {
            ...FragA
          }
          fragment FragA on Type {
            field(a: $a)
          }
        "#,
        );
    }

    #[test]
    fn variable_within_fragments_defined_in_operations() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          query Foo($a: String) {
            ...FragA
          }
          query Bar($b: String) {
            ...FragB
          }
          fragment FragA on Type {
            field(a: $a)
          }
          fragment FragB on Type {
            field(b: $b)
          }
        "#,
        );
    }

    #[test]
    fn variable_within_recursive_fragment_defined() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          query Foo($a: String) {
            ...FragA
          }
          fragment FragA on Type {
            field(a: $a) {
              ...FragA
            }
          }
        "#,
        );
    }

    #[test]
    fn variable_not_defined() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          query Foo($a: String, $b: String, $c: String) {
            field(a: $a, b: $b, c: $c, d: $d)
          }
        "#,
            &[RuleError::new(
                &error_message("d", Some("Foo")),
                &[
                    SourcePosition::new(101, 2, 42),
                    SourcePosition::new(11, 1, 10),
                ],
            )],
        );
    }

    #[test]
    fn variable_not_defined_by_unnamed_query() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          {
            field(a: $a)
          }
        "#,
            &[RuleError::new(
                &error_message("a", None),
                &[
                    SourcePosition::new(34, 2, 21),
                    SourcePosition::new(11, 1, 10),
                ],
            )],
        );
    }

    #[test]
    fn multiple_variables_not_defined() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          query Foo($b: String) {
            field(a: $a, b: $b, c: $c)
          }
        "#,
            &[
                RuleError::new(
                    &error_message("a", Some("Foo")),
                    &[
                        SourcePosition::new(56, 2, 21),
                        SourcePosition::new(11, 1, 10),
                    ],
                ),
                RuleError::new(
                    &error_message("c", Some("Foo")),
                    &[
                        SourcePosition::new(70, 2, 35),
                        SourcePosition::new(11, 1, 10),
                    ],
                ),
            ],
        );
    }

    #[test]
    fn variable_in_fragment_not_defined_by_unnamed_query() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          {
            ...FragA
          }
          fragment FragA on Type {
            field(a: $a)
          }
        "#,
            &[RuleError::new(
                &error_message("a", None),
                &[
                    SourcePosition::new(102, 5, 21),
                    SourcePosition::new(11, 1, 10),
                ],
            )],
        );
    }

    #[test]
    fn variable_in_fragment_not_defined_by_operation() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          query Foo($a: String, $b: String) {
            ...FragA
          }
          fragment FragA on Type {
            field(a: $a) {
              ...FragB
            }
          }
          fragment FragB on Type {
            field(b: $b) {
              ...FragC
            }
          }
          fragment FragC on Type {
            field(c: $c)
          }
        "#,
            &[RuleError::new(
                &error_message("c", Some("Foo")),
                &[
                    SourcePosition::new(358, 15, 21),
                    SourcePosition::new(11, 1, 10),
                ],
            )],
        );
    }

    #[test]
    fn multiple_variables_in_fragments_not_defined() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          query Foo($b: String) {
            ...FragA
          }
          fragment FragA on Type {
            field(a: $a) {
              ...FragB
            }
          }
          fragment FragB on Type {
            field(b: $b) {
              ...FragC
            }
          }
          fragment FragC on Type {
            field(c: $c)
          }
        "#,
            &[
                RuleError::new(
                    &error_message("a", Some("Foo")),
                    &[
                        SourcePosition::new(124, 5, 21),
                        SourcePosition::new(11, 1, 10),
                    ],
                ),
                RuleError::new(
                    &error_message("c", Some("Foo")),
                    &[
                        SourcePosition::new(346, 15, 21),
                        SourcePosition::new(11, 1, 10),
                    ],
                ),
            ],
        );
    }

    #[test]
    fn single_variable_in_fragment_not_defined_by_multiple_operations() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          query Foo($a: String) {
            ...FragAB
          }
          query Bar($a: String) {
            ...FragAB
          }
          fragment FragAB on Type {
            field(a: $a, b: $b)
          }
        "#,
            &[
                RuleError::new(
                    &error_message("b", Some("Foo")),
                    &[
                        SourcePosition::new(201, 8, 28),
                        SourcePosition::new(11, 1, 10),
                    ],
                ),
                RuleError::new(
                    &error_message("b", Some("Bar")),
                    &[
                        SourcePosition::new(201, 8, 28),
                        SourcePosition::new(79, 4, 10),
                    ],
                ),
            ],
        );
    }

    #[test]
    fn variables_in_fragment_not_defined_by_multiple_operations() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          query Foo($b: String) {
            ...FragAB
          }
          query Bar($a: String) {
            ...FragAB
          }
          fragment FragAB on Type {
            field(a: $a, b: $b)
          }
        "#,
            &[
                RuleError::new(
                    &error_message("a", Some("Foo")),
                    &[
                        SourcePosition::new(194, 8, 21),
                        SourcePosition::new(11, 1, 10),
                    ],
                ),
                RuleError::new(
                    &error_message("b", Some("Bar")),
                    &[
                        SourcePosition::new(201, 8, 28),
                        SourcePosition::new(79, 4, 10),
                    ],
                ),
            ],
        );
    }

    #[test]
    fn variable_in_fragment_used_by_other_operation() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          query Foo($b: String) {
            ...FragA
          }
          query Bar($a: String) {
            ...FragB
          }
          fragment FragA on Type {
            field(a: $a)
          }
          fragment FragB on Type {
            field(b: $b)
          }
        "#,
            &[
                RuleError::new(
                    &error_message("a", Some("Foo")),
                    &[
                        SourcePosition::new(191, 8, 21),
                        SourcePosition::new(11, 1, 10),
                    ],
                ),
                RuleError::new(
                    &error_message("b", Some("Bar")),
                    &[
                        SourcePosition::new(263, 11, 21),
                        SourcePosition::new(78, 4, 10),
                    ],
                ),
            ],
        );
    }

    #[test]
    fn multiple_undefined_variables_produce_multiple_errors() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          query Foo($b: String) {
            ...FragAB
          }
          query Bar($a: String) {
            ...FragAB
          }
          fragment FragAB on Type {
            field1(a: $a, b: $b)
            ...FragC
            field3(a: $a, b: $b)
          }
          fragment FragC on Type {
            field2(c: $c)
          }
        "#,
            &[
                RuleError::new(
                    &error_message("a", Some("Foo")),
                    &[
                        SourcePosition::new(195, 8, 22),
                        SourcePosition::new(11, 1, 10),
                    ],
                ),
                RuleError::new(
                    &error_message("b", Some("Bar")),
                    &[
                        SourcePosition::new(202, 8, 29),
                        SourcePosition::new(79, 4, 10),
                    ],
                ),
                RuleError::new(
                    &error_message("a", Some("Foo")),
                    &[
                        SourcePosition::new(249, 10, 22),
                        SourcePosition::new(11, 1, 10),
                    ],
                ),
                RuleError::new(
                    &error_message("b", Some("Bar")),
                    &[
                        SourcePosition::new(256, 10, 29),
                        SourcePosition::new(79, 4, 10),
                    ],
                ),
                RuleError::new(
                    &error_message("c", Some("Foo")),
                    &[
                        SourcePosition::new(329, 13, 22),
                        SourcePosition::new(11, 1, 10),
                    ],
                ),
                RuleError::new(
                    &error_message("c", Some("Bar")),
                    &[
                        SourcePosition::new(329, 13, 22),
                        SourcePosition::new(79, 4, 10),
                    ],
                ),
            ],
        );
    }
}
