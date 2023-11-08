use crate::{
    ast::{Document, Fragment, FragmentSpread, InputValue, Operation, VariableDefinition},
    parser::Spanning,
    validation::{RuleError, ValidatorContext, Visitor},
    value::ScalarValue,
};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Scope<'a> {
    Operation(Option<&'a str>),
    Fragment(&'a str),
}

pub fn factory<'a>() -> NoUnusedVariables<'a> {
    NoUnusedVariables {
        defined_variables: HashMap::new(),
        used_variables: HashMap::new(),
        current_scope: None,
        spreads: HashMap::new(),
    }
}

pub struct NoUnusedVariables<'a> {
    defined_variables: HashMap<Option<&'a str>, HashSet<&'a Spanning<&'a str>>>,
    used_variables: HashMap<Scope<'a>, Vec<&'a str>>,
    current_scope: Option<Scope<'a>>,
    spreads: HashMap<Scope<'a>, Vec<&'a str>>,
}

impl<'a> NoUnusedVariables<'a> {
    fn find_used_vars(
        &'a self,
        from: &Scope<'a>,
        defined: &HashSet<&'a str>,
        used: &mut HashSet<&'a str>,
        visited: &mut HashSet<Scope<'a>>,
    ) {
        let mut to_visit = Vec::new();
        if let Some(spreads) = self.find_used_vars_inner(from, defined, used, visited) {
            to_visit.push(spreads);
        }
        while let Some(spreads) = to_visit.pop() {
            for spread in spreads {
                if let Some(spreads) =
                    self.find_used_vars_inner(&Scope::Fragment(spread), defined, used, visited)
                {
                    to_visit.push(spreads);
                }
            }
        }
    }

    /// This function should be called only inside [`Self::find_used_vars()`],
    /// as it's a recursive function using heap instead of a stack. So, instead
    /// of the recursive call, we return a [`Vec`] that is visited inside
    /// [`Self::find_used_vars()`].
    fn find_used_vars_inner(
        &'a self,
        from: &Scope<'a>,
        defined: &HashSet<&'a str>,
        used: &mut HashSet<&'a str>,
        visited: &mut HashSet<Scope<'a>>,
    ) -> Option<&'a Vec<&'a str>> {
        if visited.contains(from) {
            return None;
        }

        visited.insert(from.clone());

        if let Some(used_vars) = self.used_variables.get(from) {
            for var in used_vars {
                if defined.contains(var) {
                    used.insert(var);
                }
            }
        }

        self.spreads.get(from)
    }
}

impl<'a, S> Visitor<'a, S> for NoUnusedVariables<'a>
where
    S: ScalarValue,
{
    fn exit_document(&mut self, ctx: &mut ValidatorContext<'a, S>, _: &'a Document<S>) {
        for (op_name, def_vars) in &self.defined_variables {
            let mut used = HashSet::new();
            let mut visited = HashSet::new();
            self.find_used_vars(
                &Scope::Operation(*op_name),
                &def_vars.iter().map(|def| def.item).collect(),
                &mut used,
                &mut visited,
            );

            ctx.append_errors(
                def_vars
                    .iter()
                    .filter(|var| !used.contains(var.item))
                    .map(|var| {
                        RuleError::new(&error_message(var.item, *op_name), &[var.span.start])
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
        self.defined_variables.insert(op_name, HashSet::new());
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
            if let Some(vars) = self.defined_variables.get_mut(name) {
                vars.insert(var_name);
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
                .append(&mut value.item.referenced_variables());
        }
    }
}

fn error_message(var_name: &str, op_name: Option<&str>) -> String {
    if let Some(op_name) = op_name {
        format!(r#"Variable "${var_name}" is not used by operation "{op_name}""#)
    } else {
        format!(r#"Variable "${var_name}" is not used"#)
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
    fn uses_all_variables() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          query ($a: String, $b: String, $c: String) {
            field(a: $a, b: $b, c: $c)
          }
        "#,
        );
    }

    #[test]
    fn uses_all_variables_deeply() {
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
    fn uses_all_variables_deeply_in_inline_fragments() {
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
    fn uses_all_variables_in_fragments() {
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
    fn variable_used_by_fragment_in_multiple_operations() {
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
    fn variable_used_by_recursive_fragment() {
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
    fn variable_not_used() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          query ($a: String, $b: String, $c: String) {
            field(a: $a, b: $b)
          }
        "#,
            &[RuleError::new(
                &error_message("c", None),
                &[SourcePosition::new(42, 1, 41)],
            )],
        );
    }

    #[test]
    fn multiple_variables_not_used_1() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          query Foo($a: String, $b: String, $c: String) {
            field(b: $b)
          }
        "#,
            &[
                RuleError::new(
                    &error_message("a", Some("Foo")),
                    &[SourcePosition::new(21, 1, 20)],
                ),
                RuleError::new(
                    &error_message("c", Some("Foo")),
                    &[SourcePosition::new(45, 1, 44)],
                ),
            ],
        );
    }

    #[test]
    fn variable_not_used_in_fragment() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
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
            field
          }
        "#,
            &[RuleError::new(
                &error_message("c", Some("Foo")),
                &[SourcePosition::new(45, 1, 44)],
            )],
        );
    }

    #[test]
    fn multiple_variables_not_used_2() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          query Foo($a: String, $b: String, $c: String) {
            ...FragA
          }
          fragment FragA on Type {
            field {
              ...FragB
            }
          }
          fragment FragB on Type {
            field(b: $b) {
              ...FragC
            }
          }
          fragment FragC on Type {
            field
          }
        "#,
            &[
                RuleError::new(
                    &error_message("a", Some("Foo")),
                    &[SourcePosition::new(21, 1, 20)],
                ),
                RuleError::new(
                    &error_message("c", Some("Foo")),
                    &[SourcePosition::new(45, 1, 44)],
                ),
            ],
        );
    }

    #[test]
    fn variable_not_used_by_unreferenced_fragment() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          query Foo($b: String) {
            ...FragA
          }
          fragment FragA on Type {
            field(a: $a)
          }
          fragment FragB on Type {
            field(b: $b)
          }
        "#,
            &[RuleError::new(
                &error_message("b", Some("Foo")),
                &[SourcePosition::new(21, 1, 20)],
            )],
        );
    }

    #[test]
    fn variable_not_used_by_fragment_used_by_other_operation() {
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
                    &error_message("b", Some("Foo")),
                    &[SourcePosition::new(21, 1, 20)],
                ),
                RuleError::new(
                    &error_message("a", Some("Bar")),
                    &[SourcePosition::new(88, 4, 20)],
                ),
            ],
        );
    }
}
