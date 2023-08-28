use std::collections::HashMap;

use crate::{
    ast::{Directive, Fragment, InputValue, Selection},
    parser::Spanning,
    value::ScalarValue,
};

use super::Variables;

/// An enum that describes if a field is available in all types of the interface
/// or only in a certain subtype
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Applies<'a> {
    /// The field is available independent from the type
    All,
    /// The field is only available for a given typename
    OnlyType(&'a str),
}

/// A JSON-like value that can is used as argument in the query execution
///
/// In contrast to `InputValue` these values do only contain constants,
/// meaning that variables are already resolved.
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs)]
pub enum LookAheadValue<'a, S: 'a> {
    Null,
    Scalar(&'a S),
    Enum(&'a str),
    List(Vec<LookAheadValue<'a, S>>),
    Object(Vec<(&'a str, LookAheadValue<'a, S>)>),
}

impl<'a, S> LookAheadValue<'a, S>
where
    S: ScalarValue,
{
    fn from_input_value(input_value: &'a InputValue<S>, vars: &'a Variables<S>) -> Self {
        match *input_value {
            InputValue::Null => LookAheadValue::Null,
            InputValue::Scalar(ref s) => LookAheadValue::Scalar(s),
            InputValue::Enum(ref e) => LookAheadValue::Enum(e),
            InputValue::Variable(ref name) => vars
                .get(name)
                .map(|v| Self::from_input_value(v, vars))
                .unwrap_or(LookAheadValue::Null),
            InputValue::List(ref l) => LookAheadValue::List(
                l.iter()
                    .map(|i| LookAheadValue::from_input_value(&i.item, vars))
                    .collect(),
            ),
            InputValue::Object(ref o) => LookAheadValue::Object(
                o.iter()
                    .map(|(n, i)| {
                        (
                            &n.item as &str,
                            LookAheadValue::from_input_value(&i.item, vars),
                        )
                    })
                    .collect(),
            ),
        }
    }
}

/// An argument passed into the query
#[derive(Debug, Clone, PartialEq)]
pub struct LookAheadArgument<'a, S: 'a> {
    name: &'a str,
    value: LookAheadValue<'a, S>,
}

impl<'a, S> LookAheadArgument<'a, S>
where
    S: ScalarValue,
{
    pub(super) fn new(
        (name, value): &'a (Spanning<&'a str>, Spanning<InputValue<S>>),
        vars: &'a Variables<S>,
    ) -> Self {
        LookAheadArgument {
            name: name.item,
            value: LookAheadValue::from_input_value(&value.item, vars),
        }
    }

    /// The argument's name
    pub fn name(&'a self) -> &str {
        self.name
    }

    /// The value of the argument
    pub fn value(&'a self) -> &LookAheadValue<'a, S> {
        &self.value
    }
}

/// A selection performed by a query
#[derive(Debug, Clone, PartialEq)]
pub struct LookAheadSelection<'a, S: 'a> {
    pub(super) name: &'a str,
    pub(super) alias: Option<&'a str>,
    pub(super) arguments: Vec<LookAheadArgument<'a, S>>,
    pub(super) children: Vec<LookAheadSelection<'a, S>>,
    pub(super) applies_for: Applies<'a>,
}

// Implemented manually to omit redundant `S: Default` trait bound, imposed by
// `#[derive(Default)]`.
impl<'a, S: 'a> Default for LookAheadSelection<'a, S> {
    fn default() -> Self {
        Self {
            name: "",
            alias: None,
            arguments: vec![],
            children: vec![],
            applies_for: Applies::All,
        }
    }
}

impl<'a, S> LookAheadSelection<'a, S>
where
    S: ScalarValue,
{
    fn should_include<'b, 'c>(
        directives: Option<&'b Vec<Spanning<Directive<S>>>>,
        vars: &'c Variables<S>,
    ) -> bool
    where
        'b: 'a,
        'c: 'a,
    {
        directives
            .map(|d| {
                d.iter().all(|d| {
                    let d = &d.item;
                    let arguments = &d.arguments;
                    match (d.name.item, arguments) {
                        ("include", Some(a)) => a
                            .item
                            .items
                            .iter()
                            .find(|item| item.0.item == "if")
                            .map(|(_, v)| {
                                if let LookAheadValue::Scalar(s) =
                                    LookAheadValue::from_input_value(&v.item, vars)
                                {
                                    s.as_bool().unwrap_or(false)
                                } else {
                                    false
                                }
                            })
                            .unwrap_or(false),
                        ("skip", Some(a)) => a
                            .item
                            .items
                            .iter()
                            .find(|item| item.0.item == "if")
                            .map(|(_, v)| {
                                if let LookAheadValue::Scalar(b) =
                                    LookAheadValue::from_input_value(&v.item, vars)
                                {
                                    b.as_bool().map(::std::ops::Not::not).unwrap_or(false)
                                } else {
                                    false
                                }
                            })
                            .unwrap_or(false),
                        ("skip", &None) => false,
                        ("include", &None) => true,
                        (_, _) => unreachable!(),
                    }
                })
            })
            .unwrap_or(true)
    }

    pub(super) fn build_from_selection(
        s: &'a Selection<'a, S>,
        vars: &'a Variables<S>,
        fragments: &'a HashMap<&'a str, Fragment<'a, S>>,
    ) -> Option<LookAheadSelection<'a, S>> {
        Self::build_from_selection_with_parent(s, None, vars, fragments)
    }

    pub(super) fn build_from_selection_with_parent(
        s: &'a Selection<'a, S>,
        parent: Option<&mut Self>,
        vars: &'a Variables<S>,
        fragments: &'a HashMap<&'a str, Fragment<'a, S>>,
    ) -> Option<LookAheadSelection<'a, S>> {
        let empty: &[Selection<S>] = &[];
        match *s {
            Selection::Field(ref field) => {
                let field = &field.item;
                let include = Self::should_include(field.directives.as_ref(), vars);
                if !include {
                    return None;
                }
                let name = field.name.item;
                let alias = field.alias.as_ref().map(|a| a.item);
                let arguments = field
                    .arguments
                    .as_ref()
                    .map(|a| &a.item)
                    .map(|a| {
                        a.items
                            .iter()
                            .map(|p| LookAheadArgument::new(p, vars))
                            .collect()
                    })
                    .unwrap_or_else(Vec::new);
                let applies_for = match &parent {
                    Some(p) => p.applies_for,
                    None => Applies::All,
                };

                let mut ret = LookAheadSelection {
                    name,
                    alias,
                    arguments,
                    children: Vec::new(),
                    applies_for,
                };
                for c in field
                    .selection_set
                    .as_ref()
                    .map(|s| s as &[_])
                    .unwrap_or_else(|| empty)
                    .iter()
                {
                    let s = LookAheadSelection::build_from_selection_with_parent(
                        c,
                        Some(&mut ret),
                        vars,
                        fragments,
                    );
                    assert!(s.is_none());
                }
                if let Some(p) = parent {
                    p.children.push(ret);
                    None
                } else {
                    Some(ret)
                }
            }
            Selection::FragmentSpread(ref fragment) => {
                let include = Self::should_include(fragment.item.directives.as_ref(), vars);
                if !include {
                    return None;
                }
                let f = fragments.get(&fragment.item.name.item).expect("a fragment");
                if let Some(parent) = parent {
                    for c in f.selection_set.iter() {
                        let s = LookAheadSelection::build_from_selection_with_parent(
                            c,
                            Some(parent),
                            vars,
                            fragments,
                        );
                        assert!(s.is_none());
                    }
                } else {
                    for c in f.selection_set.iter() {
                        let s = LookAheadSelection::build_from_selection_with_parent(
                            c, None, vars, fragments,
                        );
                        assert!(s.is_some());
                    }
                }
                None
            }
            Selection::InlineFragment(ref inline) if parent.is_some() => {
                let include = Self::should_include(inline.item.directives.as_ref(), vars);
                if !include {
                    return None;
                }
                let parent = parent.unwrap();
                for c in inline.item.selection_set.iter() {
                    let s = LookAheadSelection::build_from_selection_with_parent(
                        c,
                        Some(parent),
                        vars,
                        fragments,
                    );
                    assert!(s.is_none());
                    if let Some(c) = inline.item.type_condition.as_ref().map(|t| t.item) {
                        if let Some(p) = parent.children.last_mut() {
                            p.applies_for = Applies::OnlyType(c);
                        }
                    }
                }
                None
            }
            _ => unimplemented!(),
        }
    }

