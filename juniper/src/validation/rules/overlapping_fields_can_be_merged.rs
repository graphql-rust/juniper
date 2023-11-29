use std::{borrow::Borrow, cell::RefCell, collections::HashMap, fmt::Debug, hash::Hash};

use crate::{
    ast::{Arguments, Definition, Document, Field, Fragment, FragmentSpread, Selection, Type},
    parser::{SourcePosition, Spanning},
    schema::meta::{Field as FieldType, MetaType},
    validation::{ValidatorContext, Visitor},
    value::ScalarValue,
};

#[derive(Debug)]
struct Conflict(ConflictReason, Vec<SourcePosition>, Vec<SourcePosition>);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ConflictReason(String, ConflictReasonMessage);

#[derive(Debug)]
struct AstAndDef<'a, S: Debug + 'a>(
    Option<&'a str>,
    &'a Spanning<Field<'a, S>>,
    Option<&'a FieldType<'a, S>>,
);

type AstAndDefCollection<'a, S> = OrderedMap<&'a str, Vec<AstAndDef<'a, S>>>;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum ConflictReasonMessage {
    Message(String),
    Nested(Vec<ConflictReason>),
}

struct PairSet<'a> {
    data: HashMap<&'a str, HashMap<&'a str, bool>>,
}

#[derive(Debug)]
struct OrderedMap<K, V> {
    data: HashMap<K, V>,
    insert_order: Vec<K>,
}

struct OrderedMapIter<'a, K: 'a, V: 'a> {
    map: &'a HashMap<K, V>,
    inner: ::std::slice::Iter<'a, K>,
}

impl<K: Eq + Hash + Clone, V> OrderedMap<K, V> {
    fn new() -> OrderedMap<K, V> {
        OrderedMap {
            data: HashMap::new(),
            insert_order: Vec::new(),
        }
    }

    fn iter(&self) -> OrderedMapIter<K, V> {
        OrderedMapIter {
            map: &self.data,
            inner: self.insert_order.iter(),
        }
    }

    fn get<Q: ?Sized>(&self, k: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.data.get(k)
    }

    fn get_mut<Q: ?Sized>(&mut self, k: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.data.get_mut(k)
    }

    fn contains_key<Q: ?Sized>(&self, k: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.data.contains_key(k)
    }

    fn insert(&mut self, k: K, v: V) -> Option<V> {
        let result = self.data.insert(k.clone(), v);
        if result.is_none() {
            self.insert_order.push(k);
        }
        result
    }
}

impl<'a, K: Eq + Hash + 'a, V: 'a> Iterator for OrderedMapIter<'a, K, V> {
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner
            .next()
            .and_then(|key| self.map.get(key).map(|value| (key, value)))
    }
}

impl<'a> PairSet<'a> {
    fn new() -> PairSet<'a> {
        PairSet {
            data: HashMap::new(),
        }
    }

    fn contains(&self, a: &'a str, b: &'a str, mutex: bool) -> bool {
        if let Some(result) = self.data.get(a).and_then(|s| s.get(b)) {
            if !mutex {
                !result
            } else {
                true
            }
        } else {
            false
        }
    }

    fn insert(&mut self, a: &'a str, b: &'a str, mutex: bool) {
        self.data.entry(a).or_default().insert(b, mutex);

        self.data.entry(b).or_default().insert(a, mutex);
    }
}

pub struct OverlappingFieldsCanBeMerged<'a, S: Debug + 'a> {
    named_fragments: HashMap<&'a str, &'a Fragment<'a, S>>,
    compared_fragments: RefCell<PairSet<'a>>,
}

pub fn factory<'a, S: Debug>() -> OverlappingFieldsCanBeMerged<'a, S> {
    OverlappingFieldsCanBeMerged {
        named_fragments: HashMap::new(),
        compared_fragments: RefCell::new(PairSet::new()),
    }
}

impl<'a, S: Debug> OverlappingFieldsCanBeMerged<'a, S> {
    fn find_conflicts_within_selection_set(
        &self,
        parent_type: Option<&'a MetaType<S>>,
        selection_set: &'a [Selection<S>],
        ctx: &ValidatorContext<'a, S>,
    ) -> Vec<Conflict>
    where
        S: ScalarValue,
    {
        let mut conflicts = Vec::new();

        let (field_map, fragment_names) =
            self.get_fields_and_fragment_names(parent_type, selection_set, ctx);

        self.collect_conflicts_within(&mut conflicts, &field_map, ctx);

        for (i, frag_name1) in fragment_names.iter().enumerate() {
            self.collect_conflicts_between_fields_and_fragment(
                &mut conflicts,
                &field_map,
                frag_name1,
                false,
                ctx,
            );

            for frag_name2 in &fragment_names[i + 1..] {
                self.collect_conflicts_between_fragments(
                    &mut conflicts,
                    frag_name1,
                    frag_name2,
                    false,
                    ctx,
                );
            }
        }

        conflicts
    }

    fn collect_conflicts_between_fragments(
        &self,
        conflicts: &mut Vec<Conflict>,
        fragment_name1: &'a str,
        fragment_name2: &'a str,
        mutually_exclusive: bool,
        ctx: &ValidatorContext<'a, S>,
    ) where
        S: ScalarValue,
    {
        // Early return on fragment recursion, as it makes no sense.
        // Fragment recursions are prevented by `no_fragment_cycles` validator.
        if fragment_name1 == fragment_name2 {
            return;
        }

        let fragment1 = match self.named_fragments.get(fragment_name1) {
            Some(f) => f,
            None => return,
        };

        let fragment2 = match self.named_fragments.get(fragment_name2) {
            Some(f) => f,
            None => return,
        };

        {
            if self.compared_fragments.borrow().contains(
                fragment_name1,
                fragment_name2,
                mutually_exclusive,
            ) {
                return;
            }
        }

        {
            self.compared_fragments.borrow_mut().insert(
                fragment_name1,
                fragment_name2,
                mutually_exclusive,
            );
        }

        let (field_map1, fragment_names1) =
            self.get_referenced_fields_and_fragment_names(fragment1, ctx);
        let (field_map2, fragment_names2) =
            self.get_referenced_fields_and_fragment_names(fragment2, ctx);

        self.collect_conflicts_between(
            conflicts,
            mutually_exclusive,
            &field_map1,
            &field_map2,
            ctx,
        );

        for fragment_name2 in &fragment_names2 {
            self.collect_conflicts_between_fragments(
                conflicts,
                fragment_name1,
                fragment_name2,
                mutually_exclusive,
                ctx,
            );
        }

        for fragment_name1 in &fragment_names1 {
            self.collect_conflicts_between_fragments(
                conflicts,
                fragment_name1,
                fragment_name2,
                mutually_exclusive,
                ctx,
            );
        }
    }

