use std::collections::{HashMap, HashSet};

use crate::{
    ast::{Document, Fragment, FragmentSpread},
    parser::Spanning,
    validation::{RuleError, ValidatorContext, Visitor},
    value::ScalarValue,
    Span,
};

pub fn factory<'a>() -> NoFragmentCycles<'a> {
    NoFragmentCycles {
        current_fragment: None,
        spreads: HashMap::new(),
        fragment_order: Vec::new(),
    }
}

type BorrowedSpanning<'a, T> = Spanning<&'a T, &'a Span>;

pub struct NoFragmentCycles<'a> {
    current_fragment: Option<&'a str>,
    spreads: HashMap<&'a str, Vec<BorrowedSpanning<'a, str>>>,
    fragment_order: Vec<&'a str>,
}

impl<'a, S> Visitor<'a, S> for NoFragmentCycles<'a>
where
    S: ScalarValue,
{
    fn exit_document(&mut self, ctx: &mut ValidatorContext<'a, S>, _: &'a Document<S>) {
        assert!(self.current_fragment.is_none());

        let mut detector = CycleDetector {
            visited: HashSet::new(),
            spreads: &self.spreads,
            errors: Vec::new(),
        };

        for frag in &self.fragment_order {
            if !detector.visited.contains(frag) {
                detector.detect_from(frag);
            }
        }

        ctx.append_errors(detector.errors);
    }

    fn enter_fragment_definition(
        &mut self,
        _: &mut ValidatorContext<'a, S>,
        fragment: &'a Spanning<Fragment<S>>,
    ) {
        assert!(self.current_fragment.is_none());

        let fragment_name = &fragment.item.name.item;
        self.current_fragment = Some(fragment_name);
        self.fragment_order.push(fragment_name);
    }

    fn exit_fragment_definition(
        &mut self,
        _: &mut ValidatorContext<'a, S>,
        fragment: &'a Spanning<Fragment<S>>,
    ) {
        assert_eq!(Some(fragment.item.name.item), self.current_fragment);
        self.current_fragment = None;
    }

    fn enter_fragment_spread(
        &mut self,
        _: &mut ValidatorContext<'a, S>,
        spread: &'a Spanning<FragmentSpread<S>>,
    ) {
        if let Some(current_fragment) = self.current_fragment {
            self.spreads
                .entry(current_fragment)
                .or_default()
                .push(BorrowedSpanning {
                    item: spread.item.name.item,
                    span: &spread.span,
                });
        }
    }
}

type CycleDetectorState<'a> = (
    &'a str,
    Vec<&'a BorrowedSpanning<'a, str>>,
    HashMap<&'a str, usize>,
);

struct CycleDetector<'a> {
    visited: HashSet<&'a str>,
    spreads: &'a HashMap<&'a str, Vec<BorrowedSpanning<'a, str>>>,
    errors: Vec<RuleError>,
}

impl<'a> CycleDetector<'a> {
    fn detect_from(&mut self, from: &'a str) {
        let mut to_visit = Vec::new();
        to_visit.push((from, Vec::new(), HashMap::new()));

        while let Some((from, path, path_indices)) = to_visit.pop() {
            to_visit.extend(self.detect_from_inner(from, path, path_indices));
        }
    }

    /// This function should be called only inside [`Self::detect_from()`], as
    /// it's a recursive function using heap instead of a stack. So, instead of
    /// the recursive call, we return a [`Vec`] that is visited inside
    /// [`Self::detect_from()`].
    fn detect_from_inner(
        &mut self,
        from: &'a str,
        path: Vec<&'a BorrowedSpanning<'a, str>>,
        mut path_indices: HashMap<&'a str, usize>,
    ) -> Vec<CycleDetectorState<'a>> {
        self.visited.insert(from);

        if !self.spreads.contains_key(from) {
            return Vec::new();
        }

        path_indices.insert(from, path.len());

        let mut to_visit = Vec::new();
        for node in &self.spreads[from] {
            let name = node.item;
            let index = path_indices.get(name).cloned();

            if let Some(index) = index {
                let err_pos = if index < path.len() {
                    path[index]
                } else {
                    node
                };

                self.errors
                    .push(RuleError::new(&error_message(name), &[err_pos.span.start]));
            } else {
                let mut path = path.clone();
                path.push(node);
                to_visit.push((name, path, path_indices.clone()));
            }
        }

        to_visit
    }
}