    /// Convert a eventually type independent selection into one for a concrete type
    pub fn for_explicit_type(&self, type_name: &str) -> ConcreteLookAheadSelection<'a, S> {
        ConcreteLookAheadSelection {
            children: self
                .children
                .iter()
                .filter_map(|c| match c.applies_for {
                    Applies::OnlyType(t) if t == type_name => Some(c.for_explicit_type(type_name)),
                    Applies::All => Some(c.for_explicit_type(type_name)),
                    Applies::OnlyType(_) => None,
                })
                .collect(),
            name: self.name,
            alias: self.alias,
            arguments: self.arguments.clone(),
            applies_for: self.applies_for,
        }
    }
}

/// A selection performed by a query on a concrete type
#[derive(Debug, PartialEq)]
pub struct ConcreteLookAheadSelection<'a, S: 'a> {
    name: &'a str,
    alias: Option<&'a str>,
    arguments: Vec<LookAheadArgument<'a, S>>,
    children: Vec<ConcreteLookAheadSelection<'a, S>>,
    applies_for: Applies<'a>,
}

/// Set of common methods for `ConcreteLookAheadSelection` and `LookAheadSelection`.
///
/// `'sel` lifetime is intended to point to the data that this `LookAheadSelection` (or
/// `ConcreteLookAheadSelection`) points to.
pub trait LookAheadMethods<'sel, S> {
    /// Get the (potentially aliased) name of the field represented by the current selection
    fn field_name(&self) -> &'sel str;

    /// Get the the child selection for a given field
    /// If a child has an alias, it will only match if the alias matches `name`
    fn select_child(&self, name: &str) -> Option<&Self>;

    /// Check if a given child selection with a name exists
    /// If a child has an alias, it will only match if the alias matches `name`
    fn has_child(&self, name: &str) -> bool {
        self.select_child(name).is_some()
    }

    /// Does the current node have any arguments?
    fn has_arguments(&self) -> bool;

    /// Does the current node have any children?
    fn has_children(&self) -> bool;

    /// Get the top level arguments for the current selection
    fn arguments(&self) -> &[LookAheadArgument<S>];

    /// Get the top level argument with a given name from the current selection
    fn argument(&self, name: &str) -> Option<&LookAheadArgument<S>> {
        self.arguments().iter().find(|a| a.name == name)
    }

    /// Get the (possibly aliased) names of the top level children for the current selection
    fn child_names(&self) -> Vec<&'sel str>;

    /// Get an iterator over the children for the current selection
    fn children(&self) -> Vec<&Self>;

    /// Get the parent type in case there is any for this selection
    fn applies_for(&self) -> Option<&str>;
}