    fn collect_conflicts_between_fields_and_fragment(
        &self,
        conflicts: &mut Vec<Conflict>,
        field_map: &AstAndDefCollection<'a, S>,
        fragment_name: &str,
        mutually_exclusive: bool,
        ctx: &ValidatorContext<'a, S>,
    ) where
        S: ScalarValue,
    {
        let mut to_check = Vec::new();
        if let Some(fragments) = self.collect_conflicts_between_fields_and_fragment_inner(
            conflicts,
            field_map,
            fragment_name,
            mutually_exclusive,
            ctx,
        ) {
            to_check.push((fragment_name, fragments))
        }

        while let Some((fragment_name, fragment_names2)) = to_check.pop() {
            for fragment_name2 in fragment_names2 {
                // Early return on fragment recursion, as it makes no sense.
                // Fragment recursions are prevented by `no_fragment_cycles` validator.
                if fragment_name == fragment_name2 {
                    return;
                }
                if let Some(fragments) = self.collect_conflicts_between_fields_and_fragment_inner(
                    conflicts,
                    field_map,
                    fragment_name2,
                    mutually_exclusive,
                    ctx,
                ) {
                    to_check.push((fragment_name2, fragments));
                };
            }
        }
    }

    /// This function should be called only inside
    /// [`Self::collect_conflicts_between_fields_and_fragment()`], as it's a
    /// recursive function using heap instead of a stack. So, instead of the
    /// recursive call, we return a [`Vec`] that is visited inside
    /// [`Self::collect_conflicts_between_fields_and_fragment()`].
    fn collect_conflicts_between_fields_and_fragment_inner(
        &self,
        conflicts: &mut Vec<Conflict>,
        field_map: &AstAndDefCollection<'a, S>,
        fragment_name: &str,
        mutually_exclusive: bool,
        ctx: &ValidatorContext<'a, S>,
    ) -> Option<Vec<&'a str>>
    where
        S: ScalarValue,
    {
        let fragment = self.named_fragments.get(fragment_name)?;

        let (field_map2, fragment_names2) =
            self.get_referenced_fields_and_fragment_names(fragment, ctx);

        self.collect_conflicts_between(conflicts, mutually_exclusive, field_map, &field_map2, ctx);

        Some(fragment_names2)
    }

    fn collect_conflicts_between(
        &self,
        conflicts: &mut Vec<Conflict>,
        mutually_exclusive: bool,
        field_map1: &AstAndDefCollection<'a, S>,
        field_map2: &AstAndDefCollection<'a, S>,
        ctx: &ValidatorContext<'a, S>,
    ) where
        S: ScalarValue,
    {
        for (response_name, fields1) in field_map1.iter() {
            if let Some(fields2) = field_map2.get(response_name) {
                for field1 in fields1 {
                    for field2 in fields2 {
                        if let Some(conflict) = self.find_conflict(
                            response_name,
                            field1,
                            field2,
                            mutually_exclusive,
                            ctx,
                        ) {
                            conflicts.push(conflict);
                        }
                    }
                }
            }
        }
    }

    fn collect_conflicts_within(
        &self,
        conflicts: &mut Vec<Conflict>,
        field_map: &AstAndDefCollection<'a, S>,
        ctx: &ValidatorContext<'a, S>,
    ) where
        S: ScalarValue,
    {
        for (response_name, fields) in field_map.iter() {
            for (i, field1) in fields.iter().enumerate() {
                for field2 in &fields[i + 1..] {
                    if let Some(conflict) =
                        self.find_conflict(response_name, field1, field2, false, ctx)
                    {
                        conflicts.push(conflict);
                    }
                }
            }
        }
    }

    fn find_conflict(
        &self,
        response_name: &str,
        field1: &AstAndDef<'a, S>,
        field2: &AstAndDef<'a, S>,
        parents_mutually_exclusive: bool,
        ctx: &ValidatorContext<'a, S>,
    ) -> Option<Conflict>
    where
        S: ScalarValue,
    {
        let AstAndDef(ref parent_type1, ast1, ref def1) = *field1;
        let AstAndDef(ref parent_type2, ast2, ref def2) = *field2;

        let mutually_exclusive = parents_mutually_exclusive
            || (parent_type1 != parent_type2
                && self.is_object_type(ctx, *parent_type1)
                && self.is_object_type(ctx, *parent_type2));

        if !mutually_exclusive {
            let name1 = &ast1.item.name.item;
            let name2 = &ast2.item.name.item;

            if name1 != name2 {
                return Some(Conflict(
                    ConflictReason(
                        response_name.into(),
                        ConflictReasonMessage::Message(format!(
                            "{name1} and {name2} are different fields",
                        )),
                    ),
                    vec![ast1.span.start],
                    vec![ast2.span.start],
                ));
            }

            if !self.is_same_arguments(&ast1.item.arguments, &ast2.item.arguments) {
                return Some(Conflict(
                    ConflictReason(
                        response_name.into(),
                        ConflictReasonMessage::Message("they have differing arguments".into()),
                    ),
                    vec![ast1.span.start],
                    vec![ast2.span.start],
                ));
            }
        }

        let t1 = def1.as_ref().map(|def| &def.field_type);
        let t2 = def2.as_ref().map(|def| &def.field_type);

        if let (Some(t1), Some(t2)) = (t1, t2) {
            if Self::is_type_conflict(ctx, t1, t2) {
                return Some(Conflict(
                    ConflictReason(
                        response_name.into(),
                        ConflictReasonMessage::Message(format!(
                            "they return conflicting types {t1} and {t2}",
                        )),
                    ),
                    vec![ast1.span.start],
                    vec![ast2.span.start],
                ));
            }
        }

        if let (Some(s1), Some(s2)) = (&ast1.item.selection_set, &ast2.item.selection_set) {
            let conflicts = self.find_conflicts_between_sub_selection_sets(
                mutually_exclusive,
                t1.map(Type::innermost_name),
                s1,
                t2.map(Type::innermost_name),
                s2,
                ctx,
            );

            return self.subfield_conflicts(
                &conflicts,
                response_name,
                &ast1.span.start,
                &ast2.span.start,
            );
        }

        None
    }

    fn find_conflicts_between_sub_selection_sets(
        &self,
        mutually_exclusive: bool,
        parent_type1: Option<&str>,
        selection_set1: &'a [Selection<S>],
        parent_type2: Option<&str>,
        selection_set2: &'a [Selection<S>],
        ctx: &ValidatorContext<'a, S>,
    ) -> Vec<Conflict>
    where
        S: ScalarValue,
    {
        let mut conflicts = Vec::new();

        let parent_type1 = parent_type1.and_then(|t| ctx.schema.concrete_type_by_name(t));
        let parent_type2 = parent_type2.and_then(|t| ctx.schema.concrete_type_by_name(t));

        let (field_map1, fragment_names1) =
            self.get_fields_and_fragment_names(parent_type1, selection_set1, ctx);
        let (field_map2, fragment_names2) =
            self.get_fields_and_fragment_names(parent_type2, selection_set2, ctx);

        self.collect_conflicts_between(
            &mut conflicts,
            mutually_exclusive,
            &field_map1,
            &field_map2,
            ctx,
        );

        for fragment_name in &fragment_names2 {
            self.collect_conflicts_between_fields_and_fragment(
                &mut conflicts,
                &field_map1,
                fragment_name,
                mutually_exclusive,
                ctx,
            );
        }

        for fragment_name in &fragment_names1 {
            self.collect_conflicts_between_fields_and_fragment(
                &mut conflicts,
                &field_map2,
                fragment_name,
                mutually_exclusive,
                ctx,
            );
        }

        for fragment_name1 in &fragment_names1 {
            for fragment_name2 in &fragment_names2 {
                self.collect_conflicts_between_fragments(
                    &mut conflicts,
                    fragment_name1,
                    fragment_name2,
                    mutually_exclusive,
                    ctx,
                );
            }
        }

        conflicts
    }

