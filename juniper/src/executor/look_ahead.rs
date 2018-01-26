use ast::{Directive, Fragment, InputValue, Selection};
use parser::Spanning;

use std::collections::HashMap;

use super::Variables;

#[derive(Debug, Clone, PartialEq)]
pub enum Applies<'a> {
    All,
    OnlyType(&'a str),
}

#[derive(Debug, Clone, PartialEq)]
pub enum LookAheadValue<'a> {
    Null,
    Int(i32),
    Float(f64),
    String(&'a str),
    Boolean(bool),
    // TODO: improve
    Enum(&'a str),
    List(Vec<LookAheadValue<'a>>),
    Object(Vec<(&'a str, LookAheadValue<'a>)>),
}

impl<'a> LookAheadValue<'a> {
    fn from_input_value(input_value: &'a InputValue, vars: &'a Variables) -> Self {
        match *input_value {
            InputValue::Null => LookAheadValue::Null,
            InputValue::Int(i) => LookAheadValue::Int(i),
            InputValue::Float(f) => LookAheadValue::Float(f),
            InputValue::String(ref s) => LookAheadValue::String(s),
            InputValue::Boolean(b) => LookAheadValue::Boolean(b),
            InputValue::Enum(ref e) => LookAheadValue::Enum(e),
            InputValue::Variable(ref v) => Self::from_input_value(vars.get(v).unwrap(), vars),
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

#[derive(Debug, Clone, PartialEq)]
pub struct LookAheadArgument<'a> {
    name: &'a str,
    value: LookAheadValue<'a>,
}

impl<'a> LookAheadArgument<'a> {
    fn new(
        &(ref name, ref value): &'a (Spanning<&'a str>, Spanning<InputValue>),
        vars: &'a Variables,
    ) -> Self {
        LookAheadArgument {
            name: name.item,
            value: LookAheadValue::from_input_value(&value.item, vars),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChildSelection<'a> {
    pub(super) inner: LookAheadSelection<'a>,
    pub(super) applies_for: Applies<'a>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LookAheadSelection<'a> {
    pub(super) name: &'a str,
    pub(super) alias: Option<&'a str>,
    pub(super) arguments: Vec<LookAheadArgument<'a>>,
    pub(super) childs: Vec<ChildSelection<'a>>,
}

impl<'a> LookAheadSelection<'a> {
    fn should_include(directives: Option<&Vec<Spanning<Directive>>>, vars: &Variables) -> bool {
        directives
            .map(|d| {
                d.iter().all(|d| {
                    let d = &d.item;
                    let arguments = &d.arguments;
                    match (d.name.item, arguments) {
                        ("include", &Some(ref a)) => a.item
                            .items
                            .iter()
                            .find(|item| item.0.item == "if")
                            .map(|&(_, ref v)| {
                                if let LookAheadValue::Boolean(b) =
                                    LookAheadValue::from_input_value(&v.item, vars)
                                {
                                    b
                                } else {
                                    false
                                }
                            })
                            .unwrap_or(false),
                        ("skip", &Some(ref a)) => a.item
                            .items
                            .iter()
                            .find(|item| item.0.item == "if")
                            .map(|&(_, ref v)| {
                                if let LookAheadValue::Boolean(b) =
                                    LookAheadValue::from_input_value(&v.item, vars)
                                {
                                    !b
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
        s: &'a Selection<'a>,
        vars: &'a Variables,
        fragments: &'a HashMap<&'a str, &'a Fragment<'a>>,
    ) -> LookAheadSelection<'a> {
        Self::build_from_selection_with_parent(s, None, vars, fragments).unwrap()
    }

    fn build_from_selection_with_parent(
        s: &'a Selection<'a>,
        parent: Option<&mut Self>,
        vars: &'a Variables,
        fragments: &'a HashMap<&'a str, &'a Fragment<'a>>,
    ) -> Option<LookAheadSelection<'a>> {
        let empty: &[Selection] = &[];
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
                    childs: Vec::new(),
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
                    p.childs.push(ChildSelection {
                        inner: ret,
                        applies_for: Applies::All,
                    });
                    None
                } else {
                    Some(ret)
                }
            }
            Selection::FragmentSpread(ref fragment) if parent.is_some() => {
                let include = Self::should_include(fragment.item.directives.as_ref(), vars);
                if !include {
                    return None;
                }
                let parent = parent.unwrap();
                let f = fragments.get(&fragment.item.name.item).unwrap();
                for c in f.selection_set.iter() {
                    let s = LookAheadSelection::build_from_selection_with_parent(
                        c,
                        Some(parent),
                        vars,
                        fragments,
                    );
                    assert!(s.is_none());
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
                        if let Some(p) = parent.childs.last_mut() {
                            p.applies_for = Applies::OnlyType(c);
                        }
                    }
                }
                None
            }
            _ => unimplemented!(),
        }
    }

    pub fn for_explicit_type(&self, type_name: &str) -> ConcreteLookAheadSelection<'a> {
        ConcreteLookAheadSelection {
            childs: self.childs
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

#[derive(Debug, PartialEq)]
pub struct ConcreteLookAheadSelection<'a> {
    name: &'a str,
    alias: Option<&'a str>,
    arguments: Vec<LookAheadArgument<'a>>,
    childs: Vec<ConcreteLookAheadSelection<'a>>,
}

pub trait LookAheadMethods {
    fn field_name(&self) -> &str;

    fn select_child(&self, name: &str) -> Option<&Self>;

    fn has_child(&self, name: &str) -> bool {
        self.select_child(name).is_some()
    }
}

impl<'a> LookAheadMethods for ConcreteLookAheadSelection<'a> {
    fn field_name(&self) -> &str {
        self.alias.unwrap_or(self.name)
    }

    fn select_child(&self, name: &str) -> Option<&Self> {
       self.childs.iter().find(|c| c.name == name)
    }
}

impl<'a> LookAheadMethods for LookAheadSelection<'a> {
    fn field_name(&self) -> &str {
        self.alias.unwrap_or(self.name)
    }

    fn select_child(&self, name: &str) -> Option<&Self> {
       self.childs.iter().find(|c| c.inner.name == name).map(|s| &s.inner)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use super::*;
    use ast::Document;

    fn extract_fragments<'a>(doc: &'a Document) -> HashMap<&'a str, &'a Fragment<'a>> {
        let mut fragments = HashMap::new();
        for d in doc {
            if let ::ast::Definition::Fragment(ref f) = *d {
                let f = &f.item;
                fragments.insert(f.name.item, f);
            }
        }
        fragments
    }

    #[test]
    fn check_simple_query() {
        let docs = ::parse_document_source(
            "
query Hero {
    hero {
        id
        name
    }
}
",
        ).unwrap();
        let fragments = extract_fragments(&docs);

        if let ::ast::Definition::Operation(ref op) = docs[0] {
            let vars = Variables::default();
            let look_ahead = LookAheadSelection::build_from_selection(
                &op.item.selection_set[0],
                &vars,
                &fragments,
            );
            let expected = LookAheadSelection {
                name: "hero",
                alias: None,
                arguments: Vec::new(),
                childs: vec![
                    ChildSelection {
                        inner: LookAheadSelection {
                            name: "id",
                            alias: None,
                            arguments: Vec::new(),
                            childs: Vec::new(),
                        },
                        applies_for: Applies::All,
                    },
                    ChildSelection {
                        inner: LookAheadSelection {
                            name: "name",
                            alias: None,
                            arguments: Vec::new(),
                            childs: Vec::new(),
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
        let docs = ::parse_document_source(
            "
query Hero {
    custom_hero: hero {
        id
        my_name: name
    }
}
",
        ).unwrap();
        let fragments = extract_fragments(&docs);

        if let ::ast::Definition::Operation(ref op) = docs[0] {
            let vars = Variables::default();
            let look_ahead = LookAheadSelection::build_from_selection(
                &op.item.selection_set[0],
                &vars,
                &fragments,
            );
            let expected = LookAheadSelection {
                name: "hero",
                alias: Some("custom_hero"),
                arguments: Vec::new(),
                childs: vec![
                    ChildSelection {
                        inner: LookAheadSelection {
                            name: "id",
                            alias: None,
                            arguments: Vec::new(),
                            childs: Vec::new(),
                        },
                        applies_for: Applies::All,
                    },
                    ChildSelection {
                        inner: LookAheadSelection {
                            name: "name",
                            alias: Some("my_name"),
                            arguments: Vec::new(),
                            childs: Vec::new(),
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
        let docs = ::parse_document_source(
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
        ).unwrap();
        let fragments = extract_fragments(&docs);

        if let ::ast::Definition::Operation(ref op) = docs[0] {
            let vars = Variables::default();
            let look_ahead = LookAheadSelection::build_from_selection(
                &op.item.selection_set[0],
                &vars,
                &fragments,
            );
            let expected = LookAheadSelection {
                name: "hero",
                alias: None,
                arguments: Vec::new(),
                childs: vec![
                    ChildSelection {
                        inner: LookAheadSelection {
                            name: "id",
                            alias: None,
                            arguments: Vec::new(),
                            childs: Vec::new(),
                        },
                        applies_for: Applies::All,
                    },
                    ChildSelection {
                        inner: LookAheadSelection {
                            name: "name",
                            alias: None,
                            arguments: Vec::new(),
                            childs: Vec::new(),
                        },
                        applies_for: Applies::All,
                    },
                    ChildSelection {
                        inner: LookAheadSelection {
                            name: "friends",
                            alias: None,
                            arguments: Vec::new(),
                            childs: vec![
                                ChildSelection {
                                    inner: LookAheadSelection {
                                        name: "name",
                                        alias: None,
                                        arguments: Vec::new(),
                                        childs: Vec::new(),
                                    },
                                    applies_for: Applies::All,
                                },
                                ChildSelection {
                                    inner: LookAheadSelection {
                                        name: "id",
                                        alias: None,
                                        arguments: Vec::new(),
                                        childs: Vec::new(),
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
        let docs = ::parse_document_source(
            "
query Hero {
    hero(episode: EMPIRE) {
        id
        name(uppercase: true)
    }
}
",
        ).unwrap();
        let fragments = extract_fragments(&docs);

        if let ::ast::Definition::Operation(ref op) = docs[0] {
            let vars = Variables::default();
            let look_ahead = LookAheadSelection::build_from_selection(
                &op.item.selection_set[0],
                &vars,
                &fragments,
            );
            let expected = LookAheadSelection {
                name: "hero",
                alias: None,
                arguments: vec![
                    LookAheadArgument {
                        name: "episode",
                        value: LookAheadValue::Enum("EMPIRE"),
                    },
                ],
                childs: vec![
                    ChildSelection {
                        inner: LookAheadSelection {
                            name: "id",
                            alias: None,
                            arguments: Vec::new(),
                            childs: Vec::new(),
                        },
                        applies_for: Applies::All,
                    },
                    ChildSelection {
                        inner: LookAheadSelection {
                            name: "name",
                            alias: None,
                            arguments: vec![
                                LookAheadArgument {
                                    name: "uppercase",
                                    value: LookAheadValue::Boolean(true),
                                },
                            ],
                            childs: Vec::new(),
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
        let docs = ::parse_document_source(
            "
query Hero($episode: Episode) {
    hero(episode: $episode) {
        id
        name
    }
}
",
        ).unwrap();
        let fragments = extract_fragments(&docs);

        if let ::ast::Definition::Operation(ref op) = docs[0] {
            let mut vars = Variables::default();
            vars.insert("episode".into(), InputValue::Enum("JEDI".into()));
            let look_ahead = LookAheadSelection::build_from_selection(
                &op.item.selection_set[0],
                &vars,
                &fragments,
            );
            let expected = LookAheadSelection {
                name: "hero",
                alias: None,
                arguments: vec![
                    LookAheadArgument {
                        name: "episode",
                        value: LookAheadValue::Enum("JEDI"),
                    },
                ],
                childs: vec![
                    ChildSelection {
                        inner: LookAheadSelection {
                            name: "id",
                            alias: None,
                            arguments: Vec::new(),
                            childs: Vec::new(),
                        },
                        applies_for: Applies::All,
                    },
                    ChildSelection {
                        inner: LookAheadSelection {
                            name: "name",
                            alias: None,
                            arguments: Vec::new(),
                            childs: Vec::new(),
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
    fn check_query_with_fragment() {
        let docs = ::parse_document_source(
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
        ).unwrap();
        let fragments = extract_fragments(&docs);

        if let ::ast::Definition::Operation(ref op) = docs[0] {
            let vars = Variables::default();
            let look_ahead = LookAheadSelection::build_from_selection(
                &op.item.selection_set[0],
                &vars,
                &fragments,
            );
            let expected = LookAheadSelection {
                name: "hero",
                alias: None,
                arguments: Vec::new(),
                childs: vec![
                    ChildSelection {
                        inner: LookAheadSelection {
                            name: "id",
                            alias: None,
                            arguments: Vec::new(),
                            childs: Vec::new(),
                        },
                        applies_for: Applies::All,
                    },
                    ChildSelection {
                        inner: LookAheadSelection {
                            name: "name",
                            alias: None,
                            arguments: Vec::new(),
                            childs: Vec::new(),
                        },
                        applies_for: Applies::All,
                    },
                    ChildSelection {
                        inner: LookAheadSelection {
                            name: "appearsIn",
                            alias: None,
                            arguments: Vec::new(),
                            childs: Vec::new(),
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
        let docs = ::parse_document_source(
            "
query Hero {
    hero {
        id @include(if: true)
        name @include(if: false)
        appearsIn @skip(if: true)
        height @skip(if: false)
    }
}",
        ).unwrap();
        let fragments = extract_fragments(&docs);

        if let ::ast::Definition::Operation(ref op) = docs[0] {
            let vars = Variables::default();
            let look_ahead = LookAheadSelection::build_from_selection(
                &op.item.selection_set[0],
                &vars,
                &fragments,
            );
            let expected = LookAheadSelection {
                name: "hero",
                alias: None,
                arguments: Vec::new(),
                childs: vec![
                    ChildSelection {
                        inner: LookAheadSelection {
                            name: "id",
                            alias: None,
                            arguments: Vec::new(),
                            childs: Vec::new(),
                        },
                        applies_for: Applies::All,
                    },
                    ChildSelection {
                        inner: LookAheadSelection {
                            name: "height",
                            alias: None,
                            arguments: Vec::new(),
                            childs: Vec::new(),
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
        let docs = ::parse_document_source(
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
        ).unwrap();
        let fragments = extract_fragments(&docs);

        if let ::ast::Definition::Operation(ref op) = docs[0] {
            let vars = Variables::default();
            let look_ahead = LookAheadSelection::build_from_selection(
                &op.item.selection_set[0],
                &vars,
                &fragments,
            );
            let expected = LookAheadSelection {
                name: "hero",
                alias: None,
                arguments: Vec::new(),
                childs: vec![
                    ChildSelection {
                        inner: LookAheadSelection {
                            name: "name",
                            alias: None,
                            arguments: Vec::new(),
                            childs: Vec::new(),
                        },
                        applies_for: Applies::All,
                    },
                    ChildSelection {
                        inner: LookAheadSelection {
                            name: "primaryFunction",
                            alias: None,
                            arguments: Vec::new(),
                            childs: Vec::new(),
                        },
                        applies_for: Applies::OnlyType("Droid"),
                    },
                    ChildSelection {
                        inner: LookAheadSelection {
                            name: "height",
                            alias: None,
                            arguments: Vec::new(),
                            childs: Vec::new(),
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
    fn check_complex_query() {
        let docs = ::parse_document_source(
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
        ).unwrap();
        let fragments = extract_fragments(&docs);

        if let ::ast::Definition::Operation(ref op) = docs[0] {
            let mut vars = Variables::default();
            vars.insert("id".into(), InputValue::Int(42));
            // This will normally be there
            vars.insert("withFriends".into(), InputValue::Boolean(true));
            let look_ahead = LookAheadSelection::build_from_selection(
                &op.item.selection_set[0],
                &vars,
                &fragments,
            );
            let expected = LookAheadSelection {
                name: "hero",
                alias: None,
                arguments: vec![
                    LookAheadArgument {
                        name: "id",
                        value: LookAheadValue::Int(42),
                    },
                ],
                childs: vec![
                    ChildSelection {
                        inner: LookAheadSelection {
                            name: "id",
                            alias: None,
                            arguments: Vec::new(),
                            childs: Vec::new(),
                        },
                        applies_for: Applies::All,
                    },
                    ChildSelection {
                        inner: LookAheadSelection {
                            name: "__typename",
                            alias: None,
                            arguments: Vec::new(),
                            childs: Vec::new(),
                        },
                        applies_for: Applies::All,
                    },
                    ChildSelection {
                        inner: LookAheadSelection {
                            name: "name",
                            alias: None,
                            arguments: Vec::new(),
                            childs: Vec::new(),
                        },
                        applies_for: Applies::All,
                    },
                    ChildSelection {
                        inner: LookAheadSelection {
                            name: "appearsIn",
                            alias: None,
                            arguments: Vec::new(),
                            childs: Vec::new(),
                        },
                        applies_for: Applies::All,
                    },
                    ChildSelection {
                        inner: LookAheadSelection {
                            name: "primaryFunction",
                            alias: None,
                            arguments: Vec::new(),
                            childs: Vec::new(),
                        },
                        applies_for: Applies::OnlyType("Droid"),
                    },
                    ChildSelection {
                        inner: LookAheadSelection {
                            name: "height",
                            alias: None,
                            arguments: Vec::new(),
                            childs: Vec::new(),
                        },
                        applies_for: Applies::OnlyType("Human"),
                    },
                    ChildSelection {
                        inner: LookAheadSelection {
                            name: "friends",
                            alias: None,
                            arguments: Vec::new(),
                            childs: vec![
                                ChildSelection {
                                    inner: LookAheadSelection {
                                        name: "__typename",
                                        alias: None,
                                        arguments: Vec::new(),
                                        childs: Vec::new(),
                                    },
                                    applies_for: Applies::All,
                                },
                                ChildSelection {
                                    inner: LookAheadSelection {
                                        name: "name",
                                        alias: None,
                                        arguments: Vec::new(),
                                        childs: Vec::new(),
                                    },
                                    applies_for: Applies::All,
                                },
                                ChildSelection {
                                    inner: LookAheadSelection {
                                        name: "appearsIn",
                                        alias: None,
                                        arguments: Vec::new(),
                                        childs: Vec::new(),
                                    },
                                    applies_for: Applies::All,
                                },
                                ChildSelection {
                                    inner: LookAheadSelection {
                                        name: "primaryFunction",
                                        alias: None,
                                        arguments: Vec::new(),
                                        childs: Vec::new(),
                                    },
                                    applies_for: Applies::OnlyType("Droid"),
                                },
                                ChildSelection {
                                    inner: LookAheadSelection {
                                        name: "height",
                                        alias: None,
                                        arguments: Vec::new(),
                                        childs: Vec::new(),
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
        let docs = ::parse_document_source(
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
        ).unwrap();
        let fragments = extract_fragments(&docs);

        if let ::ast::Definition::Operation(ref op) = docs[0] {
            let vars = Variables::default();
            let look_ahead = LookAheadSelection::build_from_selection(
                &op.item.selection_set[0],
                &vars,
                &fragments,
            ).for_explicit_type("Human");
            let expected = ConcreteLookAheadSelection {
                name: "hero",
                alias: None,
                arguments: Vec::new(),
                childs: vec![
                    ConcreteLookAheadSelection {
                        name: "name",
                        alias: None,
                        arguments: Vec::new(),
                        childs: Vec::new(),
                    },
                    ConcreteLookAheadSelection {
                        name: "height",
                        alias: None,
                        arguments: Vec::new(),
                        childs: Vec::new(),
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
        let lookahead = LookAheadSelection{
            name: "hero",
            alias: None,
            arguments: Vec::new(),
            childs: vec![
                ChildSelection{
                    inner: LookAheadSelection {
                        name: "id",
                        alias: None,
                        arguments: Vec::new(),
                        childs: Vec::new(),
                    },
                    applies_for: Applies::All,
                },
                ChildSelection {
                    inner: LookAheadSelection {
                        name: "friends",
                        alias: None,
                        arguments: Vec::new(),
                        childs: vec![
                            ChildSelection {
                                inner: LookAheadSelection {
                                    name: "id",
                                    alias: None,
                                    arguments: Vec::new(),
                                    childs: Vec::new(),
                                },
                                applies_for: Applies::All
                            },
                            ChildSelection {
                                inner: LookAheadSelection {
                                    name: "name",
                                    alias: None,
                                    arguments: Vec::new(),
                                    childs: Vec::new(),
                                },
                                applies_for: Applies::All
                            }
                        ],
                    },
                    applies_for: Applies::All
                }
            ]
        };
        let concret_query = lookahead.for_explicit_type("does not matter");

        let id = lookahead.select_child("id");
        let concrete_id = concret_query.select_child("id");
        let expected = LookAheadSelection{name: "id", alias: None, arguments: Vec::new(), childs: Vec::new()};
        assert_eq!(id, Some(&expected));
        assert_eq!(concrete_id, Some(&expected.for_explicit_type("does not matter")));

        let friends = lookahead.select_child("friends");
        let concrete_friends = concret_query.select_child("friends");
        let expected =  LookAheadSelection {
                        name: "friends",
                        alias: None,
                        arguments: Vec::new(),
                        childs: vec![
                            ChildSelection {
                                inner: LookAheadSelection {
                                    name: "id",
                                    alias: None,
                                    arguments: Vec::new(),
                                    childs: Vec::new(),
                                },
                                applies_for: Applies::All
                            },
                            ChildSelection {
                                inner: LookAheadSelection {
                                    name: "name",
                                    alias: None,
                                    arguments: Vec::new(),
                                    childs: Vec::new(),
                                },
                                applies_for: Applies::All
                            }
                        ],
                    };
        assert_eq!(friends, Some(&expected));
        assert_eq!(concrete_friends, Some(&expected.for_explicit_type("does not matter")));
    }

}
