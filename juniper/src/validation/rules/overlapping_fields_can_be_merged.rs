use ast::{Arguments, Definition, Document, Field, Fragment, FragmentSpread, Selection, Type};
use parser::{SourcePosition, Spanning};
use schema::meta::{Field as FieldType, MetaType};
use std::borrow::Borrow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::Hash;
use validation::{ValidatorContext, Visitor};

#[derive(Debug)]
struct Conflict(ConflictReason, Vec<SourcePosition>, Vec<SourcePosition>);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ConflictReason(String, ConflictReasonMessage);

#[derive(Debug)]
struct AstAndDef<'a>(
    Option<&'a str>,
    &'a Spanning<Field<'a>>,
    Option<&'a FieldType<'a>>,
);

type AstAndDefCollection<'a> = OrderedMap<&'a str, Vec<AstAndDef<'a>>>;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum ConflictReasonMessage {
    Message(String),
    Nested(Vec<ConflictReason>),
}

struct PairSet<'a> {
    data: HashMap<&'a str, HashMap<&'a str, bool>>,
}

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
        self.data
            .entry(a)
            .or_insert_with(HashMap::new)
            .insert(b, mutex);

        self.data
            .entry(b)
            .or_insert_with(HashMap::new)
            .insert(a, mutex);
    }
}

pub struct OverlappingFieldsCanBeMerged<'a> {
    named_fragments: HashMap<&'a str, &'a Fragment<'a>>,
    compared_fragments: RefCell<PairSet<'a>>,
}

pub fn factory<'a>() -> OverlappingFieldsCanBeMerged<'a> {
    OverlappingFieldsCanBeMerged {
        named_fragments: HashMap::new(),
        compared_fragments: RefCell::new(PairSet::new()),
    }
}

impl<'a> OverlappingFieldsCanBeMerged<'a> {
    fn find_conflicts_within_selection_set(
        &self,
        parent_type: Option<&'a MetaType>,
        selection_set: &'a [Selection],
        ctx: &ValidatorContext<'a>,
    ) -> Vec<Conflict> {
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
        ctx: &ValidatorContext<'a>,
    ) {
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
        field_map: &AstAndDefCollection<'a>,
        fragment_name: &str,
        mutually_exclusive: bool,
        ctx: &ValidatorContext<'a>,
    ) {
        let fragment = match self.named_fragments.get(fragment_name) {
            Some(f) => f,
            None => return,
        };

        let (field_map2, fragment_names2) =
            self.get_referenced_fields_and_fragment_names(fragment, ctx);

        self.collect_conflicts_between(conflicts, mutually_exclusive, field_map, &field_map2, ctx);

        for fragment_name2 in fragment_names2 {
            self.collect_conflicts_between_fields_and_fragment(
                conflicts,
                field_map,
                fragment_name2,
                mutually_exclusive,
                ctx,
            );
        }
    }