    fn subfield_conflicts(
        &self,
        conflicts: &[Conflict],
        response_name: &str,
        pos1: &SourcePosition,
        pos2: &SourcePosition,
    ) -> Option<Conflict> {
        if conflicts.is_empty() {
            return None;
        }

        Some(Conflict(
            ConflictReason(
                response_name.into(),
                ConflictReasonMessage::Nested(conflicts.iter().map(|c| c.0.clone()).collect()),
            ),
            vec![*pos1]
                .into_iter()
                .chain(conflicts.iter().flat_map(|Conflict(_, fs1, _)| fs1.clone()))
                .collect(),
            vec![*pos2]
                .into_iter()
                .chain(conflicts.iter().flat_map(|Conflict(_, _, fs2)| fs2.clone()))
                .collect(),
        ))
    }

    fn is_type_conflict(ctx: &ValidatorContext<'a, S>, t1: &Type, t2: &Type) -> bool {
        match (t1, t2) {
            (&Type::List(ref inner1, expected_size1), &Type::List(ref inner2, expected_size2))
            | (
                &Type::NonNullList(ref inner1, expected_size1),
                &Type::NonNullList(ref inner2, expected_size2),
            ) => {
                if expected_size1 != expected_size2 {
                    return false;
                }
                Self::is_type_conflict(ctx, inner1, inner2)
            }
            (&Type::NonNullNamed(ref n1), &Type::NonNullNamed(ref n2))
            | (&Type::Named(ref n1), &Type::Named(ref n2)) => {
                let ct1 = ctx.schema.concrete_type_by_name(n1);
                let ct2 = ctx.schema.concrete_type_by_name(n2);

                if ct1.map(MetaType::is_leaf).unwrap_or(false)
                    || ct2.map(MetaType::is_leaf).unwrap_or(false)
                {
                    n1 != n2
                } else {
                    false
                }
            }
            _ => true,
        }
    }

    fn is_same_arguments(
        &self,
        args1: &Option<Spanning<Arguments<S>>>,
        args2: &Option<Spanning<Arguments<S>>>,
    ) -> bool
    where
        S: ScalarValue,
    {
        match (args1, args2) {
            (&None, &None) => true,
            (
                &Some(Spanning {
                    item: ref args1, ..
                }),
                &Some(Spanning {
                    item: ref args2, ..
                }),
            ) => {
                if args1.len() != args2.len() {
                    return false;
                }

                args1.iter().all(|(n1, v1)| {
                    if let Some((_, v2)) = args2.iter().find(|&(n2, _)| n1.item == n2.item) {
                        v1.item.unlocated_eq(&v2.item)
                    } else {
                        false
                    }
                })
            }
            _ => false,
        }
    }

    fn is_object_type(&self, ctx: &ValidatorContext<'a, S>, type_name: Option<&str>) -> bool {
        let meta = type_name.and_then(|n| ctx.schema.concrete_type_by_name(n));
        matches!(meta, Some(&MetaType::Object(_)))
    }

    fn get_referenced_fields_and_fragment_names(
        &self,
        fragment: &'a Fragment<S>,
        ctx: &ValidatorContext<'a, S>,
    ) -> (AstAndDefCollection<'a, S>, Vec<&'a str>) {
        let fragment_type = ctx
            .schema
            .concrete_type_by_name(fragment.type_condition.item);

        self.get_fields_and_fragment_names(fragment_type, &fragment.selection_set, ctx)
    }

    fn get_fields_and_fragment_names(
        &self,
        parent_type: Option<&'a MetaType<S>>,
        selection_set: &'a [Selection<S>],
        ctx: &ValidatorContext<'a, S>,
    ) -> (AstAndDefCollection<'a, S>, Vec<&'a str>) {
        let mut ast_and_defs = OrderedMap::new();
        let mut fragment_names = Vec::new();

        Self::collect_fields_and_fragment_names(
            parent_type,
            selection_set,
            ctx,
            &mut ast_and_defs,
            &mut fragment_names,
        );

        (ast_and_defs, fragment_names)
    }

    fn collect_fields_and_fragment_names(
        parent_type: Option<&'a MetaType<S>>,
        selection_set: &'a [Selection<S>],
        ctx: &ValidatorContext<'a, S>,
        ast_and_defs: &mut AstAndDefCollection<'a, S>,
        fragment_names: &mut Vec<&'a str>,
    ) {
        for selection in selection_set {
            match *selection {
                Selection::Field(ref f) => {
                    let field_name = &f.item.name.item;
                    let field_def = parent_type.and_then(|t| t.field_by_name(field_name));
                    let response_name =
                        f.item.alias.as_ref().map(|s| &s.item).unwrap_or(field_name);

                    if !ast_and_defs.contains_key(response_name) {
                        ast_and_defs.insert(response_name, Vec::new());
                    }

                    ast_and_defs.get_mut(response_name).unwrap().push(AstAndDef(
                        parent_type.and_then(MetaType::name),
                        f,
                        field_def,
                    ));
                }
                Selection::FragmentSpread(Spanning {
                    item: FragmentSpread { ref name, .. },
                    ..
                }) => {
                    if !fragment_names.iter().any(|n| *n == name.item) {
                        fragment_names.push(name.item);
                    }
                }
                Selection::InlineFragment(Spanning {
                    item: ref inline, ..
                }) => {
                    let parent_type = inline
                        .type_condition
                        .as_ref()
                        .and_then(|cond| ctx.schema.concrete_type_by_name(cond.item))
                        .or(parent_type);

                    Self::collect_fields_and_fragment_names(
                        parent_type,
                        &inline.selection_set,
                        ctx,
                        ast_and_defs,
                        fragment_names,
                    );
                }
            }
        }
    }
}

