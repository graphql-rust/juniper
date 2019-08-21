use std::collections::{HashMap, HashSet};

use crate::{
    ast::{Definition, Document, Fragment, FragmentSpread, Operation},
    parser::Spanning,
    validation::{ValidatorContext, Visitor},
    value::ScalarValue,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Scope<'a> {
    Operation(Option<&'a str>),
    Fragment(&'a str),
}

pub struct NoUnusedFragments<'a> {
    spreads: HashMap<Scope<'a>, Vec<&'a str>>,
    defined_fragments: HashSet<Spanning<&'a str>>,
    current_scope: Option<Scope<'a>>,
}

pub fn factory<'a>() -> NoUnusedFragments<'a> {
    NoUnusedFragments {
        spreads: HashMap::new(),
        defined_fragments: HashSet::new(),
        current_scope: None,
    }
}

impl<'a> NoUnusedFragments<'a> {
    fn find_reachable_fragments(&self, from: &Scope<'a>, result: &mut HashSet<&'a str>) {
        if let Scope::Fragment(name) = *from {
            if result.contains(name) {
                return;
            } else {
                result.insert(name);
            }
        }

        if let Some(spreads) = self.spreads.get(from) {
            for spread in spreads {
                self.find_reachable_fragments(&Scope::Fragment(spread), result)
            }
        }
    }
}

impl<'a, S> Visitor<'a, S> for NoUnusedFragments<'a>
where
    S: ScalarValue,
{
    fn exit_document(&mut self, ctx: &mut ValidatorContext<'a, S>, defs: &'a Document<S>) {
        let mut reachable = HashSet::new();

        for def in defs {
            if let Definition::Operation(Spanning {
                item: Operation { ref name, .. },
                ..
            }) = *def
            {
                let op_name = name.as_ref().map(|s| s.item);
                self.find_reachable_fragments(&Scope::Operation(op_name), &mut reachable);
            }
        }

        for fragment in &self.defined_fragments {
            if !reachable.contains(&fragment.item) {
                ctx.report_error(&error_message(fragment.item), &[fragment.start]);
            }
        }
    }

    fn enter_operation_definition(
        &mut self,
        _: &mut ValidatorContext<'a, S>,
        op: &'a Spanning<Operation<S>>,
    ) {
        let op_name = op.item.name.as_ref().map(|s| s.item);
        self.current_scope = Some(Scope::Operation(op_name));
    }

    fn enter_fragment_definition(
        &mut self,
        _: &mut ValidatorContext<'a, S>,
        f: &'a Spanning<Fragment<S>>,
    ) {
        self.current_scope = Some(Scope::Fragment(f.item.name.item));
        self.defined_fragments
            .insert(Spanning::start_end(&f.start, &f.end, f.item.name.item));
    }

    fn enter_fragment_spread(
        &mut self,
        _: &mut ValidatorContext<'a, S>,
        spread: &'a Spanning<FragmentSpread<S>>,
    ) {
        if let Some(ref scope) = self.current_scope {
            self.spreads
                .entry(scope.clone())
                .or_insert_with(Vec::new)
                .push(spread.item.name.item);
        }
    }
}

fn error_message(frag_name: &str) -> String {
    format!(r#"Fragment "{}" is never used"#, frag_name)
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
    fn all_fragment_names_are_used() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          {
            human(id: 4) {
              ...HumanFields1
              ... on Human {
                ...HumanFields2
              }
            }
          }
          fragment HumanFields1 on Human {
            name
            ...HumanFields3
          }
          fragment HumanFields2 on Human {
            name
          }
          fragment HumanFields3 on Human {
            name
          }
        "#,
        );
    }

    #[test]
    fn all_fragment_names_are_used_by_multiple_operations() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          query Foo {
            human(id: 4) {
              ...HumanFields1
            }
          }
          query Bar {
            human(id: 4) {
              ...HumanFields2
            }
          }
          fragment HumanFields1 on Human {
            name
            ...HumanFields3
          }
          fragment HumanFields2 on Human {
            name
          }
          fragment HumanFields3 on Human {
            name
          }
        "#,
        );
    }

    #[test]
    fn contains_unknown_fragments() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          query Foo {
            human(id: 4) {
              ...HumanFields1
            }
          }
          query Bar {
            human(id: 4) {
              ...HumanFields2
            }
          }
          fragment HumanFields1 on Human {
            name
            ...HumanFields3
          }
          fragment HumanFields2 on Human {
            name
          }
          fragment HumanFields3 on Human {
            name
          }
          fragment Unused1 on Human {
            name
          }
          fragment Unused2 on Human {
            name
          }
        "#,
            &[
                RuleError::new(
                    &error_message("Unused1"),
                    &[SourcePosition::new(465, 21, 10)],
                ),
                RuleError::new(
                    &error_message("Unused2"),
                    &[SourcePosition::new(532, 24, 10)],
                ),
            ],
        );
    }

    #[test]
    fn contains_unknown_fragments_with_ref_cycle() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          query Foo {
            human(id: 4) {
              ...HumanFields1
            }
          }
          query Bar {
            human(id: 4) {
              ...HumanFields2
            }
          }
          fragment HumanFields1 on Human {
            name
            ...HumanFields3
          }
          fragment HumanFields2 on Human {
            name
          }
          fragment HumanFields3 on Human {
            name
          }
          fragment Unused1 on Human {
            name
            ...Unused2
          }
          fragment Unused2 on Human {
            name
            ...Unused1
          }
        "#,
            &[
                RuleError::new(
                    &error_message("Unused1"),
                    &[SourcePosition::new(465, 21, 10)],
                ),
                RuleError::new(
                    &error_message("Unused2"),
                    &[SourcePosition::new(555, 25, 10)],
                ),
            ],
        );
    }

    #[test]
    fn contains_unknown_and_undef_fragments() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          query Foo {
            human(id: 4) {
              ...bar
            }
          }
          fragment foo on Human {
            name
          }
        "#,
            &[RuleError::new(
                &error_message("foo"),
                &[SourcePosition::new(107, 6, 10)],
            )],
        );
    }
}