    fn collect_conflicts_between(
        &self,
        conflicts: &mut Vec<Conflict>,
        mutually_exclusive: bool,
        field_map1: &AstAndDefCollection<'a>,
        field_map2: &AstAndDefCollection<'a>,
        ctx: &ValidatorContext<'a>,
    ) {
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
        field_map: &AstAndDefCollection<'a>,
        ctx: &ValidatorContext<'a>,
    ) {
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
        field1: &AstAndDef<'a>,
        field2: &AstAndDef<'a>,
        parents_mutually_exclusive: bool,
        ctx: &ValidatorContext<'a>,
    ) -> Option<Conflict> {
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
                        response_name.to_owned(),
                        ConflictReasonMessage::Message(format!(
                            "{} and {} are different fields",
                            name1, name2
                        )),
                    ),
                    vec![ast1.start.clone()],
                    vec![ast2.start.clone()],
                ));
            }

            if !self.is_same_arguments(&ast1.item.arguments, &ast2.item.arguments) {
                return Some(Conflict(
                    ConflictReason(
                        response_name.to_owned(),
                        ConflictReasonMessage::Message("they have differing arguments".to_owned()),
                    ),
                    vec![ast1.start.clone()],
                    vec![ast2.start.clone()],
                ));
            }
        }

        let t1 = def1.as_ref().map(|def| &def.field_type);
        let t2 = def2.as_ref().map(|def| &def.field_type);

        if let (Some(t1), Some(t2)) = (t1, t2) {
            if self.is_type_conflict(ctx, t1, t2) {
                return Some(Conflict(
                    ConflictReason(
                        response_name.to_owned(),
                        ConflictReasonMessage::Message(format!(
                            "they return conflicting types {} and {}",
                            t1, t2
                        )),
                    ),
                    vec![ast1.start.clone()],
                    vec![ast2.start.clone()],
                ));
            }
        }

        if let (&Some(ref s1), &Some(ref s2)) = (&ast1.item.selection_set, &ast2.item.selection_set)
        {
            let conflicts = self.find_conflicts_between_sub_selection_sets(
                mutually_exclusive,
                t1.map(|t| t.innermost_name()),
                s1,
                t2.map(|t| t.innermost_name()),
                s2,
                ctx,
            );

            return self.subfield_conflicts(&conflicts, response_name, &ast1.start, &ast2.start);
        }

        None
    }

    fn find_conflicts_between_sub_selection_sets(
        &self,
        mutually_exclusive: bool,
        parent_type1: Option<&str>,
        selection_set1: &'a [Selection],
        parent_type2: Option<&str>,
        selection_set2: &'a [Selection],
        ctx: &ValidatorContext<'a>,
    ) -> Vec<Conflict> {
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
                response_name.to_owned(),
                ConflictReasonMessage::Nested(conflicts.iter().map(|c| c.0.clone()).collect()),
            ),
            vec![pos1.clone()]
                .into_iter()
                .chain(
                    conflicts
                        .iter()
                        .flat_map(|&Conflict(_, ref fs1, _)| fs1.clone()),
                )
                .collect(),
            vec![pos2.clone()]
                .into_iter()
                .chain(
                    conflicts
                        .iter()
                        .flat_map(|&Conflict(_, _, ref fs2)| fs2.clone()),
                )
                .collect(),
        ))
    }

    fn is_type_conflict(&self, ctx: &ValidatorContext<'a>, t1: &Type, t2: &Type) -> bool {
        match (t1, t2) {
            (&Type::List(ref inner1), &Type::List(ref inner2))
            | (&Type::NonNullList(ref inner1), &Type::NonNullList(ref inner2)) => {
                self.is_type_conflict(ctx, inner1, inner2)
            }
            (&Type::NonNullNamed(ref n1), &Type::NonNullNamed(ref n2))
            | (&Type::Named(ref n1), &Type::Named(ref n2)) => {
                let ct1 = ctx.schema.concrete_type_by_name(n1);
                let ct2 = ctx.schema.concrete_type_by_name(n2);

                if ct1.map(|ct| ct.is_leaf()).unwrap_or(false)
                    || ct2.map(|ct| ct.is_leaf()).unwrap_or(false)
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
        args1: &Option<Spanning<Arguments>>,
        args2: &Option<Spanning<Arguments>>,
    ) -> bool {
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

                args1.iter().all(|&(ref n1, ref v1)| {
                    if let Some(&(_, ref v2)) =
                        args2.iter().find(|&&(ref n2, _)| n1.item == n2.item)
                    {
                        v1.item.unlocated_eq(&v2.item)
                    } else {
                        false
                    }
                })
            }
            _ => false,
        }
    }

    fn is_object_type(&self, ctx: &ValidatorContext<'a>, type_name: Option<&str>) -> bool {
        match type_name.and_then(|n| ctx.schema.concrete_type_by_name(n)) {
            Some(&MetaType::Object(_)) => true,
            _ => false,
        }
    }

    fn get_referenced_fields_and_fragment_names(
        &self,
        fragment: &'a Fragment,
        ctx: &ValidatorContext<'a>,
    ) -> (AstAndDefCollection<'a>, Vec<&'a str>) {
        let fragment_type = ctx.schema
            .concrete_type_by_name(fragment.type_condition.item);

        self.get_fields_and_fragment_names(fragment_type, &fragment.selection_set, ctx)
    }

    fn get_fields_and_fragment_names(
        &self,
        parent_type: Option<&'a MetaType>,
        selection_set: &'a [Selection],
        ctx: &ValidatorContext<'a>,
    ) -> (AstAndDefCollection<'a>, Vec<&'a str>) {
        let mut ast_and_defs = OrderedMap::new();
        let mut fragment_names = Vec::new();

        self.collect_fields_and_fragment_names(
            parent_type,
            selection_set,
            ctx,
            &mut ast_and_defs,
            &mut fragment_names,
        );

        (ast_and_defs, fragment_names)
    }

    fn collect_fields_and_fragment_names(
        &self,
        parent_type: Option<&'a MetaType>,
        selection_set: &'a [Selection],
        ctx: &ValidatorContext<'a>,
        ast_and_defs: &mut AstAndDefCollection<'a>,
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
                }) => if fragment_names.iter().find(|n| *n == &name.item).is_none() {
                    fragment_names.push(name.item);
                },
                Selection::InlineFragment(Spanning {
                    item: ref inline, ..
                }) => {
                    let parent_type = inline
                        .type_condition
                        .as_ref()
                        .and_then(|cond| ctx.schema.concrete_type_by_name(cond.item))
                        .or(parent_type);

                    self.collect_fields_and_fragment_names(
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

impl<'a> Visitor<'a> for OverlappingFieldsCanBeMerged<'a> {
    fn enter_document(&mut self, _: &mut ValidatorContext<'a>, defs: &'a Document) {
        for def in defs {
            if let Definition::Fragment(Spanning { ref item, .. }) = *def {
                self.named_fragments.insert(item.name.item, item);
            }
        }
    }

    fn enter_selection_set(
        &mut self,
        ctx: &mut ValidatorContext<'a>,
        selection_set: &'a Vec<Selection>,
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
        r#"Fields "{}" conflict because {}. {}"#,
        reason_name,
        format_reason(reason),
        suffix
    )
}

fn format_reason(reason: &ConflictReasonMessage) -> String {
    match *reason {
        ConflictReasonMessage::Message(ref name) => name.clone(),
        ConflictReasonMessage::Nested(ref nested) => nested
            .iter()
            .map(|&ConflictReason(ref name, ref subreason)| {
                format!(
                    r#"subfields "{}" conflict because {}"#,
                    name,
                    format_reason(subreason)
                )
            })
            .collect::<Vec<_>>()
            .join(" and "),
    }
}

#[cfg(test)]
mod tests {
    use super::ConflictReasonMessage::*;
    use super::{error_message, factory, ConflictReason};

    use executor::Registry;
    use schema::meta::MetaType;
    use types::base::GraphQLType;
    use types::scalars::ID;

    use parser::SourcePosition;
    use validation::{
        expect_fails_rule, expect_fails_rule_with_schema, expect_passes_rule,
        expect_passes_rule_with_schema, RuleError,
    };

    #[test]
    fn unique_fields() {
        expect_passes_rule(
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
        expect_passes_rule(
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
        expect_passes_rule(
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
        expect_passes_rule(
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
        expect_passes_rule(
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
        expect_passes_rule(
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
        expect_passes_rule(
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
        expect_fails_rule(
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
                    &Message("name and nickname are different fields".to_owned()),
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
        expect_passes_rule(
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
        expect_fails_rule(
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
                    &Message("nickname and name are different fields".to_owned()),
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
        expect_fails_rule(
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
                    &Message("they have differing arguments".to_owned()),
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
        expect_fails_rule(
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
                    &Message("they have differing arguments".to_owned()),
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
        expect_fails_rule(
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
                    &Message("they have differing arguments".to_owned()),
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
        expect_passes_rule(
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
        expect_fails_rule(
            factory,
            r#"
          {
            ...A
            ...B
          }
          fragment A on Type {
            x: a
          }
          fragment B on Type {
            x: b
          }
        "#,
            &[RuleError::new(
                &error_message("x", &Message("a and b are different fields".to_owned())),
                &[
                    SourcePosition::new(102, 6, 12),
                    SourcePosition::new(162, 9, 12),
                ],
            )],
        );
    }

    #[test]
    fn reports_each_conflict_once() {
        expect_fails_rule(
            factory,
            r#"
          {
            f1 {
              ...A
              ...B
            }
            f2 {
              ...B
              ...A
            }
            f3 {
              ...A
              ...B
              x: c
            }
          }
          fragment A on Type {
            x: a
          }
          fragment B on Type {
            x: b
          }
        "#,
            &[
                RuleError::new(
                    &error_message("x", &Message("c and a are different fields".to_owned())),
                    &[
                        SourcePosition::new(220, 13, 14),
                        SourcePosition::new(294, 17, 12),
                    ],
                ),
                RuleError::new(
                    &error_message("x", &Message("c and b are different fields".to_owned())),
                    &[
                        SourcePosition::new(220, 13, 14),
                        SourcePosition::new(354, 20, 12),
                    ],
                ),
                RuleError::new(
                    &error_message("x", &Message("a and b are different fields".to_owned())),
                    &[
                        SourcePosition::new(294, 17, 12),
                        SourcePosition::new(354, 20, 12),
                    ],
                ),
            ],
        );
    }

    #[test]
    fn deep_conflict() {
        expect_fails_rule(
            factory,
            r#"
          {
            field {
              x: a
            },
            field {
              x: b
            }
          }
        "#,
            &[RuleError::new(
                &error_message(
                    "field",
                    &Nested(vec![ConflictReason(
                        "x".to_owned(),
                        Message("a and b are different fields".to_owned()),
                    )]),
                ),
                &[
                    SourcePosition::new(25, 2, 12),
                    SourcePosition::new(47, 3, 14),
                    SourcePosition::new(79, 5, 12),
                    SourcePosition::new(101, 6, 14),
                ],
            )],
        );
    }

    #[test]
    fn deep_conflict_with_multiple_issues() {
        expect_fails_rule(
            factory,
            r#"
          {
            field {
              x: a
              y: c
            },
            field {
              x: b
              y: d
            }
          }
        "#,
            &[RuleError::new(
                &error_message(
                    "field",
                    &Nested(vec![
                        ConflictReason(
                            "x".to_owned(),
                            Message("a and b are different fields".to_owned()),
                        ),
                        ConflictReason(
                            "y".to_owned(),
                            Message("c and d are different fields".to_owned()),
                        ),
                    ]),
                ),
                &[
                    SourcePosition::new(25, 2, 12),
                    SourcePosition::new(47, 3, 14),
                    SourcePosition::new(66, 4, 14),
                    SourcePosition::new(98, 6, 12),
                    SourcePosition::new(120, 7, 14),
                    SourcePosition::new(139, 8, 14),
                ],
            )],
        );
    }

    #[test]
    fn very_deep_conflict() {
        expect_fails_rule(
            factory,
            r#"
          {
            field {
              deepField {
                x: a
              }
            },
            field {
              deepField {
                x: b
              }
            }
          }
        "#,
            &[RuleError::new(
                &error_message(
                    "field",
                    &Nested(vec![ConflictReason(
                        "deepField".to_owned(),
                        Nested(vec![ConflictReason(
                            "x".to_owned(),
                            Message("a and b are different fields".to_owned()),
                        )]),
                    )]),
                ),
                &[
                    SourcePosition::new(25, 2, 12),
                    SourcePosition::new(47, 3, 14),
                    SourcePosition::new(75, 4, 16),
                    SourcePosition::new(123, 7, 12),
                    SourcePosition::new(145, 8, 14),
                    SourcePosition::new(173, 9, 16),
                ],
            )],
        );
    }

    #[test]
    fn reports_deep_conflict_to_nearest_common_ancestor() {
        expect_fails_rule(
            factory,
            r#"
          {
            field {
              deepField {
                x: a
              }
              deepField {
                x: b
              }
            },
            field {
              deepField {
                y
              }
            }
          }
        "#,
            &[RuleError::new(
                &error_message(
                    "deepField",
                    &Nested(vec![ConflictReason(
                        "x".to_owned(),
                        Message("a and b are different fields".to_owned()),
                    )]),
                ),
                &[
                    SourcePosition::new(47, 3, 14),
                    SourcePosition::new(75, 4, 16),
                    SourcePosition::new(110, 6, 14),
                    SourcePosition::new(138, 7, 16),
                ],
            )],
        );
    }

    #[test]
    fn reports_deep_conflict_to_nearest_common_ancestor_in_fragments() {
        expect_fails_rule(
            factory,
            r#"
          {
            field {
              ...F
            }
            field {
              ...F
            }
          }
          fragment F on T {
            deepField {
              deeperField {
                x: a
              }
              deeperField {
                x: b
              }
            },
            deepField {
              deeperField {
                y
              }
            }
          }
        "#,
            &[RuleError::new(
                &error_message(
                    "deeperField",
                    &Nested(vec![ConflictReason(
                        "x".to_owned(),
                        Message("a and b are different fields".to_owned()),
                    )]),
                ),
                &[
                    SourcePosition::new(197, 11, 14),
                    SourcePosition::new(227, 12, 16),
                    SourcePosition::new(262, 14, 14),
                    SourcePosition::new(292, 15, 16),
                ],
            )],
        );
    }

    #[test]
    fn reports_deep_conflict_in_nested_fragments() {
        expect_fails_rule(
            factory,
            r#"
          {
            field {
              ...F
            }
            field {
              ...I
            }
          }
          fragment F on T {
            x: a
            ...G
          }
          fragment G on T {
            y: c
          }
          fragment I on T {
            y: d
            ...J
          }
          fragment J on T {
            x: b
          }
        "#,
            &[RuleError::new(
                &error_message(
                    "field",
                    &Nested(vec![
                        ConflictReason(
                            "x".to_owned(),
                            Message("a and b are different fields".to_owned()),
                        ),
                        ConflictReason(
                            "y".to_owned(),
                            Message("c and d are different fields".to_owned()),
                        ),
                    ]),
                ),
                &[
                    SourcePosition::new(25, 2, 12),
                    SourcePosition::new(171, 10, 12),
                    SourcePosition::new(245, 14, 12),
                    SourcePosition::new(78, 5, 12),
                    SourcePosition::new(376, 21, 12),
                    SourcePosition::new(302, 17, 12),
                ],
            )],
        );
    }

    #[test]
    fn ignores_unknown_fragments() {
        expect_passes_rule(
            factory,
            r#"
        {
          field
          ...Unknown
          ...Known
        }

        fragment Known on T {
          field
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

    impl GraphQLType for SomeBox {
        type Context = ();
        type TypeInfo = ();

        fn name(_: &()) -> Option<&'static str> {
            Some("SomeBox")
        }

        fn meta<'r>(i: &(), registry: &mut Registry<'r>) -> MetaType<'r> {
            let fields = &[
                registry.field::<Option<SomeBox>>("deepBox", i),
                registry.field::<Option<String>>("unrelatedField", i),
            ];

            registry.build_interface_type::<Self>(i, fields).into_meta()
        }
    }

    impl GraphQLType for StringBox {
        type Context = ();
        type TypeInfo = ();

        fn name(_: &()) -> Option<&'static str> {
            Some("StringBox")
        }

        fn meta<'r>(i: &(), registry: &mut Registry<'r>) -> MetaType<'r> {
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

    impl GraphQLType for IntBox {
        type Context = ();
        type TypeInfo = ();

        fn name(_: &()) -> Option<&'static str> {
            Some("IntBox")
        }

        fn meta<'r>(i: &(), registry: &mut Registry<'r>) -> MetaType<'r> {
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

    impl GraphQLType for NonNullStringBox1 {
        type Context = ();
        type TypeInfo = ();

        fn name(_: &()) -> Option<&'static str> {
            Some("NonNullStringBox1")
        }

        fn meta<'r>(i: &(), registry: &mut Registry<'r>) -> MetaType<'r> {
            let fields = &[registry.field::<String>("scalar", i)];

            registry.build_interface_type::<Self>(i, fields).into_meta()
        }
    }

    impl GraphQLType for NonNullStringBox1Impl {
        type Context = ();
        type TypeInfo = ();

        fn name(_: &()) -> Option<&'static str> {
            Some("NonNullStringBox1Impl")
        }

        fn meta<'r>(i: &(), registry: &mut Registry<'r>) -> MetaType<'r> {
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

    impl GraphQLType for NonNullStringBox2 {
        type Context = ();
        type TypeInfo = ();

        fn name(_: &()) -> Option<&'static str> {
            Some("NonNullStringBox2")
        }

        fn meta<'r>(i: &(), registry: &mut Registry<'r>) -> MetaType<'r> {
            let fields = &[registry.field::<String>("scalar", i)];

            registry.build_interface_type::<Self>(i, fields).into_meta()
        }
    }

    impl GraphQLType for NonNullStringBox2Impl {
        type Context = ();
        type TypeInfo = ();

        fn name(_: &()) -> Option<&'static str> {
            Some("NonNullStringBox2Impl")
        }

        fn meta<'r>(i: &(), registry: &mut Registry<'r>) -> MetaType<'r> {
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

    impl GraphQLType for Node {
        type Context = ();
        type TypeInfo = ();

        fn name(_: &()) -> Option<&'static str> {
            Some("Node")
        }

        fn meta<'r>(i: &(), registry: &mut Registry<'r>) -> MetaType<'r> {
            let fields = &[
                registry.field::<Option<ID>>("id", i),
                registry.field::<Option<String>>("name", i),
            ];

            registry.build_object_type::<Self>(i, fields).into_meta()
        }
    }

    impl GraphQLType for Edge {
        type Context = ();
        type TypeInfo = ();

        fn name(_: &()) -> Option<&'static str> {
            Some("Edge")
        }

        fn meta<'r>(i: &(), registry: &mut Registry<'r>) -> MetaType<'r> {
            let fields = &[registry.field::<Option<Node>>("node", i)];

            registry.build_object_type::<Self>(i, fields).into_meta()
        }
    }

    impl GraphQLType for Connection {
        type Context = ();
        type TypeInfo = ();

        fn name(_: &()) -> Option<&'static str> {
            Some("Connection")
        }

        fn meta<'r>(i: &(), registry: &mut Registry<'r>) -> MetaType<'r> {
            let fields = &[registry.field::<Option<Vec<Option<Edge>>>>("edges", i)];

            registry.build_object_type::<Self>(i, fields).into_meta()
        }
    }

    impl GraphQLType for QueryRoot {
        type Context = ();
        type TypeInfo = ();

        fn name(_: &()) -> Option<&'static str> {
            Some("QueryRoot")
        }

        fn meta<'r>(i: &(), registry: &mut Registry<'r>) -> MetaType<'r> {
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

    #[test]
    fn conflicting_return_types_which_potentially_overlap() {
        expect_fails_rule_with_schema(
            QueryRoot,
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
                    &Message("they return conflicting types Int and String!".to_owned()),
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
        expect_passes_rule_with_schema(
            QueryRoot,
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
        expect_fails_rule_with_schema(
            QueryRoot,
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
                    &Message("they return conflicting types Int and String".to_owned()),
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
        expect_fails_rule_with_schema(
            QueryRoot,
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
              scalar
            }
            fragment Y on SomeBox {
              scalar: unrelatedField
            }
        "#,
            &[RuleError::new(
                &error_message(
                    "other",
                    &Nested(vec![ConflictReason(
                        "scalar".to_owned(),
                        Message("scalar and unrelatedField are different fields".to_owned()),
                    )]),
                ),
                &[
                    SourcePosition::new(703, 30, 14),
                    SourcePosition::new(889, 38, 14),
                    SourcePosition::new(771, 33, 14),
                    SourcePosition::new(960, 41, 14),
                ],
            )],
        );
    }

    #[test]
    fn disallows_differing_return_type_nullability_despite_no_overlap() {
        expect_fails_rule_with_schema(
            QueryRoot,
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
                    &Message("they return conflicting types String! and String".to_owned()),
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
        expect_fails_rule_with_schema(
            QueryRoot,
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
                    &Message("they return conflicting types [StringBox] and StringBox".to_owned()),
                ),
                &[
                    SourcePosition::new(89, 4, 18),
                    SourcePosition::new(228, 9, 18),
                ],
            )],
        );

        expect_fails_rule_with_schema(
            QueryRoot,
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
                    &Message("they return conflicting types StringBox and [StringBox]".to_owned()),
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
        expect_fails_rule_with_schema(
            QueryRoot,
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
                    &Message("scalar and unrelatedField are different fields".to_owned()),
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
        expect_fails_rule_with_schema(
            QueryRoot,
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
                        "scalar".to_owned(),
                        Message("they return conflicting types String and Int".to_owned()),
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
        expect_passes_rule_with_schema(
            QueryRoot,
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
        expect_passes_rule_with_schema(
            QueryRoot,
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
        expect_passes_rule_with_schema(
            QueryRoot,
            factory,
            r#"
            {
              a
              ... {
                a
              }
            }
        "#,
        );
    }

    #[test]
    fn compares_deep_types_including_list() {
        expect_fails_rule_with_schema(
            QueryRoot,
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
                        "node".to_owned(),
                        Nested(vec![ConflictReason(
                            "id".to_owned(),
                            Message("name and id are different fields".to_owned()),
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
        expect_passes_rule_with_schema(
            QueryRoot,
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
            &error_message("x", &Message("a and b are different fields".to_owned())),
            "Fields \"x\" conflict because a and b are different fields. Use \
             different aliases on the fields to fetch both if this \
             was intentional"
        );
    }
}