impl<'a, S> LookAheadMethods<'a, S> for ConcreteLookAheadSelection<'a, S> {
    fn field_name(&self) -> &'a str {
        self.alias.unwrap_or(self.name)
    }

    fn select_child(&self, name: &str) -> Option<&Self> {
        self.children.iter().find(|c| c.field_name() == name)
    }

    fn arguments(&self) -> &[LookAheadArgument<S>] {
        &self.arguments
    }

    fn child_names(&self) -> Vec<&'a str> {
        self.children.iter().map(|c| c.field_name()).collect()
    }

    fn has_arguments(&self) -> bool {
        !self.arguments.is_empty()
    }

    fn has_children(&self) -> bool {
        !self.children.is_empty()
    }

    fn children(&self) -> Vec<&Self> {
        self.children.iter().collect()
    }

    fn applies_for(&self) -> Option<&str> {
        match self.applies_for {
            Applies::OnlyType(typ) => Some(typ),
            Applies::All => None,
        }
    }
}

impl<'a, S> LookAheadMethods<'a, S> for LookAheadSelection<'a, S> {
    fn field_name(&self) -> &'a str {
        self.alias.unwrap_or(self.name)
    }

    fn select_child(&self, name: &str) -> Option<&Self> {
        self.children.iter().find(|c| c.field_name() == name)
    }

    fn arguments(&self) -> &[LookAheadArgument<S>] {
        &self.arguments
    }

    fn child_names(&self) -> Vec<&'a str> {
        self.children.iter().map(|c| c.field_name()).collect()
    }

    fn has_arguments(&self) -> bool {
        !self.arguments.is_empty()
    }

    fn has_children(&self) -> bool {
        !self.children.is_empty()
    }

    fn children(&self) -> Vec<&Self> {
        self.children.iter().collect()
    }

    fn applies_for(&self) -> Option<&str> {
        match self.applies_for {
            Applies::OnlyType(typ) => Some(typ),
            Applies::All => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::{
        ast::{Document, OwnedDocument},
        graphql_vars,
        parser::UnlocatedParseResult,
        schema::model::SchemaType,
        validation::test_harness::{MutationRoot, QueryRoot, SubscriptionRoot},
        value::{DefaultScalarValue, ScalarValue},
    };

    use super::*;

    fn parse_document_source<S>(q: &str) -> UnlocatedParseResult<OwnedDocument<S>>
    where
        S: ScalarValue,
    {
        crate::parse_document_source(
            q,
            &SchemaType::new::<QueryRoot, MutationRoot, SubscriptionRoot>(&(), &(), &()),
        )
    }

    fn extract_fragments<'a, S>(doc: &'a Document<S>) -> HashMap<&'a str, Fragment<'a, S>>
    where
        S: Clone,
    {
        let mut fragments = HashMap::new();
        for d in doc {
            if let crate::ast::Definition::Fragment(ref f) = *d {
                let f = f.item.clone();
                fragments.insert(f.name.item, f);
            }
        }
        fragments
    }

    #[test]
    fn check_simple_query() {
        let docs = parse_document_source::<DefaultScalarValue>(
            "
query Hero {
    hero {
        id
        name
    }
}
",
        )
        .unwrap();
        let fragments = extract_fragments(&docs);

        if let crate::ast::Definition::Operation(ref op) = docs[0] {
            let vars = graphql_vars! {};
            let look_ahead = LookAheadSelection::build_from_selection(
                &op.item.selection_set[0],
                &vars,
                &fragments,
            )
            .unwrap();
            let expected = LookAheadSelection {
                name: "hero",
                alias: None,
                arguments: Vec::new(),
                applies_for: Applies::All,
                children: vec![
                    LookAheadSelection {
                        name: "id",
                        alias: None,
                        arguments: Vec::new(),
                        children: Vec::new(),
                        applies_for: Applies::All,
                    },
                    LookAheadSelection {
                        name: "name",
                        alias: None,
                        arguments: Vec::new(),
                        children: Vec::new(),
                        applies_for: Applies::All,
                    },
                ],
            };
            assert_eq!(look_ahead, expected);
        } else {
            panic!("No Operation found");
        }
    }

    #[test]
    fn check_query_with_alias() {
        let docs = parse_document_source::<DefaultScalarValue>(
            "
query Hero {
    custom_hero: hero {
        id
        my_name: name
    }
}
",
        )
        .unwrap();
        let fragments = extract_fragments(&docs);

        if let crate::ast::Definition::Operation(ref op) = docs[0] {
            let vars = graphql_vars! {};
            let look_ahead = LookAheadSelection::build_from_selection(
                &op.item.selection_set[0],
                &vars,
                &fragments,
            )
            .unwrap();
            let expected = LookAheadSelection {
                name: "hero",
                alias: Some("custom_hero"),
                arguments: Vec::new(),
                applies_for: Applies::All,
                children: vec![
                    LookAheadSelection {
                        name: "id",
                        alias: None,
                        arguments: Vec::new(),
                        children: Vec::new(),
                        applies_for: Applies::All,
                    },
                    LookAheadSelection {
                        name: "name",
                        alias: Some("my_name"),
                        arguments: Vec::new(),
                        children: Vec::new(),
                        applies_for: Applies::All,
                    },
                ],
            };
            assert_eq!(look_ahead, expected);
        } else {
            panic!("No Operation found");
        }
    }

    #[test]
    fn check_query_with_child() {
        let docs = parse_document_source::<DefaultScalarValue>(
            "
query Hero {
    hero {
        id
        name
        friends {
            name
            id
        }
    }
}
",
        )
        .unwrap();
        let fragments = extract_fragments(&docs);

        if let crate::ast::Definition::Operation(ref op) = docs[0] {
            let vars = graphql_vars! {};
            let look_ahead = LookAheadSelection::build_from_selection(
                &op.item.selection_set[0],
                &vars,
                &fragments,
            )
            .unwrap();
            let expected = LookAheadSelection {
                name: "hero",
                alias: None,
                arguments: Vec::new(),
                applies_for: Applies::All,
                children: vec![
                    LookAheadSelection {
                        name: "id",
                        alias: None,
                        arguments: Vec::new(),
                        children: Vec::new(),
                        applies_for: Applies::All,
                    },
                    LookAheadSelection {
                        name: "name",
                        alias: None,
                        arguments: Vec::new(),
                        children: Vec::new(),
                        applies_for: Applies::All,
                    },
                    LookAheadSelection {
                        name: "friends",
                        alias: None,
                        arguments: Vec::new(),
                        children: vec![
                            LookAheadSelection {
                                name: "name",
                                alias: None,
                                arguments: Vec::new(),
                                children: Vec::new(),
                                applies_for: Applies::All,
                            },
                            LookAheadSelection {
                                name: "id",
                                alias: None,
                                arguments: Vec::new(),
                                children: Vec::new(),
                                applies_for: Applies::All,
                            },
                        ],
                        applies_for: Applies::All,
                    },
                ],
            };
            assert_eq!(look_ahead, expected);
        } else {
            panic!("No Operation found");
        }
    }

    #[test]
    fn check_query_with_argument() {
        let docs = parse_document_source(
            "
query Hero {
    hero(episode: EMPIRE) {
        id
        name(uppercase: true)
    }
}
",
        )
        .unwrap();
        let fragments = extract_fragments(&docs);

        if let crate::ast::Definition::Operation(ref op) = docs[0] {
            let vars = graphql_vars! {};
            let look_ahead = LookAheadSelection::build_from_selection(
                &op.item.selection_set[0],
                &vars,
                &fragments,
            )
            .unwrap();
            let expected = LookAheadSelection {
                name: "hero",
                alias: None,
                arguments: vec![LookAheadArgument {
                    name: "episode",
                    value: LookAheadValue::Enum("EMPIRE"),
                }],
                applies_for: Applies::All,
                children: vec![
                    LookAheadSelection {
                        name: "id",
                        alias: None,
                        arguments: Vec::new(),
                        children: Vec::new(),
                        applies_for: Applies::All,
                    },
                    LookAheadSelection {
                        name: "name",
                        alias: None,
                        arguments: vec![LookAheadArgument {
                            name: "uppercase",
                            value: LookAheadValue::Scalar(&DefaultScalarValue::Boolean(true)),
                        }],
                        children: Vec::new(),
                        applies_for: Applies::All,
                    },
                ],
            };
            assert_eq!(look_ahead, expected);
        } else {
            panic!("No Operation found");
        }
    }

    #[test]
    fn check_query_with_variable() {
        let docs = parse_document_source::<DefaultScalarValue>(
            "
query Hero($episode: Episode) {
    hero(episode: $episode) {
        id
        name
    }
}
",
        )
        .unwrap();
        let fragments = extract_fragments(&docs);

        if let crate::ast::Definition::Operation(ref op) = docs[0] {
            let vars = graphql_vars! {"episode": JEDI};
            let look_ahead = LookAheadSelection::build_from_selection(
                &op.item.selection_set[0],
                &vars,
                &fragments,
            )
            .unwrap();
            let expected = LookAheadSelection {
                name: "hero",
                alias: None,
                arguments: vec![LookAheadArgument {
                    name: "episode",
                    value: LookAheadValue::Enum("JEDI"),
                }],
                applies_for: Applies::All,
                children: vec![
                    LookAheadSelection {
                        name: "id",
                        alias: None,
                        arguments: Vec::new(),
                        children: Vec::new(),
                        applies_for: Applies::All,
                    },
                    LookAheadSelection {
                        name: "name",
                        alias: None,
                        arguments: Vec::new(),
                        children: Vec::new(),
                        applies_for: Applies::All,
                    },
                ],
            };
            assert_eq!(look_ahead, expected);
        } else {
            panic!("No Operation found");
        }
    }

    #[test]
    fn check_query_with_optional_variable() {
        let docs = parse_document_source::<DefaultScalarValue>(
            "
query Hero($episode: Episode) {
    hero(episode: $episode) {
        id
    }
}
",
        )
        .unwrap();
        let fragments = extract_fragments(&docs);

        if let crate::ast::Definition::Operation(ref op) = docs[0] {
            let vars = graphql_vars! {};
            let look_ahead = LookAheadSelection::build_from_selection(
                &op.item.selection_set[0],
                &vars,
                &fragments,
            )
            .unwrap();
            let expected = LookAheadSelection {
                name: "hero",
                alias: None,
                arguments: vec![LookAheadArgument {
                    name: "episode",
                    value: LookAheadValue::Null,
                }],
                applies_for: Applies::All,
                children: vec![LookAheadSelection {
                    name: "id",
                    alias: None,
                    arguments: Vec::new(),
                    children: Vec::new(),
                    applies_for: Applies::All,
                }],
            };
            assert_eq!(look_ahead, expected);
        } else {
            panic!("No Operation found");
        }
    }
    #[test]
    fn check_query_with_fragment() {
        let docs = parse_document_source::<DefaultScalarValue>(
            "
query Hero {
    hero {
        id
        ...commonFields
    }
}

fragment commonFields on Character {
  name
  appearsIn
}
",
        )
        .unwrap();
        let fragments = extract_fragments(&docs);

        if let crate::ast::Definition::Operation(ref op) = docs[0] {
            let vars = graphql_vars! {};
            let look_ahead = LookAheadSelection::build_from_selection(
                &op.item.selection_set[0],
                &vars,
                &fragments,
            )
            .unwrap();
            let expected = LookAheadSelection {
                name: "hero",
                alias: None,
                arguments: Vec::new(),
                applies_for: Applies::All,
                children: vec![
                    LookAheadSelection {
                        name: "id",
                        alias: None,
                        arguments: Vec::new(),
                        children: Vec::new(),
                        applies_for: Applies::All,
                    },
                    LookAheadSelection {
                        name: "name",
                        alias: None,
                        arguments: Vec::new(),
                        children: Vec::new(),
                        applies_for: Applies::All,
                    },
                    LookAheadSelection {
                        name: "appearsIn",
                        alias: None,
                        arguments: Vec::new(),
                        children: Vec::new(),
                        applies_for: Applies::All,
                    },
                ],
            };
            assert_eq!(look_ahead, expected);
        } else {
            panic!("No Operation found");
        }
    }

    #[test]
    fn check_query_with_directives() {
        let docs = parse_document_source::<DefaultScalarValue>(
            "
query Hero {
    hero {
        id @include(if: true)
        name @include(if: false)
        appearsIn @skip(if: true)
        height @skip(if: false)
    }
}",
        )
        .unwrap();
        let fragments = extract_fragments(&docs);

        if let crate::ast::Definition::Operation(ref op) = docs[0] {
            let vars = graphql_vars! {};
            let look_ahead = LookAheadSelection::build_from_selection(
                &op.item.selection_set[0],
                &vars,
                &fragments,
            )
            .unwrap();
            let expected = LookAheadSelection {
                name: "hero",
                alias: None,
                arguments: Vec::new(),
                applies_for: Applies::All,
                children: vec![
                    LookAheadSelection {
                        name: "id",
                        alias: None,
                        arguments: Vec::new(),
                        children: Vec::new(),
                        applies_for: Applies::All,
                    },
                    LookAheadSelection {
                        name: "height",
                        alias: None,
                        arguments: Vec::new(),
                        children: Vec::new(),
                        applies_for: Applies::All,
                    },
                ],
            };
            assert_eq!(look_ahead, expected);
        } else {
            panic!("No Operation found");
        }
    }

    #[test]
    fn check_query_with_inline_fragments() {
        let docs = parse_document_source::<DefaultScalarValue>(
            "
query Hero {
    hero {
        name
        ... on Droid {
            primaryFunction
        }
        ... on Human {
            height
        }
    }
}",
        )
        .unwrap();
        let fragments = extract_fragments(&docs);

        if let crate::ast::Definition::Operation(ref op) = docs[0] {
            let vars = graphql_vars! {};
            let look_ahead = LookAheadSelection::build_from_selection(
                &op.item.selection_set[0],
                &vars,
                &fragments,
            )
            .unwrap();
            let expected = LookAheadSelection {
                name: "hero",
                alias: None,
                arguments: Vec::new(),
                applies_for: Applies::All,
                children: vec![
                    LookAheadSelection {
                        name: "name",
                        alias: None,
                        arguments: Vec::new(),
                        children: Vec::new(),
                        applies_for: Applies::All,
                    },
                    LookAheadSelection {
                        name: "primaryFunction",
                        alias: None,
                        arguments: Vec::new(),
                        children: Vec::new(),
                        applies_for: Applies::OnlyType("Droid"),
                    },
                    LookAheadSelection {
                        name: "height",
                        alias: None,
                        arguments: Vec::new(),
                        children: Vec::new(),
                        applies_for: Applies::OnlyType("Human"),
                    },
                ],
            };
            assert_eq!(look_ahead, expected);
        } else {
            panic!("No Operation found");
        }
    }

    #[test]
    fn check_query_with_multiple() {
        let docs = parse_document_source::<DefaultScalarValue>(
            "
query HeroAndHuman {
    hero {
        id
    }
    human {
        name
    }
}
",
        )
        .unwrap();
        let fragments = extract_fragments(&docs);

        if let crate::ast::Definition::Operation(ref op) = docs[0] {
            let vars = graphql_vars! {};
            let look_ahead = LookAheadSelection::build_from_selection(
                &op.item.selection_set[0],
                &vars,
                &fragments,
            )
            .unwrap();
            let expected = LookAheadSelection {
                name: "hero",
                alias: None,
                arguments: Vec::new(),
                applies_for: Applies::All,
                children: vec![LookAheadSelection {
                    name: "id",
                    alias: None,
                    arguments: Vec::new(),
                    children: Vec::new(),
                    applies_for: Applies::All,
                }],
            };
            assert_eq!(look_ahead, expected);

            let look_ahead = LookAheadSelection::build_from_selection(
                &op.item.selection_set[1],
                &vars,
                &fragments,
            )
            .unwrap();
            let expected = LookAheadSelection {
                name: "human",
                alias: None,
                arguments: Vec::new(),
                applies_for: Applies::All,
                children: vec![LookAheadSelection {
                    name: "name",
                    alias: None,
                    arguments: Vec::new(),
                    children: Vec::new(),
                    applies_for: Applies::All,
                }],
            };
            assert_eq!(look_ahead, expected);
        } else {
            panic!("No Operation found");
        }
    }

    #[test]
    fn check_complex_query() {
        let docs = parse_document_source(
            "
query HeroNameAndFriends($id: Integer!, $withFriends: Boolean! = true) {
  hero(id: $id) {
    id
    ... comparisonFields
    friends @include(if: $withFriends) {
      ... comparisonFields
      ... on Human @skip(if: true) { mass }
    }
  }
}

fragment comparisonFields on Character {
  __typename
  name
  appearsIn
  ... on Droid { primaryFunction }
  ... on Human { height }
}",
        )
        .unwrap();
        let fragments = extract_fragments(&docs);

        if let crate::ast::Definition::Operation(ref op) = docs[0] {
            let vars = graphql_vars! {
                "id": 42,
                "withFriends": true,
            };
            let look_ahead = LookAheadSelection::build_from_selection(
                &op.item.selection_set[0],
                &vars,
                &fragments,
            )
            .unwrap();
            let expected = LookAheadSelection {
                name: "hero",
                alias: None,
                arguments: vec![LookAheadArgument {
                    name: "id",
                    value: LookAheadValue::Scalar(&DefaultScalarValue::Int(42)),
                }],
                applies_for: Applies::All,
                children: vec![
                    LookAheadSelection {
                        name: "id",
                        alias: None,
                        arguments: Vec::new(),
                        children: Vec::new(),
                        applies_for: Applies::All,
                    },
                    LookAheadSelection {
                        name: "__typename",
                        alias: None,
                        arguments: Vec::new(),
                        children: Vec::new(),
                        applies_for: Applies::All,
                    },
                    LookAheadSelection {
                        name: "name",
                        alias: None,
                        arguments: Vec::new(),
                        children: Vec::new(),
                        applies_for: Applies::All,
                    },
                    LookAheadSelection {
                        name: "appearsIn",
                        alias: None,
                        arguments: Vec::new(),
                        children: Vec::new(),
                        applies_for: Applies::All,
                    },
                    LookAheadSelection {
                        name: "primaryFunction",
                        alias: None,
                        arguments: Vec::new(),
                        children: Vec::new(),
                        applies_for: Applies::OnlyType("Droid"),
                    },
                    LookAheadSelection {
                        name: "height",
                        alias: None,
                        arguments: Vec::new(),
                        children: Vec::new(),
                        applies_for: Applies::OnlyType("Human"),
                    },
                    LookAheadSelection {
                        name: "friends",
                        alias: None,
                        arguments: Vec::new(),
                        applies_for: Applies::All,
                        children: vec![
                            LookAheadSelection {
                                name: "__typename",
                                alias: None,
                                arguments: Vec::new(),
                                children: Vec::new(),
                                applies_for: Applies::All,
                            },
                            LookAheadSelection {
                                name: "name",
                                alias: None,
                                arguments: Vec::new(),
                                children: Vec::new(),
                                applies_for: Applies::All,
                            },
                            LookAheadSelection {
                                name: "appearsIn",
                                alias: None,
                                arguments: Vec::new(),
                                children: Vec::new(),
                                applies_for: Applies::All,
                            },
                            LookAheadSelection {
                                name: "primaryFunction",
                                alias: None,
                                arguments: Vec::new(),
                                children: Vec::new(),
                                applies_for: Applies::OnlyType("Droid"),
                            },
                            LookAheadSelection {
                                name: "height",
                                alias: None,
                                arguments: Vec::new(),
                                children: Vec::new(),
                                applies_for: Applies::OnlyType("Human"),
                            },
                        ],
                    },
                ],
            };
            assert_eq!(look_ahead, expected);
        } else {
            panic!("No Operation found");
        }
    }

    #[test]
    fn check_resolve_concrete_type() {
        let docs = parse_document_source::<DefaultScalarValue>(
            "
query Hero {
    hero {
        name
        ... on Droid {
            primaryFunction
        }
        ... on Human {
            height
        }
    }
}",
        )
        .unwrap();
        let fragments = extract_fragments(&docs);

        if let crate::ast::Definition::Operation(ref op) = docs[0] {
            let vars = graphql_vars! {};
            let look_ahead = LookAheadSelection::build_from_selection(
                &op.item.selection_set[0],
                &vars,
                &fragments,
            )
            .unwrap()
            .for_explicit_type("Human");
            let expected = ConcreteLookAheadSelection {
                name: "hero",
                alias: None,
                arguments: Vec::new(),
                applies_for: Applies::All,
                children: vec![
                    ConcreteLookAheadSelection {
                        name: "name",
                        alias: None,
                        arguments: Vec::new(),
                        children: Vec::new(),
                        applies_for: Applies::All,
                    },
                    ConcreteLookAheadSelection {
                        name: "height",
                        alias: None,
                        arguments: Vec::new(),
                        children: Vec::new(),
                        applies_for: Applies::OnlyType("Human"),
                    },
                ],
            };
            assert_eq!(look_ahead, expected);
        } else {
            panic!("No Operation found");
        }
    }

    #[test]
    fn check_select_child() {
        let lookahead: LookAheadSelection<DefaultScalarValue> = LookAheadSelection {
            name: "hero",
            alias: None,
            arguments: Vec::new(),
            applies_for: Applies::All,
            children: vec![
                LookAheadSelection {
                    name: "id",
                    alias: None,
                    arguments: Vec::new(),
                    children: Vec::new(),
                    applies_for: Applies::All,
                },
                LookAheadSelection {
                    name: "friends",
                    alias: None,
                    arguments: Vec::new(),
                    applies_for: Applies::All,
                    children: vec![
                        LookAheadSelection {
                            name: "id",
                            alias: None,
                            arguments: Vec::new(),
                            children: Vec::new(),
                            applies_for: Applies::All,
                        },
                        LookAheadSelection {
                            name: "name",
                            alias: None,
                            arguments: Vec::new(),
                            children: Vec::new(),
                            applies_for: Applies::All,
                        },
                    ],
                },
            ],
        };
        let concret_query = lookahead.for_explicit_type("does not matter");

        let id = lookahead.select_child("id");
        let concrete_id = concret_query.select_child("id");
        let expected = LookAheadSelection {
            name: "id",
            alias: None,
            arguments: Vec::new(),
            children: Vec::new(),
            applies_for: Applies::All,
        };
        assert_eq!(id, Some(&expected));
        assert_eq!(
            concrete_id,
            Some(&expected.for_explicit_type("does not matter"))
        );

        let friends = lookahead.select_child("friends");
        let concrete_friends = concret_query.select_child("friends");
        let expected = LookAheadSelection {
            name: "friends",
            alias: None,
            arguments: Vec::new(),
            applies_for: Applies::All,
            children: vec![
                LookAheadSelection {
                    name: "id",
                    alias: None,
                    arguments: Vec::new(),
                    children: Vec::new(),
                    applies_for: Applies::All,
                },
                LookAheadSelection {
                    name: "name",
                    alias: None,
                    arguments: Vec::new(),
                    children: Vec::new(),
                    applies_for: Applies::All,
                },
            ],
        };
        assert_eq!(friends, Some(&expected));
        assert_eq!(
            concrete_friends,
            Some(&expected.for_explicit_type("does not matter"))
        );
    }

    #[test]
    // https://github.com/graphql-rust/juniper/issues/335
    fn check_fragment_with_nesting() {
        let docs = parse_document_source::<DefaultScalarValue>(
            "
query Hero {
    hero {
        ...heroFriendNames
    }
}

fragment heroFriendNames on Hero {
  friends { name }
}
",
        )
        .unwrap();
        let fragments = extract_fragments(&docs);

        if let crate::ast::Definition::Operation(ref op) = docs[0] {
            let vars = graphql_vars! {};
            let look_ahead = LookAheadSelection::build_from_selection(
                &op.item.selection_set[0],
                &vars,
                &fragments,
            )
            .unwrap();
            let expected = LookAheadSelection {
                name: "hero",
                alias: None,
                arguments: Vec::new(),
                applies_for: Applies::All,
                children: vec![LookAheadSelection {
                    name: "friends",
                    alias: None,
                    arguments: Vec::new(),
                    applies_for: Applies::All,
                    children: vec![LookAheadSelection {
                        name: "name",
                        alias: None,
                        arguments: Vec::new(),
                        children: Vec::new(),
                        applies_for: Applies::All,
                    }],
                }],
            };
            assert_eq!(look_ahead, expected);
        } else {
            panic!("No Operation found");
        }
    }

    #[test]
    fn check_visitability() {
        let docs = parse_document_source::<DefaultScalarValue>(
            "
query Hero {
    hero(episode: EMPIRE) {
        name
        aliasedName: name
        friends {
            name
        }
    }
}
            ",
        )
        .unwrap();
        let fragments = extract_fragments(&docs);

        if let crate::ast::Definition::Operation(ref op) = docs[0] {
            let vars = graphql_vars! {};
            let look_ahead = LookAheadSelection::build_from_selection(
                &op.item.selection_set[0],
                &vars,
                &fragments,
            )
            .unwrap();

            assert_eq!(look_ahead.field_name(), "hero");

            assert!(look_ahead.has_arguments());
            let args = look_ahead.arguments();
            assert_eq!(args[0].name(), "episode");
            assert_eq!(args[0].value(), &LookAheadValue::Enum("EMPIRE"));

            assert!(look_ahead.has_children());
            assert_eq!(
                look_ahead.child_names(),
                vec!["name", "aliasedName", "friends"]
            );
            let mut children = look_ahead.children().into_iter();

            let name_child = children.next().unwrap();
            assert!(look_ahead.has_child("name"));
            assert_eq!(name_child, look_ahead.select_child("name").unwrap());
            assert_eq!(name_child.name, "name");
            assert_eq!(name_child.alias, None);
            assert_eq!(name_child.field_name(), "name");
            assert!(!name_child.has_arguments());
            assert!(!name_child.has_children());

            let aliased_name_child = children.next().unwrap();
            assert!(look_ahead.has_child("aliasedName"));
            assert_eq!(
                aliased_name_child,
                look_ahead.select_child("aliasedName").unwrap()
            );
            assert_eq!(aliased_name_child.name, "name");
            assert_eq!(aliased_name_child.alias, Some("aliasedName"));
            assert_eq!(aliased_name_child.field_name(), "aliasedName");
            assert!(!aliased_name_child.has_arguments());
            assert!(!aliased_name_child.has_children());

            let friends_child = children.next().unwrap();
            assert!(look_ahead.has_child("friends"));
            assert_eq!(friends_child, look_ahead.select_child("friends").unwrap());
            assert_eq!(friends_child.name, "friends");
            assert_eq!(friends_child.alias, None);
            assert_eq!(friends_child.field_name(), "friends");
            assert!(!friends_child.has_arguments());
            assert!(friends_child.has_children());
            assert_eq!(friends_child.child_names(), vec!["name"]);

            assert!(children.next().is_none());

            let mut friends_children = friends_child.children().into_iter();
            let child = friends_children.next().unwrap();
            assert!(friends_child.has_child("name"));
            assert_eq!(child, friends_child.select_child("name").unwrap());
            assert_eq!(child.name, "name");
            assert_eq!(child.alias, None);
            assert_eq!(child.field_name(), "name");
            assert!(!child.has_arguments());
            assert!(!child.has_children());

            assert!(friends_children.next().is_none());
        } else {
            panic!("No Operation found");
        }
    }

    #[test]
    fn check_resolves_applies_for() {
        let docs = parse_document_source::<DefaultScalarValue>(
            "
query Hero {
    hero {
        ... on Human {
            height
        }
    }
}",
        )
        .unwrap();
        let fragments = extract_fragments(&docs);

        if let crate::ast::Definition::Operation(ref op) = docs[0] {
            let vars = graphql_vars! {};
            let look_ahead = LookAheadSelection::build_from_selection(
                &op.item.selection_set[0],
                &vars,
                &fragments,
            )
            .unwrap()
            .for_explicit_type("Human");

            let mut children = look_ahead.children().into_iter();
            let heights_child = children.next().unwrap();
            assert_eq!(heights_child.name, "height");
            assert_eq!(heights_child.applies_for, Applies::OnlyType("Human"));
            assert_eq!(heights_child.applies_for().unwrap(), "Human");
        } else {
            panic!("No Operation found");
        }
    }
}
