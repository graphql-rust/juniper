use std::collections::hash_map::{Entry, HashMap};

use crate::{
    ast::Fragment,
    parser::{SourcePosition, Spanning},
    validation::{ValidatorContext, Visitor},
    value::ScalarValue,
};

pub struct UniqueFragmentNames<'a> {
    names: HashMap<&'a str, SourcePosition>,
}

pub fn factory<'a>() -> UniqueFragmentNames<'a> {
    UniqueFragmentNames {
        names: HashMap::new(),
    }
}

impl<'a, S> Visitor<'a, S> for UniqueFragmentNames<'a>
where
    S: ScalarValue,
{
    fn enter_fragment_definition(
        &mut self,
        context: &mut ValidatorContext<'a, S>,
        f: &'a Spanning<Fragment<S>>,
    ) {
        match self.names.entry(f.item.name.item) {
            Entry::Occupied(e) => {
                context.report_error(
                    &duplicate_message(f.item.name.item),
                    &[*e.get(), f.item.name.span.start],
                );
            }
            Entry::Vacant(e) => {
                e.insert(f.item.name.span.start);
            }
        }
    }
}

fn duplicate_message(frag_name: &str) -> String {
    format!("There can only be one fragment named {frag_name}")
}

#[cfg(test)]
mod tests {
    use super::{duplicate_message, factory};

    use crate::{
        parser::SourcePosition,
        validation::{expect_fails_rule, expect_passes_rule, RuleError},
        value::DefaultScalarValue,
    };

    #[test]
    fn no_fragments() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          {
            dog {
              name
            }
          }
        "#,
        );
    }

    #[test]
    fn one_fragment() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          {
            dog {
              ...fragA
            }
          }

          fragment fragA on Dog {
            name
          }
        "#,
        );
    }

    #[test]
    fn many_fragments() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          {
            dog {
              ...fragA
              ...fragB
              ...fragC
            }
          }
          fragment fragA on Dog {
            name
          }
          fragment fragB on Dog {
            nickname
          }
          fragment fragC on Dog {
            barkVolume
          }
        "#,
        );
    }

    #[test]
    fn inline_fragments_always_unique() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          {
            dorOrHuman {
              ...on Dog {
                name
              }
              ...on Dog {
                barkVolume
              }
            }
          }
        "#,
        );
    }

    #[test]
    fn fragment_and_operation_named_the_same() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          query Foo {
            dog {
              ...Foo
            }
          }
          fragment Foo on Dog {
            name
          }
        "#,
        );
    }

    #[test]
    fn fragments_named_the_same() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          {
            dog {
              ...fragA
            }
          }
          fragment fragA on Dog {
            name
          }
          fragment fragA on Dog {
            barkVolume
          }
        "#,
            &[RuleError::new(
                &duplicate_message("fragA"),
                &[
                    SourcePosition::new(99, 6, 19),
                    SourcePosition::new(162, 9, 19),
                ],
            )],
        );
    }

    #[test]
    fn fragments_named_the_same_no_reference() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment fragA on Dog {
            name
          }
          fragment fragA on Dog {
            barkVolume
          }
        "#,
            &[RuleError::new(
                &duplicate_message("fragA"),
                &[
                    SourcePosition::new(20, 1, 19),
                    SourcePosition::new(83, 4, 19),
                ],
            )],
        );
    }
}
