use crate::{
    ast::{Directive, Fragment, InputValue, Selection},
    parser::Spanning,
    value::{ScalarRefValue, ScalarValue},
};

use std::collections::HashMap;

use super::Variables;

/// An enum that describes if a field is available in all types of the interface
/// or only in a certain subtype
#[derive(Debug, Clone, PartialEq)]
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
            InputValue::Variable(ref name) => {
                let value = vars
                    .get(name)
                    .map(|v| Self::from_input_value(v, vars))
                    .unwrap_or(LookAheadValue::Null);
                value
            }
            InputValue::List(ref l) => LookAheadValue::List(
                l.iter()
                    .map(|i| LookAheadValue::from_input_value(&i.item, vars))
                    .collect(),
            ),
            InputValue::Object(ref o) => LookAheadValue::Object(
                o.iter()
                    .map(|&(ref n, ref i)| {
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
        &(ref name, ref value): &'a (Spanning<&'a str>, Spanning<InputValue<S>>),
        vars: &'a Variables<S>,
    ) -> Self {
        LookAheadArgument {
            name: name.item,
            value: LookAheadValue::from_input_value(&value.item, vars),
        }
    }

    /// The argument's name
    pub fn name(&'a self) -> &str {
        &self.name
    }

    /// The value of the argument
    pub fn value(&'a self) -> &LookAheadValue<'a, S> {
        &self.value
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChildSelection<'a, S: 'a> {
    pub(super) inner: LookAheadSelection<'a, S>,
    pub(super) applies_for: Applies<'a>,
}

/// A selection performed by a query
#[derive(Debug, Clone, PartialEq)]
pub struct LookAheadSelection<'a, S: 'a> {
    pub(super) name: &'a str,
    pub(super) alias: Option<&'a str>,
    pub(super) arguments: Vec<LookAheadArgument<'a, S>>,
    pub(super) children: Vec<ChildSelection<'a, S>>,
}

impl<'a, S> Default for LookAheadSelection<'a, S>
where
    S: ScalarValue,
    &'a S: ScalarRefValue<'a>,
{
    fn default() -> Self {
        LookAheadSelection {
            name: "",
            alias: None,
            arguments: vec![],
            children: vec![],
        }
    }
}

impl<'a, S> LookAheadSelection<'a, S>
where
    S: ScalarValue,
    &'a S: ScalarRefValue<'a>,
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
                        ("include", &Some(ref a)) => a
                            .item
                            .items
                            .iter()
                            .find(|item| item.0.item == "if")
                            .map(|&(_, ref v)| {
                                if let LookAheadValue::Scalar(s) =
                                    LookAheadValue::from_input_value(&v.item, vars)
                                {
                                    <&S as Into<Option<&bool>>>::into(s)
                                        .cloned()
                                        .unwrap_or(false)
                                } else {
                                    false
                                }
                            })
                            .unwrap_or(false),
                        ("skip", &Some(ref a)) => a
                            .item
                            .items
                            .iter()
                            .find(|item| item.0.item == "if")
                            .map(|&(_, ref v)| {
                                if let LookAheadValue::Scalar(b) =
                                    LookAheadValue::from_input_value(&v.item, vars)
                                {
                                    <&S as Into<Option<&bool>>>::into(b)
                                        .map(::std::ops::Not::not)
                                        .unwrap_or(false)
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
        fragments: &'a HashMap<&'a str, &'a Fragment<'a, S>>,
    ) -> Option<LookAheadSelection<'a, S>> {
        Self::build_from_selection_with_parent(s, None, vars, fragments)
    }

    fn build_from_selection_with_parent(
        s: &'a Selection<'a, S>,
        parent: Option<&mut Self>,
        vars: &'a Variables<S>,
        fragments: &'a HashMap<&'a str, &'a Fragment<'a, S>>,
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
                let mut ret = LookAheadSelection {
                    name,
                    alias,
                    arguments,
                    children: Vec::new(),
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
                    p.children.push(ChildSelection {
                        inner: ret,
                        applies_for: Applies::All,
                    });
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
                    if let Some(ref c) = inline.item.type_condition.as_ref().map(|t| t.item) {
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
                    Applies::OnlyType(ref t) if *t == type_name => {
                        Some(c.inner.for_explicit_type(type_name))
                    }
                    Applies::All => Some(c.inner.for_explicit_type(type_name)),
                    Applies::OnlyType(_) => None,
                })
                .collect(),
            name: self.name,
            alias: self.alias,
            arguments: self.arguments.clone(),
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
}

/// A set of common methods for `ConcreteLookAheadSelection` and `LookAheadSelection`
pub trait LookAheadMethods<S> {
    /// Get the name of the field represented by the current selection
    fn field_name(&self) -> &str;

    /// Get the the child selection for a given field
    fn select_child(&self, name: &str) -> Option<&Self>;

    /// Check if a given field exists
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

    /// Get the top level children for the current selection
    fn child_names(&self) -> Vec<&str>;
}

impl<'a, S> LookAheadMethods<S> for ConcreteLookAheadSelection<'a, S> {
    fn field_name(&self) -> &str {
        self.alias.unwrap_or(self.name)
    }

    fn select_child(&self, name: &str) -> Option<&Self> {
        self.children.iter().find(|c| c.name == name)
    }

    fn arguments(&self) -> &[LookAheadArgument<S>] {
        &self.arguments
    }

    fn child_names(&self) -> Vec<&str> {
        self.children
            .iter()
            .map(|c| c.alias.unwrap_or(c.name))
            .collect()
    }

    fn has_arguments(&self) -> bool {
        !self.arguments.is_empty()
    }

    fn has_children(&self) -> bool {
        !self.children.is_empty()
    }
}

impl<'a, S> LookAheadMethods<S> for LookAheadSelection<'a, S> {
    fn field_name(&self) -> &str {
        self.alias.unwrap_or(self.name)
    }

    fn select_child(&self, name: &str) -> Option<&Self> {
        self.children
            .iter()
            .find(|c| c.inner.name == name)
            .map(|s| &s.inner)
    }

    fn arguments(&self) -> &[LookAheadArgument<S>] {
        &self.arguments
    }

    fn child_names(&self) -> Vec<&str> {
        self.children
            .iter()
            .map(|c| c.inner.alias.unwrap_or(c.inner.name))
            .collect()
    }

    fn has_arguments(&self) -> bool {
        !self.arguments.is_empty()
    }

    fn has_children(&self) -> bool {
        !self.children.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ast::Document,
        parser::UnlocatedParseResult,
        schema::model::SchemaType,
        validation::test_harness::{MutationRoot, QueryRoot},
        value::{DefaultScalarValue, ScalarRefValue, ScalarValue},
    };
    use std::collections::HashMap;

    fn parse_document_source<S>(q: &str) -> UnlocatedParseResult<Document<S>>
    where
        S: ScalarValue,
        for<'b> &'b S: ScalarRefValue<'b>,
    {
        crate::parse_document_source(q, &SchemaType::new::<QueryRoot, MutationRoot>(&(), &()))
    }

    fn extract_fragments<'a, S>(doc: &'a Document<S>) -> HashMap<&'a str, &'a Fragment<'a, S>> {
        let mut fragments = HashMap::new();
        for d in doc {
            if let crate::ast::Definition::Fragment(ref f) = *d {
                let f = &f.item;
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
            let vars = Variables::default();
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
                children: vec![
                    ChildSelection {
                        inner: LookAheadSelection {
                            name: "id",
                            alias: None,
                            arguments: Vec::new(),
                            children: Vec::new(),
                        },
                        applies_for: Applies::All,
                    },
                    ChildSelection {
                        inner: LookAheadSelection {
                            name: "name",
                            alias: None,
                            arguments: Vec::new(),
                            children: Vec::new(),
                        },
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
            let vars = Variables::default();
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
                children: vec![
                    ChildSelection {
                        inner: LookAheadSelection {
                            name: "id",
                            alias: None,
                            arguments: Vec::new(),
                            children: Vec::new(),
                        },
                        applies_for: Applies::All,
                    },
                    ChildSelection {
                        inner: LookAheadSelection {
                            name: "name",
                            alias: Some("my_name"),
                            arguments: Vec::new(),
                            children: Vec::new(),
                        },
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
            let vars = Variables::default();
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
                children: vec![
                    ChildSelection {
                        inner: LookAheadSelection {
                            name: "id",
                            alias: None,
                            arguments: Vec::new(),
                            children: Vec::new(),
                        },
                        applies_for: Applies::All,
                    },
                    ChildSelection {
                        inner: LookAheadSelection {
                            name: "name",
                            alias: None,
                            arguments: Vec::new(),
                            children: Vec::new(),
                        },
                        applies_for: Applies::All,
                    },
                    ChildSelection {
                        inner: LookAheadSelection {
                            name: "friends",
                            alias: None,
                            arguments: Vec::new(),
                            children: vec![
                                ChildSelection {
                                    inner: LookAheadSelection {
                                        name: "name",
                                        alias: None,
                                        arguments: Vec::new(),
                                        children: Vec::new(),
                                    },
                                    applies_for: Applies::All,
                                },
                                ChildSelection {
                                    inner: LookAheadSelection {
                                        name: "id",
                                        alias: None,
                                        arguments: Vec::new(),
                                        children: Vec::new(),
                                    },
                                    applies_for: Applies::All,
                                },
                            ],
                        },
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
            let vars = Variables::default();
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
                children: vec![
                    ChildSelection {
                        inner: LookAheadSelection {
                            name: "id",
                            alias: None,
                            arguments: Vec::new(),
                            children: Vec::new(),
                        },
                        applies_for: Applies::All,
                    },
                    ChildSelection {
                        inner: LookAheadSelection {
                            name: "name",
                            alias: None,
                            arguments: vec![LookAheadArgument {
                                name: "uppercase",
                                value: LookAheadValue::Scalar(&DefaultScalarValue::Boolean(true)),
                            }],
                            children: Vec::new(),
                        },
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
            let mut vars = Variables::default();
            vars.insert("episode".into(), InputValue::Enum("JEDI".into()));
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
                children: vec![
                    ChildSelection {
                        inner: LookAheadSelection {
                            name: "id",
                            alias: None,
                            arguments: Vec::new(),
                            children: Vec::new(),
                        },
                        applies_for: Applies::All,
                    },
                    ChildSelection {
                        inner: LookAheadSelection {
                            name: "name",
                            alias: None,
                            arguments: Vec::new(),
                            children: Vec::new(),
                        },
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
            let vars = Variables::default();
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
                children: vec![ChildSelection {
                    inner: LookAheadSelection {
                        name: "id",
                        alias: None,
                        arguments: Vec::new(),
                        children: Vec::new(),
                    },
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
            let vars = Variables::default();
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
                children: vec![
                    ChildSelection {
                        inner: LookAheadSelection {
                            name: "id",
                            alias: None,
                            arguments: Vec::new(),
                            children: Vec::new(),
                        },
                        applies_for: Applies::All,
                    },
                    ChildSelection {
                        inner: LookAheadSelection {
                            name: "name",
                            alias: None,
                            arguments: Vec::new(),
                            children: Vec::new(),
                        },
                        applies_for: Applies::All,
                    },
                    ChildSelection {
                        inner: LookAheadSelection {
                            name: "appearsIn",
                            alias: None,
                            arguments: Vec::new(),
                            children: Vec::new(),
                        },
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
            let vars = Variables::default();
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
                children: vec![
                    ChildSelection {
                        inner: LookAheadSelection {
                            name: "id",
                            alias: None,
                            arguments: Vec::new(),
                            children: Vec::new(),
                        },
                        applies_for: Applies::All,
                    },
                    ChildSelection {
                        inner: LookAheadSelection {
                            name: "height",
                            alias: None,
                            arguments: Vec::new(),
                            children: Vec::new(),
                        },
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
            let vars = Variables::default();
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
                children: vec![
                    ChildSelection {
                        inner: LookAheadSelection {
                            name: "name",
                            alias: None,
                            arguments: Vec::new(),
                            children: Vec::new(),
                        },
                        applies_for: Applies::All,
                    },
                    ChildSelection {
                        inner: LookAheadSelection {
                            name: "primaryFunction",
                            alias: None,
                            arguments: Vec::new(),
                            children: Vec::new(),
                        },
                        applies_for: Applies::OnlyType("Droid"),
                    },
                    ChildSelection {
                        inner: LookAheadSelection {
                            name: "height",
                            alias: None,
                            arguments: Vec::new(),
                            children: Vec::new(),
                        },
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
            let vars = Variables::default();
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
                children: vec![ChildSelection {
                    inner: LookAheadSelection {
                        name: "id",
                        alias: None,
                        arguments: Vec::new(),
                        children: Vec::new(),
                    },
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
                children: vec![ChildSelection {
                    inner: LookAheadSelection {
                        name: "name",
                        alias: None,
                        arguments: Vec::new(),
                        children: Vec::new(),
                    },
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
            let mut vars = Variables::default();
            vars.insert("id".into(), InputValue::Scalar(DefaultScalarValue::Int(42)));
            // This will normally be there
            vars.insert(
                "withFriends".into(),
                InputValue::Scalar(DefaultScalarValue::Boolean(true)),
            );
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
                children: vec![
                    ChildSelection {
                        inner: LookAheadSelection {
                            name: "id",
                            alias: None,
                            arguments: Vec::new(),
                            children: Vec::new(),
                        },
                        applies_for: Applies::All,
                    },
                    ChildSelection {
                        inner: LookAheadSelection {
                            name: "__typename",
                            alias: None,
                            arguments: Vec::new(),
                            children: Vec::new(),
                        },
                        applies_for: Applies::All,
                    },
                    ChildSelection {
                        inner: LookAheadSelection {
                            name: "name",
                            alias: None,
                            arguments: Vec::new(),
                            children: Vec::new(),
                        },
                        applies_for: Applies::All,
                    },
                    ChildSelection {
                        inner: LookAheadSelection {
                            name: "appearsIn",
                            alias: None,
                            arguments: Vec::new(),
                            children: Vec::new(),
                        },
                        applies_for: Applies::All,
                    },
                    ChildSelection {
                        inner: LookAheadSelection {
                            name: "primaryFunction",
                            alias: None,
                            arguments: Vec::new(),
                            children: Vec::new(),
                        },
                        applies_for: Applies::OnlyType("Droid"),
                    },
                    ChildSelection {
                        inner: LookAheadSelection {
                            name: "height",
                            alias: None,
                            arguments: Vec::new(),
                            children: Vec::new(),
                        },
                        applies_for: Applies::OnlyType("Human"),
                    },
                    ChildSelection {
                        inner: LookAheadSelection {
                            name: "friends",
                            alias: None,
                            arguments: Vec::new(),
                            children: vec![
                                ChildSelection {
                                    inner: LookAheadSelection {
                                        name: "__typename",
                                        alias: None,
                                        arguments: Vec::new(),
                                        children: Vec::new(),
                                    },
                                    applies_for: Applies::All,
                                },
                                ChildSelection {
                                    inner: LookAheadSelection {
                                        name: "name",
                                        alias: None,
                                        arguments: Vec::new(),
                                        children: Vec::new(),
                                    },
                                    applies_for: Applies::All,
                                },
                                ChildSelection {
                                    inner: LookAheadSelection {
                                        name: "appearsIn",
                                        alias: None,
                                        arguments: Vec::new(),
                                        children: Vec::new(),
                                    },
                                    applies_for: Applies::All,
                                },
                                ChildSelection {
                                    inner: LookAheadSelection {
                                        name: "primaryFunction",
                                        alias: None,
                                        arguments: Vec::new(),
                                        children: Vec::new(),
                                    },
                                    applies_for: Applies::OnlyType("Droid"),
                                },
                                ChildSelection {
                                    inner: LookAheadSelection {
                                        name: "height",
                                        alias: None,
                                        arguments: Vec::new(),
                                        children: Vec::new(),
                                    },
                                    applies_for: Applies::OnlyType("Human"),
                                },
                            ],
                        },
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
            let vars = Variables::default();
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
                children: vec![
                    ConcreteLookAheadSelection {
                        name: "name",
                        alias: None,
                        arguments: Vec::new(),
                        children: Vec::new(),
                    },
                    ConcreteLookAheadSelection {
                        name: "height",
                        alias: None,
                        arguments: Vec::new(),
                        children: Vec::new(),
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
            children: vec![
                ChildSelection {
                    inner: LookAheadSelection {
                        name: "id",
                        alias: None,
                        arguments: Vec::new(),
                        children: Vec::new(),
                    },
                    applies_for: Applies::All,
                },
                ChildSelection {
                    inner: LookAheadSelection {
                        name: "friends",
                        alias: None,
                        arguments: Vec::new(),
                        children: vec![
                            ChildSelection {
                                inner: LookAheadSelection {
                                    name: "id",
                                    alias: None,
                                    arguments: Vec::new(),
                                    children: Vec::new(),
                                },
                                applies_for: Applies::All,
                            },
                            ChildSelection {
                                inner: LookAheadSelection {
                                    name: "name",
                                    alias: None,
                                    arguments: Vec::new(),
                                    children: Vec::new(),
                                },
                                applies_for: Applies::All,
                            },
                        ],
                    },
                    applies_for: Applies::All,
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
            children: vec![
                ChildSelection {
                    inner: LookAheadSelection {
                        name: "id",
                        alias: None,
                        arguments: Vec::new(),
                        children: Vec::new(),
                    },
                    applies_for: Applies::All,
                },
                ChildSelection {
                    inner: LookAheadSelection {
                        name: "name",
                        alias: None,
                        arguments: Vec::new(),
                        children: Vec::new(),
                    },
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
            let vars = Variables::default();
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
                children: vec![ChildSelection {
                    inner: LookAheadSelection {
                        name: "friends",
                        alias: None,
                        arguments: Vec::new(),
                        children: vec![ChildSelection {
                            inner: LookAheadSelection {
                                name: "name",
                                alias: None,
                                arguments: Vec::new(),
                                children: Vec::new(),
                            },
                            applies_for: Applies::All,
                        }],
                    },
                    applies_for: Applies::All,
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
            let vars = Variables::default();
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
            assert_eq!(look_ahead.child_names(), vec!["name", "friends"]);

            let child0 = look_ahead.select_child("name").unwrap();
            assert_eq!(child0.field_name(), "name");
            assert!(!child0.has_arguments());
            assert!(!child0.has_children());

            let child1 = look_ahead.select_child("friends").unwrap();
            assert_eq!(child1.field_name(), "friends");
            assert!(!child1.has_arguments());
            assert!(child1.has_children());
            assert_eq!(child1.child_names(), vec!["name"]);

            let child2 = child1.select_child("name").unwrap();
            assert!(!child2.has_arguments());
            assert!(!child2.has_children());
        } else {
            panic!("No Operation found");
        }
    }
}
