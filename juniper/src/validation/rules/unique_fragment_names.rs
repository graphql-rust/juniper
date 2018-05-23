use std::collections::hash_map::{Entry, HashMap};

use ast::Fragment;
use parser::{SourcePosition, Spanning};
use validation::{ValidatorContext, Visitor};

pub struct UniqueFragmentNames<'a> {
    names: HashMap<&'a str, SourcePosition>,
}

pub fn factory<'a>() -> UniqueFragmentNames<'a> {
    UniqueFragmentNames {
        names: HashMap::new(),
    }
}

impl<'a> Visitor<'a> for UniqueFragmentNames<'a> {
    fn enter_fragment_definition(
        &mut self,
        context: &mut ValidatorContext<'a>,
        f: &'a Spanning<Fragment>,
    ) {
        match self.names.entry(f.item.name.item) {
            Entry::Occupied(e) => {
                context.report_error(
                    &duplicate_message(f.item.name.item),
                    &[e.get().clone(), f.item.name.start.clone()],
                );
            }
            Entry::Vacant(e) => {
                e.insert(f.item.name.start.clone());
            }
        }
    }
}

fn duplicate_message(frag_name: &str) -> String {
    format!("There can only be one fragment named {}", frag_name)
}

#[cfg(test)]
mod tests {
    use super::{duplicate_message, factory};

    use parser::SourcePosition;
    use validation::{expect_fails_rule, expect_passes_rule, RuleError};

    #[test]
    fn no_fragments() {
        expect_passes_rule(
            factory,
            r#"
          {
            field
          }
        "#,
        );
    }

    #[test]
    fn one_fragment() {
        expect_passes_rule(
            factory,
            r#"
          {
            ...fragA
          }

          fragment fragA on Type {
            field
          }
        "#,
        );
    }

    #[test]
    fn many_fragments() {
        expect_passes_rule(
            factory,
            r#"
          {
            ...fragA
            ...fragB
            ...fragC
          }
          fragment fragA on Type {
            fieldA
          }
          fragment fragB on Type {
            fieldB
          }
          fragment fragC on Type {
            fieldC
          }
        "#,
        );
    }

    #[test]
    fn inline_fragments_always_unique() {
        expect_passes_rule(
            factory,
            r#"
          {
            ...on Type {
              fieldA
            }
            ...on Type {
              fieldB
            }
          }
        "#,
        );
    }

    #[test]
    fn fragment_and_operation_named_the_same() {
        expect_passes_rule(
            factory,
            r#"
          query Foo {
            ...Foo
          }
          fragment Foo on Type {
            field
          }
        "#,
        );
    }

    #[test]
    fn fragments_named_the_same() {
        expect_fails_rule(
            factory,
            r#"
          {
            ...fragA
          }
          fragment fragA on Type {
            fieldA
          }
          fragment fragA on Type {
            fieldB
          }
        "#,
            &[RuleError::new(
                &duplicate_message("fragA"),
                &[
                    SourcePosition::new(65, 4, 19),
                    SourcePosition::new(131, 7, 19),
                ],
            )],
        );
    }

    #[test]
    fn fragments_named_the_same_no_reference() {
        expect_fails_rule(
            factory,
            r#"
          fragment fragA on Type {
            fieldA
          }
          fragment fragA on Type {
            fieldB
          }
        "#,
            &[RuleError::new(
                &duplicate_message("fragA"),
                &[
                    SourcePosition::new(20, 1, 19),
                    SourcePosition::new(86, 4, 19),
                ],
            )],
        );
    }
}