impl<'a, S> Visitor<'a, S> for OverlappingFieldsCanBeMerged<'a, S>
where
    S: ScalarValue,
{
    fn enter_document(&mut self, _: &mut ValidatorContext<'a, S>, defs: &'a Document<S>) {
        for def in defs {
            if let Definition::Fragment(Spanning { ref item, .. }) = *def {
                self.named_fragments.insert(item.name.item, item);
            }
        }
    }

    fn enter_selection_set(
        &mut self,
        ctx: &mut ValidatorContext<'a, S>,
        selection_set: &'a [Selection<S>],
    ) {
        for Conflict(ConflictReason(reason_name, reason_msg), mut p1, mut p2) in
            self.find_conflicts_within_selection_set(ctx.parent_type(), selection_set, ctx)
        {
            p1.append(&mut p2);
            ctx.report_error(&error_message(&reason_name, &reason_msg), &p1);
        }
    }
}

fn error_message(reason_name: &str, reason: &ConflictReasonMessage) -> String {
    let suffix = "Use different aliases on the fields to fetch both if this was intentional";
    format!(
        r#"Fields "{reason_name}" conflict because {}. {suffix}"#,
        format_reason(reason),
    )
}

fn format_reason(reason: &ConflictReasonMessage) -> String {
    match reason {
        ConflictReasonMessage::Message(name) => name.clone(),
        ConflictReasonMessage::Nested(nested) => nested
            .iter()
            .map(|ConflictReason(name, subreason)| {
                format!(
                    r#"subfields "{name}" conflict because {}"#,
                    format_reason(subreason),
                )
            })
            .collect::<Vec<_>>()
            .join(" and "),
    }
}

#[cfg(test)]
mod tests {
    use super::{error_message, factory, ConflictReason, ConflictReasonMessage::*};

    use crate::{
        executor::Registry,
        schema::meta::MetaType,
        types::{
            base::{GraphQLType, GraphQLValue},
            scalars::{EmptyMutation, EmptySubscription, ID},
        },
    };

    use crate::{
        parser::SourcePosition,
        validation::{
            expect_fails_rule, expect_fails_rule_with_schema, expect_passes_rule,
            expect_passes_rule_with_schema, RuleError,
        },
        value::{DefaultScalarValue, ScalarValue},
    };

