use std::fmt::Debug;

use crate::{
    ast::{Definition, Document, FragmentSpread, InlineFragment},
    meta::InterfaceMeta,
    parser::Spanning,
    schema::meta::MetaType,
    validation::{ValidatorContext, Visitor},
    value::ScalarValue,
};
use std::collections::HashMap;

pub struct PossibleFragmentSpreads<'a, S: Debug + 'a> {
    fragment_types: HashMap<&'a str, &'a MetaType<'a, S>>,
}

pub fn factory<'a, S: Debug>() -> PossibleFragmentSpreads<'a, S> {
    PossibleFragmentSpreads {
        fragment_types: HashMap::new(),
    }
}

impl<'a, S> Visitor<'a, S> for PossibleFragmentSpreads<'a, S>
where
    S: ScalarValue,
{
    fn enter_document(&mut self, ctx: &mut ValidatorContext<'a, S>, defs: &'a Document<S>) {
        for def in defs {
            if let Definition::Fragment(Spanning { ref item, .. }) = *def {
                if let Some(t) = ctx.schema.concrete_type_by_name(item.type_condition.item) {
                    self.fragment_types.insert(item.name.item, t);
                }
            }
        }
    }

    fn enter_inline_fragment(
        &mut self,
        ctx: &mut ValidatorContext<'a, S>,
        frag: &'a Spanning<InlineFragment<S>>,
    ) {
        if let (Some(parent_type), Some(frag_type)) = (
            ctx.parent_type(),
            frag.item
                .type_condition
                .as_ref()
                .and_then(|s| ctx.schema.concrete_type_by_name(s.item)),
        ) {
            // Even if there is no object type in the overlap of interfaces
            // implementers, it's OK to spread in case `frag_type` implements
            // `parent_type`.
            // https://spec.graphql.org/October2021#sel-JALVFJNRDABABqDy5B
            if let MetaType::Interface(InterfaceMeta {
                interface_names, ..
            }) = frag_type
            {
                let implements_parent = parent_type
                    .name()
                    .map(|parent| interface_names.iter().any(|i| i == parent))
                    .unwrap_or_default();
                if implements_parent {
                    return;
                }
            }

            if !ctx.schema.type_overlap(parent_type, frag_type) {
                ctx.report_error(
                    &error_message(
                        None,
                        parent_type.name().unwrap_or("<unknown>"),
                        frag_type.name().unwrap_or("<unknown>"),
                    ),
                    &[frag.span.start],
                );
            }
        }
    }

    fn enter_fragment_spread(
        &mut self,
        ctx: &mut ValidatorContext<'a, S>,
        spread: &'a Spanning<FragmentSpread<S>>,
    ) {
        if let (Some(parent_type), Some(frag_type)) = (
            ctx.parent_type(),
            self.fragment_types.get(spread.item.name.item),
        ) {
            // Even if there is no object type in the overlap of interfaces
            // implementers, it's OK to spread in case `frag_type` implements
            // `parent_type`.
            // https://spec.graphql.org/October2021/#sel-JALVFJNRDABABqDy5B
            if let MetaType::Interface(InterfaceMeta {
                interface_names, ..
            }) = frag_type
            {
                let implements_parent = parent_type
                    .name()
                    .map(|parent| interface_names.iter().any(|i| i == parent))
                    .unwrap_or_default();
                if implements_parent {
                    return;
                }
            }

            if !ctx.schema.type_overlap(parent_type, frag_type) {
                ctx.report_error(
                    &error_message(
                        Some(spread.item.name.item),
                        parent_type.name().unwrap_or("<unknown>"),
                        frag_type.name().unwrap_or("<unknown>"),
                    ),
                    &[spread.span.start],
                );
            }
        }
    }
}

