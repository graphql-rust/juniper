use crate::{
    ast::FragmentSpread,
    parser::Spanning,
    validation::{ValidatorContext, Visitor},
    value::ScalarValue,
};

pub struct KnownFragmentNames;

pub fn factory() -> KnownFragmentNames {
    KnownFragmentNames
}

impl<'a, S> Visitor<'a, S> for KnownFragmentNames
where
    S: ScalarValue,
{
    fn enter_fragment_spread(
        &mut self,
        context: &mut ValidatorContext<'a, S>,
        spread: &'a Spanning<FragmentSpread<S>>,
    ) {
        let spread_name = &spread.item.name;
        if !context.is_known_fragment(spread_name.item) {
            context.report_error(&error_message(spread_name.item), &[spread_name.span.start]);
        }
    }
}

fn error_message(frag_name: &str) -> String {
    format!(r#"Unknown fragment: "{frag_name}""#)
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
    fn known() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          {
            human(id: 4) {
              ...HumanFields1
              ... on Human {
                ...HumanFields2
              }
              ... {
                name
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
    fn unknown() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          {
            human(id: 4) {
              ...UnknownFragment1
              ... on Human {
                ...UnknownFragment2
              }
            }
          }
          fragment HumanFields on Human {
            name
            ...UnknownFragment3
          }
        "#,
            &[
                RuleError::new(
                    &error_message("UnknownFragment1"),
                    &[SourcePosition::new(57, 3, 17)],
                ),
                RuleError::new(
                    &error_message("UnknownFragment2"),
                    &[SourcePosition::new(122, 5, 19)],
                ),
                RuleError::new(
                    &error_message("UnknownFragment3"),
                    &[SourcePosition::new(255, 11, 15)],
                ),
            ],
        );
    }
}
