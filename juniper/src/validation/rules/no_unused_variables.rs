use ast::{Document, Fragment, FragmentSpread, InputValue, Operation, VariableDefinition};
use parser::Spanning;
use std::collections::{HashMap, HashSet};
use validation::{RuleError, ValidatorContext, Visitor};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Scope<'a> {
    Operation(Option<&'a str>),
    Fragment(&'a str),
}

pub struct NoUnusedVariables<'a> {
    defined_variables: HashMap<Option<&'a str>, HashSet<&'a Spanning<&'a str>>>,
    used_variables: HashMap<Scope<'a>, Vec<&'a str>>,
    current_scope: Option<Scope<'a>>,
    spreads: HashMap<Scope<'a>, Vec<&'a str>>,
}

pub fn factory<'a>() -> NoUnusedVariables<'a> {
    NoUnusedVariables {
        defined_variables: HashMap::new(),
        used_variables: HashMap::new(),
        current_scope: None,
        spreads: HashMap::new(),
    }
}

impl<'a> NoUnusedVariables<'a> {
    fn find_used_vars(
        &self,
        from: &Scope<'a>,
        defined: &HashSet<&'a str>,
        used: &mut HashSet<&'a str>,
        visited: &mut HashSet<Scope<'a>>,
    ) {
        if visited.contains(from) {
            return;
        }

        visited.insert(from.clone());

        if let Some(used_vars) = self.used_variables.get(from) {
            for var in used_vars {
                if defined.contains(var) {
                    used.insert(var);
                }
            }
        }

        if let Some(spreads) = self.spreads.get(from) {
            for spread in spreads {
                self.find_used_vars(&Scope::Fragment(spread), defined, used, visited);
            }
        }
    }
}

impl<'a> Visitor<'a> for NoUnusedVariables<'a> {
    fn exit_document(&mut self, ctx: &mut ValidatorContext<'a>, _: &'a Document) {
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
                        RuleError::new(&error_message(var.item, *op_name), &[var.start.clone()])
                    })
                    .collect(),
            );
        }
    }

    fn enter_operation_definition(
        &mut self,
        _: &mut ValidatorContext<'a>,
        op: &'a Spanning<Operation>,
    ) {
        let op_name = op.item.name.as_ref().map(|s| s.item);
        self.current_scope = Some(Scope::Operation(op_name));
        self.defined_variables.insert(op_name, HashSet::new());
    }

    fn enter_fragment_definition(
        &mut self,
        _: &mut ValidatorContext<'a>,
        f: &'a Spanning<Fragment>,
    ) {
        self.current_scope = Some(Scope::Fragment(f.item.name.item));
    }

    fn enter_fragment_spread(
        &mut self,
        _: &mut ValidatorContext<'a>,
        spread: &'a Spanning<FragmentSpread>,
    ) {
        if let Some(ref scope) = self.current_scope {
            self.spreads
                .entry(scope.clone())
                .or_insert_with(Vec::new)
                .push(spread.item.name.item);
        }
    }

    fn enter_variable_definition(
        &mut self,
        _: &mut ValidatorContext<'a>,
        &(ref var_name, _): &'a (Spanning<&'a str>, VariableDefinition),
    ) {
        if let Some(Scope::Operation(ref name)) = self.current_scope {
            if let Some(vars) = self.defined_variables.get_mut(name) {
                vars.insert(var_name);
            }
        }
    }

    fn enter_argument(
        &mut self,
        _: &mut ValidatorContext<'a>,
        &(_, ref value): &'a (Spanning<&'a str>, Spanning<InputValue>),
    ) {
        if let Some(ref scope) = self.current_scope {
            self.used_variables
                .entry(scope.clone())
                .or_insert_with(Vec::new)
                .append(&mut value.item.referenced_variables());
        }
    }
}

fn error_message(var_name: &str, op_name: Option<&str>) -> String {
    if let Some(op_name) = op_name {
        format!(
            r#"Variable "${}" is not defined by operation "{}""#,
            var_name, op_name
        )
    } else {
        format!(r#"Variable "${}" is not defined"#, var_name)
    }
}

#[cfg(test)]
mod tests {
    use super::{error_message, factory};

    use parser::SourcePosition;
    use validation::{expect_fails_rule, expect_passes_rule, RuleError};

    #[test]
    fn uses_all_variables() {
        expect_passes_rule(
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
        expect_passes_rule(
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
        expect_passes_rule(
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
        expect_passes_rule(
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
        expect_passes_rule(
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
        expect_passes_rule(
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
        expect_fails_rule(
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
        expect_fails_rule(
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
        expect_fails_rule(
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
        expect_fails_rule(
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
        expect_fails_rule(
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
        expect_fails_rule(
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