fn error_message(frag_name: Option<&str>, parent_type_name: &str, frag_type: &str) -> String {
    if let Some(frag_name) = frag_name {
        format!(
            "Fragment \"{frag_name}\" cannot be spread here as objects of type \
             \"{parent_type_name}\" can never be of type \"{frag_type}\"",
        )
    } else {
        format!(
            "Fragment cannot be spread here as objects of type \
             \"{parent_type_name}\" can never be of type \"{frag_type}\"",
        )
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
    fn of_the_same_object() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment objectWithinObject on Dog { ...dogFragment }
          fragment dogFragment on Dog { barkVolume }
        "#,
        );
    }

    #[test]
    fn of_the_same_object_with_inline_fragment() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment objectWithinObjectAnon on Dog { ... on Dog { barkVolume } }
        "#,
        );
    }

    #[test]
    fn object_into_an_implemented_interface() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment objectWithinInterface on Pet { ...dogFragment }
          fragment dogFragment on Dog { barkVolume }
        "#,
        );
    }

    #[test]
    fn object_into_containing_union() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment objectWithinUnion on CatOrDog { ...dogFragment }
          fragment dogFragment on Dog { barkVolume }
        "#,
        );
    }

    #[test]
    fn union_into_contained_object() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment unionWithinObject on Dog { ...catOrDogFragment }
          fragment catOrDogFragment on CatOrDog { __typename }
        "#,
        );
    }

    #[test]
    fn union_into_overlapping_interface() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment unionWithinInterface on Pet { ...catOrDogFragment }
          fragment catOrDogFragment on CatOrDog { __typename }
        "#,
        );
    }

    #[test]
    fn union_into_overlapping_union() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment unionWithinUnion on DogOrHuman { ...catOrDogFragment }
          fragment catOrDogFragment on CatOrDog { __typename }
        "#,
        );
    }

    #[test]
    fn interface_into_implemented_object() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment interfaceWithinObject on Dog { ...petFragment }
          fragment petFragment on Pet { name }
        "#,
        );
    }

    #[test]
    fn interface_into_overlapping_interface() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment interfaceWithinInterface on Pet { ...beingFragment }
          fragment beingFragment on Being { name }
        "#,
        );
    }

    #[test]
    fn interface_into_overlapping_interface_in_inline_fragment() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment interfaceWithinInterface on Pet { ... on Being { name } }
        "#,
        );
    }

    #[test]
    fn interface_into_overlapping_union() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment interfaceWithinUnion on CatOrDog { ...petFragment }
          fragment petFragment on Pet { name }
        "#,
        );
    }

    #[test]
    fn no_object_overlap_but_implements_parent() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment beingFragment on Being { ...unpopulatedFragment }
          fragment unpopulatedFragment on Unpopulated { name }
        "#,
        );
    }

    #[test]
    fn no_object_overlap_but_implements_parent_inline() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment beingFragment on Being { ...on Unpopulated { name } }
        "#,
        );
    }

    #[test]
    fn different_object_into_object() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment invalidObjectWithinObject on Cat { ...dogFragment }
          fragment dogFragment on Dog { barkVolume }
        "#,
            &[RuleError::new(
                &error_message(Some("dogFragment"), "Cat", "Dog"),
                &[SourcePosition::new(55, 1, 54)],
            )],
        );
    }

    #[test]
    fn different_object_into_object_in_inline_fragment() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment invalidObjectWithinObjectAnon on Cat {
            ... on Dog { barkVolume }
          }
        "#,
            &[RuleError::new(
                &error_message(None, "Cat", "Dog"),
                &[SourcePosition::new(71, 2, 12)],
            )],
        );
    }

    #[test]
    fn object_into_not_implementing_interface() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment invalidObjectWithinInterface on Pet { ...humanFragment }
          fragment humanFragment on Human { pets { name } }
        "#,
            &[RuleError::new(
                &error_message(Some("humanFragment"), "Pet", "Human"),
                &[SourcePosition::new(58, 1, 57)],
            )],
        );
    }

    #[test]
    fn object_into_not_containing_union() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment invalidObjectWithinUnion on CatOrDog { ...humanFragment }
          fragment humanFragment on Human { pets { name } }
        "#,
            &[RuleError::new(
                &error_message(Some("humanFragment"), "CatOrDog", "Human"),
                &[SourcePosition::new(59, 1, 58)],
            )],
        );
    }

    #[test]
    fn union_into_not_contained_object() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment invalidUnionWithinObject on Human { ...catOrDogFragment }
          fragment catOrDogFragment on CatOrDog { __typename }
        "#,
            &[RuleError::new(
                &error_message(Some("catOrDogFragment"), "Human", "CatOrDog"),
                &[SourcePosition::new(56, 1, 55)],
            )],
        );
    }

    #[test]
    fn union_into_non_overlapping_interface() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment invalidUnionWithinInterface on Pet { ...humanOrAlienFragment }
          fragment humanOrAlienFragment on HumanOrAlien { __typename }
        "#,
            &[RuleError::new(
                &error_message(Some("humanOrAlienFragment"), "Pet", "HumanOrAlien"),
                &[SourcePosition::new(57, 1, 56)],
            )],
        );
    }

    #[test]
    fn union_into_non_overlapping_union() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment invalidUnionWithinUnion on CatOrDog { ...humanOrAlienFragment }
          fragment humanOrAlienFragment on HumanOrAlien { __typename }
        "#,
            &[RuleError::new(
                &error_message(Some("humanOrAlienFragment"), "CatOrDog", "HumanOrAlien"),
                &[SourcePosition::new(58, 1, 57)],
            )],
        );
    }

    #[test]
    fn interface_into_non_implementing_object() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment invalidInterfaceWithinObject on Cat { ...intelligentFragment }
          fragment intelligentFragment on Intelligent { iq }
        "#,
            &[RuleError::new(
                &error_message(Some("intelligentFragment"), "Cat", "Intelligent"),
                &[SourcePosition::new(58, 1, 57)],
            )],
        );
    }

    #[test]
    fn interface_into_non_overlapping_interface() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment invalidInterfaceWithinInterface on Pet {
            ...intelligentFragment
          }
          fragment intelligentFragment on Intelligent { iq }
        "#,
            &[RuleError::new(
                &error_message(Some("intelligentFragment"), "Pet", "Intelligent"),
                &[SourcePosition::new(73, 2, 12)],
            )],
        );
    }

    #[test]
    fn interface_into_non_overlapping_interface_in_inline_fragment() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment invalidInterfaceWithinInterfaceAnon on Pet {
            ...on Intelligent { iq }
          }
        "#,
            &[RuleError::new(
                &error_message(None, "Pet", "Intelligent"),
                &[SourcePosition::new(77, 2, 12)],
            )],
        );
    }

    #[test]
    fn interface_into_non_overlapping_union() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment invalidInterfaceWithinUnion on HumanOrAlien { ...petFragment }
          fragment petFragment on Pet { name }
        "#,
            &[RuleError::new(
                &error_message(Some("petFragment"), "HumanOrAlien", "Pet"),
                &[SourcePosition::new(66, 1, 65)],
            )],
        );
    }
}