fn error_message(frag_name: &str) -> String {
    format!(r#"Cannot spread fragment "{frag_name}""#)
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
    fn single_reference_is_valid() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment fragA on Dog { ...fragB }
          fragment fragB on Dog { name }
        "#,
        );
    }

    #[test]
    fn spreading_twice_is_not_circular() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment fragA on Dog { ...fragB, ...fragB }
          fragment fragB on Dog { name }
        "#,
        );
    }

    #[test]
    fn spreading_twice_indirectly_is_not_circular() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment fragA on Dog { ...fragB, ...fragC }
          fragment fragB on Dog { ...fragC }
          fragment fragC on Dog { name }
        "#,
        );
    }

    #[test]
    fn double_spread_within_abstract_types() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment nameFragment on Pet {
            ... on Dog { name }
            ... on Cat { name }
          }

          fragment spreadsInAnon on Pet {
            ... on Dog { ...nameFragment }
            ... on Cat { ...nameFragment }
          }
        "#,
        );
    }

    #[test]
    fn does_not_false_positive_on_unknown_fragment() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment nameFragment on Pet {
            ...UnknownFragment
          }
        "#,
        );
    }

    #[test]
    fn spreading_recursively_within_field_fails() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment fragA on Human { relatives { ...fragA } },
        "#,
            &[RuleError::new(
                &error_message("fragA"),
                &[SourcePosition::new(49, 1, 48)],
            )],
        );
    }

    #[test]
    fn no_spreading_itself_directly() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment fragA on Dog { ...fragA }
        "#,
            &[RuleError::new(
                &error_message("fragA"),
                &[SourcePosition::new(35, 1, 34)],
            )],
        );
    }

    #[test]
    fn no_spreading_itself_directly_within_inline_fragment() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment fragA on Pet {
            ... on Dog {
              ...fragA
            }
          }
        "#,
            &[RuleError::new(
                &error_message("fragA"),
                &[SourcePosition::new(74, 3, 14)],
            )],
        );
    }

    #[test]
    fn no_spreading_itself_indirectly() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment fragA on Dog { ...fragB }
          fragment fragB on Dog { ...fragA }
        "#,
            &[RuleError::new(
                &error_message("fragA"),
                &[SourcePosition::new(35, 1, 34)],
            )],
        );
    }

    #[test]
    fn no_spreading_itself_indirectly_reports_opposite_order() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment fragB on Dog { ...fragA }
          fragment fragA on Dog { ...fragB }
        "#,
            &[RuleError::new(
                &error_message("fragB"),
                &[SourcePosition::new(35, 1, 34)],
            )],
        );
    }

    #[test]
    fn no_spreading_itself_indirectly_within_inline_fragment() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment fragA on Pet {
            ... on Dog {
              ...fragB
            }
          }
          fragment fragB on Pet {
            ... on Dog {
              ...fragA
            }
          }
        "#,
            &[RuleError::new(
                &error_message("fragA"),
                &[SourcePosition::new(74, 3, 14)],
            )],
        );
    }

    #[test]
    fn no_spreading_itself_deeply() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment fragA on Dog { ...fragB }
          fragment fragB on Dog { ...fragC }
          fragment fragC on Dog { ...fragO }
          fragment fragX on Dog { ...fragY }
          fragment fragY on Dog { ...fragZ }
          fragment fragZ on Dog { ...fragO }
          fragment fragO on Dog { ...fragP }
          fragment fragP on Dog { ...fragA, ...fragX }
        "#,
            &[
                RuleError::new(&error_message("fragA"), &[SourcePosition::new(35, 1, 34)]),
                RuleError::new(&error_message("fragO"), &[SourcePosition::new(305, 7, 34)]),
            ],
        );
    }

    #[test]
    fn no_spreading_itself_deeply_two_paths() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment fragA on Dog { ...fragB, ...fragC }
          fragment fragB on Dog { ...fragA }
          fragment fragC on Dog { ...fragA }
        "#,
            &[
                RuleError::new(&error_message("fragA"), &[SourcePosition::new(35, 1, 34)]),
                RuleError::new(&error_message("fragA"), &[SourcePosition::new(45, 1, 44)]),
            ],
        );
    }

    #[test]
    fn no_spreading_itself_deeply_two_paths_alt_traversal_order() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment fragA on Dog { ...fragC }
          fragment fragB on Dog { ...fragC }
          fragment fragC on Dog { ...fragA, ...fragB }
        "#,
            &[
                RuleError::new(&error_message("fragA"), &[SourcePosition::new(35, 1, 34)]),
                RuleError::new(&error_message("fragC"), &[SourcePosition::new(135, 3, 44)]),
            ],
        );
    }

    #[test]
    fn no_spreading_itself_deeply_and_immediately() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment fragA on Dog { ...fragB }
          fragment fragB on Dog { ...fragB, ...fragC }
          fragment fragC on Dog { ...fragA, ...fragB }
        "#,
            &[
                RuleError::new(&error_message("fragA"), &[SourcePosition::new(35, 1, 34)]),
                RuleError::new(&error_message("fragB"), &[SourcePosition::new(80, 2, 34)]),
                RuleError::new(&error_message("fragB"), &[SourcePosition::new(90, 2, 44)]),
            ],
        );
    }
}