    #[test]
    fn unique_fields() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment uniqueFields on Dog {
            name
            nickname
          }
        "#,
        );
    }

    #[test]
    fn identical_fields() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment mergeIdenticalFields on Dog {
            name
            name
          }
        "#,
        );
    }

    #[test]
    fn identical_fields_with_identical_args() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment mergeIdenticalFieldsWithIdenticalArgs on Dog {
            doesKnowCommand(dogCommand: SIT)
            doesKnowCommand(dogCommand: SIT)
          }
        "#,
        );
    }

    #[test]
    fn identical_fields_with_identical_directives() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment mergeSameFieldsWithSameDirectives on Dog {
            name @include(if: true)
            name @include(if: true)
          }
        "#,
        );
    }

    #[test]
    fn different_args_with_different_aliases() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment differentArgsWithDifferentAliases on Dog {
            knowsSit: doesKnowCommand(dogCommand: SIT)
            knowsDown: doesKnowCommand(dogCommand: DOWN)
          }
        "#,
        );
    }

    #[test]
    fn different_directives_with_different_aliases() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment differentDirectivesWithDifferentAliases on Dog {
            nameIfTrue: name @include(if: true)
            nameIfFalse: name @include(if: false)
          }
        "#,
        );
    }

    #[test]
    fn different_skip_include_directives_accepted() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment differentDirectivesWithDifferentAliases on Dog {
            name @include(if: true)
            name @include(if: false)
          }
        "#,
        );
    }

    #[test]
    fn same_aliases_with_different_field_targets() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment sameAliasesWithDifferentFieldTargets on Dog {
            fido: name
            fido: nickname
          }
        "#,
            &[RuleError::new(
                &error_message(
                    "fido",
                    &Message("name and nickname are different fields".into()),
                ),
                &[
                    SourcePosition::new(78, 2, 12),
                    SourcePosition::new(101, 3, 12),
                ],
            )],
        );
    }

    #[test]
    fn same_aliases_allowed_on_nonoverlapping_fields() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment sameAliasesWithDifferentFieldTargets on Pet {
            ... on Dog {
              name
            }
            ... on Cat {
              name: nickname
            }
          }
        "#,
        );
    }

    #[test]
    fn alias_masking_direct_field_access() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment aliasMaskingDirectFieldAccess on Dog {
            name: nickname
            name
          }
        "#,
            &[RuleError::new(
                &error_message(
                    "name",
                    &Message("nickname and name are different fields".into()),
                ),
                &[
                    SourcePosition::new(71, 2, 12),
                    SourcePosition::new(98, 3, 12),
                ],
            )],
        );
    }

    #[test]
    fn different_args_second_adds_an_argument() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment conflictingArgs on Dog {
            doesKnowCommand
            doesKnowCommand(dogCommand: HEEL)
          }
        "#,
            &[RuleError::new(
                &error_message(
                    "doesKnowCommand",
                    &Message("they have differing arguments".into()),
                ),
                &[
                    SourcePosition::new(57, 2, 12),
                    SourcePosition::new(85, 3, 12),
                ],
            )],
        );
    }

    #[test]
    fn different_args_second_missing_an_argument() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment conflictingArgs on Dog {
            doesKnowCommand(dogCommand: SIT)
            doesKnowCommand
          }
        "#,
            &[RuleError::new(
                &error_message(
                    "doesKnowCommand",
                    &Message("they have differing arguments".into()),
                ),
                &[
                    SourcePosition::new(57, 2, 12),
                    SourcePosition::new(102, 3, 12),
                ],
            )],
        );
    }

    #[test]
    fn conflicting_args() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment conflictingArgs on Dog {
            doesKnowCommand(dogCommand: SIT)
            doesKnowCommand(dogCommand: HEEL)
          }
        "#,
            &[RuleError::new(
                &error_message(
                    "doesKnowCommand",
                    &Message("they have differing arguments".into()),
                ),
                &[
                    SourcePosition::new(57, 2, 12),
                    SourcePosition::new(102, 3, 12),
                ],
            )],
        );
    }

    #[test]
    fn allows_different_args_where_no_conflict_is_possible() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          fragment conflictingArgs on Pet {
            ... on Dog {
              name(surname: true)
            }
            ... on Cat {
              name
            }
          }
        "#,
        );
    }

    #[test]
    fn encounters_conflict_in_fragments() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          {
            ...A
            ...B
          }
          fragment A on Dog {
            x: name
          }
          fragment B on Dog {
            x: barks
          }
        "#,
            &[RuleError::new(
                &error_message("x", &Message("name and barks are different fields".into())),
                &[
                    SourcePosition::new(101, 6, 12),
                    SourcePosition::new(163, 9, 12),
                ],
            )],
        );
    }

    #[test]
    fn reports_each_conflict_once() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          {
            dorOrHuman {
              ...A
              ...B
            }
            catOrDog {
              ...B
              ...A
            }
            dog {
              ...A
              ...B
              x: name
            }
          }
          fragment A on Dog {
            x: barks
          }
          fragment B on Dog {
            x: nickname
          }
        "#,
            &[
                RuleError::new(
                    &error_message("x", &Message("name and barks are different fields".into())),
                    &[
                        SourcePosition::new(235, 13, 14),
                        SourcePosition::new(311, 17, 12),
                    ],
                ),
                RuleError::new(
                    &error_message(
                        "x",
                        &Message("name and nickname are different fields".into()),
                    ),
                    &[
                        SourcePosition::new(235, 13, 14),
                        SourcePosition::new(374, 20, 12),
                    ],
                ),
                RuleError::new(
                    &error_message(
                        "x",
                        &Message("barks and nickname are different fields".into()),
                    ),
                    &[
                        SourcePosition::new(311, 17, 12),
                        SourcePosition::new(374, 20, 12),
                    ],
                ),
            ],
        );
    }

    #[test]
    fn deep_conflict() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          {
            dog {
              x: name
            },
            dog {
              x: barks
            }
          }
        "#,
            &[RuleError::new(
                &error_message(
                    "dog",
                    &Nested(vec![ConflictReason(
                        "x".into(),
                        Message("name and barks are different fields".into()),
                    )]),
                ),
                &[
                    SourcePosition::new(25, 2, 12),
                    SourcePosition::new(45, 3, 14),
                    SourcePosition::new(80, 5, 12),
                    SourcePosition::new(100, 6, 14),
                ],
            )],
        );
    }

    #[test]
    fn deep_conflict_with_multiple_issues() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          {
            dog {
              x: barks
              y: name
            },
              dog {
              x: nickname
              y: barkVolume
            }
          }
        "#,
            &[RuleError::new(
                &error_message(
                    "dog",
                    &Nested(vec![
                        ConflictReason(
                            "x".into(),
                            Message("barks and nickname are different fields".into()),
                        ),
                        ConflictReason(
                            "y".into(),
                            Message("name and barkVolume are different fields".into()),
                        ),
                    ]),
                ),
                &[
                    SourcePosition::new(25, 2, 12),
                    SourcePosition::new(45, 3, 14),
                    SourcePosition::new(68, 4, 14),
                    SourcePosition::new(105, 6, 14),
                    SourcePosition::new(125, 7, 14),
                    SourcePosition::new(151, 8, 14),
                ],
            )],
        );
    }

    #[test]
    fn very_deep_conflict() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          {
            human {
              relatives {
                x: name
              }
            },
            human {
              relatives {
                x: iq
              }
            }
          }
        "#,
            &[RuleError::new(
                &error_message(
                    "human",
                    &Nested(vec![ConflictReason(
                        "relatives".into(),
                        Nested(vec![ConflictReason(
                            "x".into(),
                            Message("name and iq are different fields".into()),
                        )]),
                    )]),
                ),
                &[
                    SourcePosition::new(25, 2, 12),
                    SourcePosition::new(47, 3, 14),
                    SourcePosition::new(75, 4, 16),
                    SourcePosition::new(126, 7, 12),
                    SourcePosition::new(148, 8, 14),
                    SourcePosition::new(176, 9, 16),
                ],
            )],
        );
    }

    #[test]
    fn reports_deep_conflict_to_nearest_common_ancestor() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          {
            human {
              relatives {
                x: iq
              }
              relatives {
                x: name
              }
            },
            human {
              relatives {
                iq
              }
            }
          }
        "#,
            &[RuleError::new(
                &error_message(
                    "relatives",
                    &Nested(vec![ConflictReason(
                        "x".into(),
                        Message("iq and name are different fields".into()),
                    )]),
                ),
                &[
                    SourcePosition::new(47, 3, 14),
                    SourcePosition::new(75, 4, 16),
                    SourcePosition::new(111, 6, 14),
                    SourcePosition::new(139, 7, 16),
                ],
            )],
        );
    }

    #[test]
    fn reports_deep_conflict_to_nearest_common_ancestor_in_fragments() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          {
            human {
              ...F
            }
            human {
              ...F
            }
          }
          fragment F on Human {
            relatives {
              relatives {
                x: iq
              }
              relatives {
                x: name
              }
            },
            relatives {
              relatives {
                iq
              }
            }
          }
        "#,
            &[RuleError::new(
                &error_message(
                    "relatives",
                    &Nested(vec![ConflictReason(
                        "x".into(),
                        Message("iq and name are different fields".into()),
                    )]),
                ),
                &[
                    SourcePosition::new(201, 11, 14),
                    SourcePosition::new(229, 12, 16),
                    SourcePosition::new(265, 14, 14),
                    SourcePosition::new(293, 15, 16),
                ],
            )],
        );
    }

    #[test]
    fn reports_deep_conflict_in_nested_fragments() {
        expect_fails_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
          {
            dog {
              ...F
            }
            dog {
              ...I
            }
          }
          fragment F on Dog {
            x: name
            ...G
          }
          fragment G on Dog {
            y: barkVolume
          }
          fragment I on Dog {
            y: nickname
            ...J
          }
          fragment J on Dog {
            x: barks
          }
        "#,
            &[RuleError::new(
                &error_message(
                    "dog",
                    &Nested(vec![
                        ConflictReason(
                            "x".into(),
                            Message("name and barks are different fields".into()),
                        ),
                        ConflictReason(
                            "y".into(),
                            Message("barkVolume and nickname are different fields".into()),
                        ),
                    ]),
                ),
                &[
                    SourcePosition::new(25, 2, 12),
                    SourcePosition::new(169, 10, 12),
                    SourcePosition::new(248, 14, 12),
                    SourcePosition::new(76, 5, 12),
                    SourcePosition::new(399, 21, 12),
                    SourcePosition::new(316, 17, 12),
                ],
            )],
        );
    }

    #[test]
    fn ignores_unknown_fragments() {
        expect_passes_rule::<_, _, DefaultScalarValue>(
            factory,
            r#"
        {
          dog {
            name
          }
          ...Unknown
          ...Known
        }

        fragment Known on QueryRoot {
          dog {
            name
          }
          ...OtherUnknown
        }
        "#,
        );
    }

    struct SomeBox;
    struct StringBox;
    struct IntBox;
    struct NonNullStringBox1;
    struct NonNullStringBox1Impl;
    struct NonNullStringBox2;
    struct NonNullStringBox2Impl;
    struct Connection;
    struct Edge;
    struct Node;
    struct QueryRoot;

    impl<S> GraphQLType<S> for SomeBox
    where
        S: ScalarValue,
    {
        fn name(_: &()) -> Option<&'static str> {
            Some("SomeBox")
        }

        fn meta<'r>(i: &(), registry: &mut Registry<'r, S>) -> MetaType<'r, S>
        where
            S: 'r,
        {
            let fields = &[
                registry.field::<Option<SomeBox>>("deepBox", i),
                registry.field::<Option<String>>("unrelatedField", i),
                registry.field::<Option<String>>("otherField", i),
            ];

            registry.build_interface_type::<Self>(i, fields).into_meta()
        }
    }

    impl<S> GraphQLValue<S> for SomeBox
    where
        S: ScalarValue,
    {
        type Context = ();
        type TypeInfo = ();

        fn type_name<'i>(&self, info: &'i Self::TypeInfo) -> Option<&'i str> {
            <Self as GraphQLType>::name(info)
        }
    }

    impl<S> GraphQLType<S> for StringBox
    where
        S: ScalarValue,
    {
        fn name(_: &()) -> Option<&'static str> {
            Some("StringBox")
        }

        fn meta<'r>(i: &(), registry: &mut Registry<'r, S>) -> MetaType<'r, S>
        where
            S: 'r,
        {
            let fields = &[
                registry.field::<Option<String>>("scalar", i),
                registry.field::<Option<StringBox>>("deepBox", i),
                registry.field::<Option<String>>("unrelatedField", i),
                registry.field::<Option<Vec<Option<StringBox>>>>("listStringBox", i),
                registry.field::<Option<StringBox>>("stringBox", i),
                registry.field::<Option<IntBox>>("intBox", i),
            ];

            registry
                .build_object_type::<Self>(i, fields)
                .interfaces(&[registry.get_type::<SomeBox>(i)])
                .into_meta()
        }
    }

    impl<S> GraphQLValue<S> for StringBox
    where
        S: ScalarValue,
    {
        type Context = ();
        type TypeInfo = ();

        fn type_name<'i>(&self, info: &'i Self::TypeInfo) -> Option<&'i str> {
            <Self as GraphQLType>::name(info)
        }
    }

    impl<S> GraphQLType<S> for IntBox
    where
        S: ScalarValue,
    {
        fn name(_: &()) -> Option<&'static str> {
            Some("IntBox")
        }

        fn meta<'r>(i: &(), registry: &mut Registry<'r, S>) -> MetaType<'r, S>
        where
            S: 'r,
        {
            let fields = &[
                registry.field::<Option<i32>>("scalar", i),
                registry.field::<Option<IntBox>>("deepBox", i),
                registry.field::<Option<String>>("unrelatedField", i),
                registry.field::<Option<Vec<Option<StringBox>>>>("listStringBox", i),
                registry.field::<Option<StringBox>>("stringBox", i),
                registry.field::<Option<IntBox>>("intBox", i),
            ];

            registry
                .build_object_type::<Self>(i, fields)
                .interfaces(&[registry.get_type::<SomeBox>(i)])
                .into_meta()
        }
    }

    impl<S> GraphQLValue<S> for IntBox
    where
        S: ScalarValue,
    {
        type Context = ();
        type TypeInfo = ();

        fn type_name<'i>(&self, info: &'i Self::TypeInfo) -> Option<&'i str> {
            <Self as GraphQLType>::name(info)
        }
    }

    impl<S> GraphQLType<S> for NonNullStringBox1
    where
        S: ScalarValue,
    {
        fn name(_: &()) -> Option<&'static str> {
            Some("NonNullStringBox1")
        }

        fn meta<'r>(i: &(), registry: &mut Registry<'r, S>) -> MetaType<'r, S>
        where
            S: 'r,
        {
            let fields = &[registry.field::<String>("scalar", i)];

            registry.build_interface_type::<Self>(i, fields).into_meta()
        }
    }

    impl<S> GraphQLValue<S> for NonNullStringBox1
    where
        S: ScalarValue,
    {
        type Context = ();
        type TypeInfo = ();

        fn type_name<'i>(&self, info: &'i Self::TypeInfo) -> Option<&'i str> {
            <Self as GraphQLType>::name(info)
        }
    }

    impl<S> GraphQLType<S> for NonNullStringBox1Impl
    where
        S: ScalarValue,
    {
        fn name(_: &()) -> Option<&'static str> {
            Some("NonNullStringBox1Impl")
        }

        fn meta<'r>(i: &(), registry: &mut Registry<'r, S>) -> MetaType<'r, S>
        where
            S: 'r,
        {
            let fields = &[
                registry.field::<String>("scalar", i),
                registry.field::<Option<SomeBox>>("deepBox", i),
                registry.field::<Option<String>>("unrelatedField", i),
            ];

            registry
                .build_object_type::<Self>(i, fields)
                .interfaces(&[
                    registry.get_type::<NonNullStringBox1>(i),
                    registry.get_type::<SomeBox>(i),
                ])
                .into_meta()
        }
    }

    impl<S> GraphQLValue<S> for NonNullStringBox1Impl
    where
        S: ScalarValue,
    {
        type Context = ();
        type TypeInfo = ();

        fn type_name<'i>(&self, info: &'i Self::TypeInfo) -> Option<&'i str> {
            <Self as GraphQLType>::name(info)
        }
    }

    impl<S> GraphQLType<S> for NonNullStringBox2
    where
        S: ScalarValue,
    {
        fn name(_: &()) -> Option<&'static str> {
            Some("NonNullStringBox2")
        }

        fn meta<'r>(i: &(), registry: &mut Registry<'r, S>) -> MetaType<'r, S>
        where
            S: 'r,
        {
            let fields = &[registry.field::<String>("scalar", i)];

            registry.build_interface_type::<Self>(i, fields).into_meta()
        }
    }

    impl<S> GraphQLValue<S> for NonNullStringBox2
    where
        S: ScalarValue,
    {
        type Context = ();
        type TypeInfo = ();

        fn type_name<'i>(&self, info: &'i Self::TypeInfo) -> Option<&'i str> {
            <Self as GraphQLType>::name(info)
        }
    }

    impl<S> GraphQLType<S> for NonNullStringBox2Impl
    where
        S: ScalarValue,
    {
        fn name(_: &()) -> Option<&'static str> {
            Some("NonNullStringBox2Impl")
        }

        fn meta<'r>(i: &(), registry: &mut Registry<'r, S>) -> MetaType<'r, S>
        where
            S: 'r,
        {
            let fields = &[
                registry.field::<String>("scalar", i),
                registry.field::<Option<SomeBox>>("deepBox", i),
                registry.field::<Option<String>>("unrelatedField", i),
            ];

            registry
                .build_object_type::<Self>(i, fields)
                .interfaces(&[
                    registry.get_type::<NonNullStringBox2>(i),
                    registry.get_type::<SomeBox>(i),
                ])
                .into_meta()
        }
    }

    impl<S> GraphQLValue<S> for NonNullStringBox2Impl
    where
        S: ScalarValue,
    {
        type Context = ();
        type TypeInfo = ();

        fn type_name<'i>(&self, info: &'i Self::TypeInfo) -> Option<&'i str> {
            <Self as GraphQLType>::name(info)
        }
    }

    impl<S> GraphQLType<S> for Node
    where
        S: ScalarValue,
    {
        fn name(_: &()) -> Option<&'static str> {
            Some("Node")
        }

        fn meta<'r>(i: &(), registry: &mut Registry<'r, S>) -> MetaType<'r, S>
        where
            S: 'r,
        {
            let fields = &[
                registry.field::<Option<ID>>("id", i),
                registry.field::<Option<String>>("name", i),
            ];

            registry.build_object_type::<Self>(i, fields).into_meta()
        }
    }

    impl<S> GraphQLValue<S> for Node
    where
        S: ScalarValue,
    {
        type Context = ();
        type TypeInfo = ();

        fn type_name<'i>(&self, info: &'i Self::TypeInfo) -> Option<&'i str> {
            <Self as GraphQLType>::name(info)
        }
    }

    impl<S> GraphQLType<S> for Edge
    where
        S: ScalarValue,
    {
        fn name(_: &()) -> Option<&'static str> {
            Some("Edge")
        }

        fn meta<'r>(i: &(), registry: &mut Registry<'r, S>) -> MetaType<'r, S>
        where
            S: 'r,
        {
            let fields = &[registry.field::<Option<Node>>("node", i)];

            registry.build_object_type::<Self>(i, fields).into_meta()
        }
    }

    impl<S> GraphQLValue<S> for Edge
    where
        S: ScalarValue,
    {
        type Context = ();
        type TypeInfo = ();

        fn type_name<'i>(&self, info: &'i Self::TypeInfo) -> Option<&'i str> {
            <Self as GraphQLType>::name(info)
        }
    }

    impl<S> GraphQLType<S> for Connection
    where
        S: ScalarValue,
    {
        fn name(_: &()) -> Option<&'static str> {
            Some("Connection")
        }

        fn meta<'r>(i: &(), registry: &mut Registry<'r, S>) -> MetaType<'r, S>
        where
            S: 'r,
        {
            let fields = &[registry.field::<Option<Vec<Option<Edge>>>>("edges", i)];

            registry.build_object_type::<Self>(i, fields).into_meta()
        }
    }

    impl<S> GraphQLValue<S> for Connection
    where
        S: ScalarValue,
    {
        type Context = ();
        type TypeInfo = ();

        fn type_name<'i>(&self, info: &'i Self::TypeInfo) -> Option<&'i str> {
            <Self as GraphQLType>::name(info)
        }
    }

    impl<S> GraphQLType<S> for QueryRoot
    where
        S: ScalarValue,
    {
        fn name(_: &()) -> Option<&'static str> {
            Some("QueryRoot")
        }

        fn meta<'r>(i: &(), registry: &mut Registry<'r, S>) -> MetaType<'r, S>
        where
            S: 'r,
        {
            registry.get_type::<IntBox>(i);
            registry.get_type::<StringBox>(i);
            registry.get_type::<NonNullStringBox1Impl>(i);
            registry.get_type::<NonNullStringBox2Impl>(i);

            let fields = &[
                registry.field::<Option<SomeBox>>("someBox", i),
                registry.field::<Option<Connection>>("connection", i),
            ];
            registry.build_object_type::<Self>(i, fields).into_meta()
        }
    }

    impl<S> GraphQLValue<S> for QueryRoot
    where
        S: ScalarValue,
    {
        type Context = ();
        type TypeInfo = ();

        fn type_name<'i>(&self, info: &'i Self::TypeInfo) -> Option<&'i str> {
            <Self as GraphQLType>::name(info)
        }
    }

    #[test]
    fn conflicting_return_types_which_potentially_overlap() {
        expect_fails_rule_with_schema::<_, EmptyMutation<()>, _, _, DefaultScalarValue>(
            QueryRoot,
            EmptyMutation::new(),
            factory,
            r#"
            {
              someBox {
                ...on IntBox {
                  scalar
                }
                ...on NonNullStringBox1 {
                  scalar
                }
              }
            }
        "#,
            &[RuleError::new(
                &error_message(
                    "scalar",
                    &Message("they return conflicting types Int and String!".into()),
                ),
                &[
                    SourcePosition::new(88, 4, 18),
                    SourcePosition::new(173, 7, 18),
                ],
            )],
        );
    }

    #[test]
    fn compatible_return_shapes_on_different_return_types() {
        expect_passes_rule_with_schema::<
            _,
            EmptyMutation<()>,
            EmptySubscription<()>,
            _,
            _,
            DefaultScalarValue,
        >(
            QueryRoot,
            EmptyMutation::new(),
            EmptySubscription::new(),
            factory,
            r#"
          {
            someBox {
              ... on SomeBox {
                deepBox {
                  unrelatedField
                }
              }
              ... on StringBox {
                deepBox {
                  unrelatedField
                }
              }
            }
          }
        "#,
        );
    }

    #[test]
    fn disallows_differing_return_types_despite_no_overlap() {
        expect_fails_rule_with_schema::<_, EmptyMutation<()>, _, _, DefaultScalarValue>(
            QueryRoot,
            EmptyMutation::new(),
            factory,
            r#"
            {
              someBox {
                ... on IntBox {
                  scalar
                }
                ... on StringBox {
                  scalar
                }
              }
            }
        "#,
            &[RuleError::new(
                &error_message(
                    "scalar",
                    &Message("they return conflicting types Int and String".into()),
                ),
                &[
                    SourcePosition::new(89, 4, 18),
                    SourcePosition::new(167, 7, 18),
                ],
            )],
        );
    }

    #[test]
    fn reports_correctly_when_a_non_exclusive_follows_an_exclusive() {
        expect_fails_rule_with_schema::<_, EmptyMutation<()>, _, _, DefaultScalarValue>(
            QueryRoot,
            EmptyMutation::new(),
            factory,
            r#"
            {
              someBox {
                ... on IntBox {
                  deepBox {
                    ...X
                  }
                }
              }
              someBox {
                ... on StringBox {
                  deepBox {
                    ...Y
                  }
                }
              }
              memoed: someBox {
                ... on IntBox {
                  deepBox {
                    ...X
                  }
                }
              }
              memoed: someBox {
                ... on StringBox {
                  deepBox {
                    ...Y
                  }
                }
              }
              other: someBox {
                ...X
              }
              other: someBox {
                ...Y
              }
            }
            fragment X on SomeBox {
              otherField
            }
            fragment Y on SomeBox {
              otherField: unrelatedField
            }
        "#,
            &[RuleError::new(
                &error_message(
                    "other",
                    &Nested(vec![ConflictReason(
                        "otherField".into(),
                        Message("otherField and unrelatedField are different fields".into()),
                    )]),
                ),
                &[
                    SourcePosition::new(703, 30, 14),
                    SourcePosition::new(889, 38, 14),
                    SourcePosition::new(771, 33, 14),
                    SourcePosition::new(964, 41, 14),
                ],
            )],
        );
    }

    #[test]
    fn disallows_differing_return_type_nullability_despite_no_overlap() {
        expect_fails_rule_with_schema::<_, EmptyMutation<()>, _, _, DefaultScalarValue>(
            QueryRoot,
            EmptyMutation::new(),
            factory,
            r#"
            {
              someBox {
                ... on NonNullStringBox1 {
                  scalar
                }
                ... on StringBox {
                  scalar
                }
              }
            }
        "#,
            &[RuleError::new(
                &error_message(
                    "scalar",
                    &Message("they return conflicting types String! and String".into()),
                ),
                &[
                    SourcePosition::new(100, 4, 18),
                    SourcePosition::new(178, 7, 18),
                ],
            )],
        );
    }

    #[test]
    fn disallows_differing_return_type_list_despite_no_overlap() {
        expect_fails_rule_with_schema::<_, EmptyMutation<()>, _, _, DefaultScalarValue>(
            QueryRoot,
            EmptyMutation::new(),
            factory,
            r#"
            {
              someBox {
                ... on IntBox {
                  box: listStringBox {
                    scalar
                  }
                }
                ... on StringBox {
                  box: stringBox {
                    scalar
                  }
                }
              }
            }
        "#,
            &[RuleError::new(
                &error_message(
                    "box",
                    &Message("they return conflicting types [StringBox] and StringBox".into()),
                ),
                &[
                    SourcePosition::new(89, 4, 18),
                    SourcePosition::new(228, 9, 18),
                ],
            )],
        );

        expect_fails_rule_with_schema::<_, EmptyMutation<()>, _, _, DefaultScalarValue>(
            QueryRoot,
            EmptyMutation::new(),
            factory,
            r#"
            {
              someBox {
                ... on IntBox {
                  box: stringBox {
                    scalar
                  }
                }
                ... on StringBox {
                  box: listStringBox {
                    scalar
                  }
                }
              }
            }
        "#,
            &[RuleError::new(
                &error_message(
                    "box",
                    &Message("they return conflicting types StringBox and [StringBox]".into()),
                ),
                &[
                    SourcePosition::new(89, 4, 18),
                    SourcePosition::new(224, 9, 18),
                ],
            )],
        );
    }

    #[test]
    fn disallows_differing_subfields() {
        expect_fails_rule_with_schema::<_, EmptyMutation<()>, _, _, DefaultScalarValue>(
            QueryRoot,
            EmptyMutation::new(),
            factory,
            r#"
            {
              someBox {
                ... on IntBox {
                  box: stringBox {
                    val: scalar
                    val: unrelatedField
                  }
                }
                ... on StringBox {
                  box: stringBox {
                    val: scalar
                  }
                }
              }
            }
        "#,
            &[RuleError::new(
                &error_message(
                    "val",
                    &Message("scalar and unrelatedField are different fields".into()),
                ),
                &[
                    SourcePosition::new(126, 5, 20),
                    SourcePosition::new(158, 6, 20),
                ],
            )],
        );
    }

    #[test]
    fn disallows_differing_deep_return_types_despite_no_overlap() {
        expect_fails_rule_with_schema::<_, EmptyMutation<()>, _, _, DefaultScalarValue>(
            QueryRoot,
            EmptyMutation::new(),
            factory,
            r#"
            {
              someBox {
                ... on IntBox {
                  box: stringBox {
                    scalar
                  }
                }
                ... on StringBox {
                  box: intBox {
                    scalar
                  }
                }
              }
            }
        "#,
            &[RuleError::new(
                &error_message(
                    "box",
                    &Nested(vec![ConflictReason(
                        "scalar".into(),
                        Message("they return conflicting types String and Int".into()),
                    )]),
                ),
                &[
                    SourcePosition::new(89, 4, 18),
                    SourcePosition::new(126, 5, 20),
                    SourcePosition::new(224, 9, 18),
                    SourcePosition::new(258, 10, 20),
                ],
            )],
        );
    }

    #[test]
    fn allows_non_conflicting_overlapping_types() {
        expect_passes_rule_with_schema::<
            _,
            EmptyMutation<()>,
            EmptySubscription<()>,
            _,
            _,
            DefaultScalarValue,
        >(
            QueryRoot,
            EmptyMutation::new(),
            EmptySubscription::new(),
            factory,
            r#"
            {
              someBox {
                ... on IntBox {
                  scalar: unrelatedField
                }
                ... on StringBox {
                  scalar
                }
              }
            }
        "#,
        );
    }

    #[test]
    fn same_wrapped_scalar_return_types() {
        expect_passes_rule_with_schema::<
            _,
            EmptyMutation<()>,
            EmptySubscription<()>,
            _,
            _,
            DefaultScalarValue,
        >(
            QueryRoot,
            EmptyMutation::new(),
            EmptySubscription::new(),
            factory,
            r#"
            {
              someBox {
                ...on NonNullStringBox1 {
                  scalar
                }
                ...on NonNullStringBox2 {
                  scalar
                }
              }
            }
        "#,
        );
    }

    #[test]
    fn allows_inline_typeless_fragments() {
        expect_passes_rule_with_schema::<
            _,
            EmptyMutation<()>,
            EmptySubscription<()>,
            _,
            _,
            DefaultScalarValue,
        >(
            QueryRoot,
            EmptyMutation::new(),
            EmptySubscription::new(),
            factory,
            r#"
            {
              someBox {
                unrelatedField
              }
              ... {
                someBox {
                  unrelatedField
                }
              }
            }
        "#,
        );
    }

    #[test]
    fn compares_deep_types_including_list() {
        expect_fails_rule_with_schema::<_, EmptyMutation<()>, _, _, DefaultScalarValue>(
            QueryRoot,
            EmptyMutation::new(),
            factory,
            r#"
            {
              connection {
                ...edgeID
                edges {
                  node {
                    id: name
                  }
                }
              }
            }

            fragment edgeID on Connection {
              edges {
                node {
                  id
                }
              }
            }
        "#,
            &[RuleError::new(
                &error_message(
                    "edges",
                    &Nested(vec![ConflictReason(
                        "node".into(),
                        Nested(vec![ConflictReason(
                            "id".into(),
                            Message("name and id are different fields".into()),
                        )]),
                    )]),
                ),
                &[
                    SourcePosition::new(84, 4, 16),
                    SourcePosition::new(110, 5, 18),
                    SourcePosition::new(137, 6, 20),
                    SourcePosition::new(273, 13, 14),
                    SourcePosition::new(297, 14, 16),
                    SourcePosition::new(322, 15, 18),
                ],
            )],
        );
    }

    #[test]
    fn ignores_unknown_types() {
        expect_passes_rule_with_schema::<
            _,
            EmptyMutation<()>,
            EmptySubscription<()>,
            _,
            _,
            DefaultScalarValue,
        >(
            QueryRoot,
            EmptyMutation::new(),
            EmptySubscription::new(),
            factory,
            r#"
            {
              someBox {
                ...on UnknownType {
                  scalar
                }
                ...on NonNullStringBox2 {
                  scalar
                }
              }
            }
        "#,
        );
    }

    #[test]
    fn error_message_contains_hint_for_alias_conflict() {
        assert_eq!(
            &error_message("x", &Message("a and b are different fields".into())),
            "Fields \"x\" conflict because a and b are different fields. Use \
             different aliases on the fields to fetch both if this \
             was intentional"
        );
    }
}
