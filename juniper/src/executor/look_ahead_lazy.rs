use std::collections::HashMap;

use crate::{
    ast::{Arguments, Directive, Field, Fragment, InputValue, Selection},
    parser::{Span, Spanning},
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

/// Shortcut for a [`Spanning`] containing a borrowed [`Span`].
type BorrowedSpanning<'a, T> = Spanning<T, &'a Span>;

/// JSON-like value that can be used as an argument in the query execution.
///
/// In contrast to an [`InputValue`], these values do only contain constants,
/// meaning that variables get automatically resolved.
#[derive(Clone, Copy, Debug, PartialEq)]
#[allow(missing_docs)]
pub enum LookAheadValue<'a, S: ScalarValue + 'a> {
    Null,
    Scalar(&'a S),
    Enum(&'a str),
    List(LookAheadList<'a, S>),
    Object(LookAheadObject<'a, S>),
}

impl<'a, S: ScalarValue + 'a> LookAheadValue<'a, S> {
    fn from_input_value(
        input_value: &'a InputValue<S>,
        span: &'a Span,
        vars: Option<&'a Variables<S>>,
    ) -> BorrowedSpanning<'a, Self> {
        Spanning {
            item: match input_value {
                InputValue::Null => Self::Null,
                InputValue::Scalar(s) => Self::Scalar(s),
                InputValue::Enum(e) => Self::Enum(e),
                InputValue::Variable(name) => vars
                    .and_then(|vars| vars.get(name))
                    .map(|item| Self::from_input_value(item, span, vars).item)
                    .unwrap_or(Self::Null),
                InputValue::List(input_list) => Self::List(LookAheadList { input_list, vars }),
                InputValue::Object(input_object) => Self::Object(LookAheadObject {
                    input_object: input_object.as_slice(),
                    vars,
                }),
            },
            span,
        }
    }
}

/// A JSON-like list that can be used as an argument in the query execution.
#[derive(Clone, Copy, Debug)]
pub struct LookAheadList<'a, S> {
    input_list: &'a [Spanning<InputValue<S>>],
    vars: Option<&'a Variables<S>>,
}

impl<'a, S: ScalarValue> LookAheadList<'a, S> {
    /// Returns an iterator over the list's elements.
    pub fn iter(&self) -> impl Iterator<Item = BorrowedSpanning<'a, LookAheadValue<'a, S>>> + '_ {
        self.input_list
            .iter()
            .map(|val| LookAheadValue::from_input_value(&val.item, &val.span, self.vars))
    }
}

impl<'a, S> Default for LookAheadList<'a, S> {
    fn default() -> Self {
        Self {
            input_list: &[],
            vars: None,
        }
    }
}

impl<'a, S: ScalarValue> PartialEq for LookAheadList<'a, S> {
    fn eq(&self, other: &Self) -> bool {
        self.iter().eq(other.iter())
    }
}

/// A JSON-like object that can be used as an argument in the query execution.
#[derive(Clone, Copy, Debug)]
pub struct LookAheadObject<'a, S> {
    input_object: &'a [(Spanning<String>, Spanning<InputValue<S>>)],
    vars: Option<&'a Variables<S>>,
}

impl<'a, S: ScalarValue + 'a> LookAheadObject<'a, S> {
    /// Returns an iterator over the object's entries.
    pub fn iter(
        &self,
    ) -> impl Iterator<
        Item = (
            BorrowedSpanning<'a, &'a str>,
            BorrowedSpanning<'a, LookAheadValue<'a, S>>,
        ),
    > + '_ {
        self.input_object.iter().map(|(key, val)| {
            (
                Spanning {
                    span: &key.span,
                    item: key.item.as_str(),
                },
                LookAheadValue::from_input_value(&val.item, &val.span, self.vars),
            )
        })
    }
}

impl<'a, S> Default for LookAheadObject<'a, S> {
    fn default() -> Self {
        Self {
            input_object: &[],
            vars: None,
        }
    }
}

impl<'a, S: ScalarValue> PartialEq for LookAheadObject<'a, S> {
    fn eq(&self, other: &Self) -> bool {
        self.iter().eq(other.iter())
    }
}

/// An argument passed into the query
#[derive(Clone, Copy, Debug)]
pub struct LookAheadArgument<'a, S> {
    name: &'a Spanning<&'a str>,
    input_value: &'a Spanning<InputValue<S>>,
    vars: &'a Variables<S>,
}

impl<'a, S: ScalarValue> LookAheadArgument<'a, S> {
    /// The argument's name
    pub fn name(&self) -> &'a str {
        self.name.item
    }

    /// The Span of the argument's name
    pub fn name_span(&self) -> &'a Span {
        &self.name.span
    }

    /// The argument's value
    pub fn value(&self) -> LookAheadValue<'a, S> {
        LookAheadValue::from_input_value(
            &self.input_value.item,
            &self.input_value.span,
            Some(self.vars),
        )
        .item
    }

    /// The Span of the argument's value
    pub fn value_span(&self) -> &'a Span {
        &self.input_value.span
    }
}

/// The arguments passed into a query.
#[derive(Copy, Clone, Debug)]
pub struct LookAheadArguments<'a, S> {
    arguments: &'a Arguments<'a, S>,
    vars: &'a Variables<S>,
}

impl<'a, S> LookAheadArguments<'a, S> {
    /// Returns an iterator over the arguments.
    pub fn iter(&self) -> impl Iterator<Item = LookAheadArgument<'a, S>> + '_ {
        self.arguments
            .items
            .iter()
            .map(|(name, input_value)| LookAheadArgument {
                name,
                input_value,
                vars: self.vars,
            })
    }
}

/// The children of a selection.
#[derive(Clone, Debug)]
pub struct LookAheadChildren<'a, S: ScalarValue + 'a> {
    children: Vec<LookAheadSelection<'a, S>>,
}

impl<'a, S: ScalarValue> Default for LookAheadChildren<'a, S> {
    fn default() -> Self {
        Self { children: vec![] }
    }
}

impl<'a, S: ScalarValue> LookAheadChildren<'a, S> {
    /// Returns the number children.
    pub fn len(&self) -> usize {
        self.children.len()
    }

    /// Indicates whether the current node has any children.
    pub fn is_empty(&self) -> bool {
        self.children.is_empty()
    }

    /// Returns the child selection for the specified field.
    ///
    /// If a child has an alias, it will only match if the alias matches the specified `name`.
    pub fn select(&self, name: &str) -> Option<LookAheadSelection<'a, S>> {
        self.children
            .iter()
            .find(|child| child.field_name() == name)
            .cloned()
    }

    /// Checks if a child selection with the specified `name` exists.
    ///
    /// If a child has an alias, it will only match if the alias matches the specified `name`.
    pub fn has_child(&self, name: &str) -> bool {
        self.select(name).is_some()
    }

    /// Returns the (possibly aliased) names of the top level children from the current selection.
    pub fn names(&self) -> impl Iterator<Item = &'a str> + '_ {
        self.children.iter().map(|selection| selection.field_name())
    }

    /// Iterate over the children, by reference.
    pub fn iter(&self) -> impl Iterator<Item = &LookAheadSelection<'a, S>> + '_ {
        self.children.iter()
    }
}

impl<'a, S: ScalarValue> IntoIterator for LookAheadChildren<'a, S> {
    type Item = LookAheadSelection<'a, S>;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.children.into_iter()
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) enum SelectionSource<'a, S: ScalarValue> {
    Field(&'a Field<'a, S>),
    Spread {
        field_name: &'a str,
        set: Option<&'a [Selection<'a, S>]>,
    },
}

/// A selection performed by a query
#[derive(Clone, Copy, Debug)]
pub struct LookAheadSelection<'a, S: ScalarValue + 'a> {
    source: SelectionSource<'a, S>,
    applies_for: Applies<'a>,
    vars: &'a Variables<S>,
    fragments: &'a HashMap<&'a str, Fragment<'a, S>>,
}

impl<'a, S: ScalarValue> LookAheadSelection<'a, S> {
    pub(super) fn new(
        source: SelectionSource<'a, S>,
        vars: &'a Variables<S>,
        fragments: &'a HashMap<&'a str, Fragment<'a, S>>,
    ) -> Self {
        Self {
            source,
            applies_for: Applies::All,
            vars,
            fragments,
        }
    }

    /// Returns the original name of the field, represented by the current selection.
    pub fn field_original_name(&self) -> &'a str {
        match self.source {
            SelectionSource::Field(field) => field.name.item,
            SelectionSource::Spread { field_name, .. } => field_name,
        }
    }

    /// Returns the alias of the field, represented by the current selection, if any.
    pub fn field_alias(&self) -> Option<&'a str> {
        match self.source {
            SelectionSource::Field(field) => field.alias.map(|alias| alias.item),
            SelectionSource::Spread { .. } => None,
        }
    }

    /// Returns the (potentially aliased) name of the field, represented by the current selection.
    pub fn field_name(&self) -> &'a str {
        self.field_alias()
            .unwrap_or_else(|| self.field_original_name())
    }

    /// Returns the top level arguments from the current selection.
    pub fn arguments(&self) -> Option<LookAheadArguments<'a, S>> {
        match self.source {
            SelectionSource::Field(field) => {
                field
                    .arguments
                    .as_ref()
                    .map(|spanned_arguments| LookAheadArguments {
                        arguments: &spanned_arguments.item,
                        vars: self.vars,
                    })
            }
            _ => None,
        }
    }

    /// Returns the children from the current selection.
    pub fn children(&self) -> LookAheadChildren<'a, S> {
        self.build_children(Applies::All)
    }

    /// Returns the children from the current selection that only applies to a specific type.
    pub fn children_for_explicit_type(&self, type_name: &str) -> LookAheadChildren<'a, S> {
        self.build_children(Applies::OnlyType(type_name))
    }

    fn build_children(&self, type_filter: Applies) -> LookAheadChildren<'a, S> {
        let mut builder = ChildrenBuilder {
            vars: self.vars,
            fragments: self.fragments,
            type_filter,
            output: vec![],
        };
        match &self.source {
            SelectionSource::Field(field) => {
                builder.visit_field_children(field, Applies::All);
            }
            SelectionSource::Spread {
                set: Some(selections),
                ..
            } => {
                for selection in selections.iter() {
                    builder.visit_selection_children(selection, Applies::All);
                }
            }
            SelectionSource::Spread { set: None, .. } => {}
        }
        LookAheadChildren {
            children: builder.output,
        }
    }

    /// Returns the parent type, in case there is any for the current selection.
    pub fn applies_for(&self) -> Option<&str> {
        match self.applies_for {
            Applies::OnlyType(typ) => Some(typ),
            Applies::All => None,
        }
    }
}

struct ChildrenBuilder<'a, 'f, S: ScalarValue> {
    vars: &'a Variables<S>,
    fragments: &'a HashMap<&'a str, Fragment<'a, S>>,
    type_filter: Applies<'f>,
    output: Vec<LookAheadSelection<'a, S>>,
}

impl<'a, 'f, S: ScalarValue> ChildrenBuilder<'a, 'f, S> {
    /// Add the children of the given field
    fn visit_field_children(&mut self, field: &'a Field<'a, S>, applies_for: Applies<'a>) {
        if let Some(selection_set) = &field.selection_set {
            for child in selection_set {
                self.visit_child(child, applies_for);
            }
        }
    }

    /// Add the children of a given selection
    fn visit_selection_children(
        &mut self,
        selection: &'a Selection<'a, S>,
        applies_for: Applies<'a>,
    ) {
        match selection {
            Selection::Field(field) => {
                self.visit_field_children(&field.item, applies_for);
            }
            Selection::FragmentSpread(fragment) => {
                let f = self
                    .fragments
                    .get(&fragment.item.name.item)
                    .expect("a fragment");
                for c in f.selection_set.iter() {
                    self.visit_selection_children(c, applies_for);
                }
            }
            Selection::InlineFragment(inline) => {
                for c in inline.item.selection_set.iter() {
                    self.visit_selection_children(c, applies_for);
                }
            }
        }
    }

    fn visit_child(&mut self, selection: &'a Selection<'a, S>, applies_for: Applies<'a>) {
        match selection {
            Selection::Field(field) => {
                let field = &field.item;
                if !self.should_include_child(field.directives.as_ref()) {
                    return;
                }
                if let (Applies::OnlyType(type_name), Applies::OnlyType(filter)) =
                    (applies_for, self.type_filter)
                {
                    if type_name != filter {
                        return;
                    }
                }

                self.output.push(LookAheadSelection {
                    source: SelectionSource::Field(field),
                    applies_for,
                    vars: self.vars,
                    fragments: self.fragments,
                });
            }
            Selection::FragmentSpread(fragment) => {
                if !self.should_include_child(fragment.item.directives.as_ref()) {
                    return;
                }
                let f = self
                    .fragments
                    .get(&fragment.item.name.item)
                    .expect("a fragment");
                for c in f.selection_set.iter() {
                    self.visit_child(c, applies_for);
                }
            }
            Selection::InlineFragment(inline) => {
                if !self.should_include_child(inline.item.directives.as_ref()) {
                    return;
                }
                let applies_for = inline
                    .item
                    .type_condition
                    .as_ref()
                    .map(|name| Applies::OnlyType(name.item))
                    .unwrap_or(applies_for);
                for c in inline.item.selection_set.iter() {
                    self.visit_child(c, applies_for);
                }
            }
        }
    }

    fn should_include_child<'b, 'c>(
        &self,
        directives: Option<&'b Vec<Spanning<Directive<S>>>>,
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
                                if let LookAheadValue::Scalar(s) = LookAheadValue::from_input_value(
                                    &v.item,
                                    &v.span,
                                    Some(self.vars),
                                )
                                .item
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
                                if let LookAheadValue::Scalar(b) = LookAheadValue::from_input_value(
                                    &v.item,
                                    &v.span,
                                    Some(self.vars),
                                )
                                .item
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

    fn selection_look_ahead<'a, S: ScalarValue>(
        selection: &'a Selection<'a, S>,
        vars: &'a Variables<S>,
        fragments: &'a HashMap<&'a str, Fragment<'a, S>>,
    ) -> LookAheadSelection<'a, S> {
        let mut collector = ChildrenBuilder {
            vars,
            fragments,
            type_filter: Applies::All,
            output: vec![],
        };
        collector.visit_child(selection, Applies::All);
        collector.output.into_iter().next().unwrap()
    }

    #[derive(PartialEq, Debug)]
    enum ValueDebug<'a, S: ScalarValue> {
        Null,
        Scalar(&'a S),
        Enum(&'a str),
        List(Vec<ValueDebug<'a, S>>),
        Object(Vec<(&'a str, ValueDebug<'a, S>)>),
    }

    impl<'a, S: ScalarValue> From<LookAheadValue<'a, S>> for ValueDebug<'a, S> {
        fn from(look_ahead: LookAheadValue<'a, S>) -> Self {
            match look_ahead {
                LookAheadValue::Null => Self::Null,
                LookAheadValue::Scalar(s) => Self::Scalar(s),
                LookAheadValue::Enum(e) => Self::Enum(e),
                LookAheadValue::List(list) => {
                    Self::List(list.iter().map(|val| val.item.into()).collect())
                }
                LookAheadValue::Object(object) => Self::Object(
                    object
                        .iter()
                        .map(|(key, value)| (key.item, value.item.into()))
                        .collect(),
                ),
            }
        }
    }

    #[derive(PartialEq, Debug)]
    struct LookAheadDebug<'a, S: ScalarValue> {
        name: &'a str,
        alias: Option<&'a str>,
        applies_for: Applies<'a>,
        arguments: Option<Vec<(&'a str, ValueDebug<'a, S>)>>,
        children: Vec<LookAheadDebug<'a, S>>,
    }
    impl<'a, S: ScalarValue> LookAheadDebug<'a, S> {
        fn new(look_ahead: &LookAheadSelection<'a, S>) -> Self {
            Self::new_filtered(look_ahead, Applies::All)
        }

        fn new_filtered(look_ahead: &LookAheadSelection<'a, S>, type_filter: Applies) -> Self {
            Self {
                name: look_ahead.field_name(),
                alias: look_ahead.field_alias(),
                applies_for: look_ahead.applies_for,
                arguments: if let Some(arguments) = look_ahead.arguments() {
                    Some(
                        arguments
                            .iter()
                            .map(|argument| (argument.name(), ValueDebug::from(argument.value())))
                            .collect(),
                    )
                } else {
                    None
                },
                children: look_ahead
                    .build_children(type_filter)
                    .iter()
                    .map(|child| Self::new_filtered(child, type_filter))
                    .collect(),
            }
        }
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
            let look_ahead = selection_look_ahead(&op.item.selection_set[0], &vars, &fragments);

            let expected = LookAheadDebug {
                name: "hero",
                alias: None,
                applies_for: Applies::All,
                arguments: None,
                children: vec![
                    LookAheadDebug {
                        name: "id",
                        alias: None,
                        arguments: None,
                        children: vec![],
                        applies_for: Applies::All,
                    },
                    LookAheadDebug {
                        name: "name",
                        alias: None,
                        arguments: None,
                        children: Vec::new(),
                        applies_for: Applies::All,
                    },
                ],
            };
            assert_eq!(LookAheadDebug::new(&look_ahead), expected);
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
            let look_ahead = selection_look_ahead(&op.item.selection_set[0], &vars, &fragments);

            let expected = LookAheadDebug {
                name: "hero",
                alias: None,
                arguments: None,
                applies_for: Applies::All,
                children: vec![
                    LookAheadDebug {
                        name: "id",
                        alias: None,
                        arguments: None,
                        children: Vec::new(),
                        applies_for: Applies::All,
                    },
                    LookAheadDebug {
                        name: "name",
                        alias: None,
                        arguments: None,
                        children: Vec::new(),
                        applies_for: Applies::All,
                    },
                    LookAheadDebug {
                        name: "friends",
                        alias: None,
                        arguments: None,
                        children: vec![
                            LookAheadDebug {
                                name: "name",
                                alias: None,
                                arguments: None,
                                children: Vec::new(),
                                applies_for: Applies::All,
                            },
                            LookAheadDebug {
                                name: "id",
                                alias: None,
                                arguments: None,
                                children: Vec::new(),
                                applies_for: Applies::All,
                            },
                        ],
                        applies_for: Applies::All,
                    },
                ],
            };
            assert_eq!(LookAheadDebug::new(&look_ahead), expected);
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
            let look_ahead = selection_look_ahead(&op.item.selection_set[0], &vars, &fragments);

            let expected = LookAheadDebug {
                name: "hero",
                alias: None,
                arguments: Some(vec![("episode", ValueDebug::Enum("EMPIRE"))]),
                applies_for: Applies::All,
                children: vec![
                    LookAheadDebug {
                        name: "id",
                        alias: None,
                        arguments: None,
                        children: Vec::new(),
                        applies_for: Applies::All,
                    },
                    LookAheadDebug {
                        name: "name",
                        alias: None,
                        arguments: Some(vec![(
                            "uppercase",
                            ValueDebug::Scalar(&DefaultScalarValue::Boolean(true)),
                        )]),
                        children: Vec::new(),
                        applies_for: Applies::All,
                    },
                ],
            };
            assert_eq!(LookAheadDebug::new(&look_ahead), expected);
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
            let look_ahead = selection_look_ahead(&op.item.selection_set[0], &vars, &fragments);

            let expected = LookAheadDebug {
                name: "hero",
                alias: None,
                arguments: Some(vec![("episode", ValueDebug::Enum("JEDI"))]),
                applies_for: Applies::All,
                children: vec![
                    LookAheadDebug {
                        name: "id",
                        alias: None,
                        arguments: None,
                        children: Vec::new(),
                        applies_for: Applies::All,
                    },
                    LookAheadDebug {
                        name: "name",
                        alias: None,
                        arguments: None,
                        children: Vec::new(),
                        applies_for: Applies::All,
                    },
                ],
            };
            assert_eq!(LookAheadDebug::new(&look_ahead), expected);
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
            let look_ahead = selection_look_ahead(&op.item.selection_set[0], &vars, &fragments);

            let expected = LookAheadDebug {
                name: "hero",
                alias: None,
                arguments: Some(vec![("episode", ValueDebug::Null)]),
                applies_for: Applies::All,
                children: vec![LookAheadDebug {
                    name: "id",
                    alias: None,
                    arguments: None,
                    children: Vec::new(),
                    applies_for: Applies::All,
                }],
            };
            assert_eq!(LookAheadDebug::new(&look_ahead), expected);
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
            let look_ahead = selection_look_ahead(&op.item.selection_set[0], &vars, &fragments);

            let expected = LookAheadDebug {
                name: "hero",
                alias: None,
                arguments: None,
                applies_for: Applies::All,
                children: vec![
                    LookAheadDebug {
                        name: "id",
                        alias: None,
                        arguments: None,
                        children: Vec::new(),
                        applies_for: Applies::All,
                    },
                    LookAheadDebug {
                        name: "name",
                        alias: None,
                        arguments: None,
                        children: Vec::new(),
                        applies_for: Applies::All,
                    },
                    LookAheadDebug {
                        name: "appearsIn",
                        alias: None,
                        arguments: None,
                        children: Vec::new(),
                        applies_for: Applies::All,
                    },
                ],
            };
            assert_eq!(LookAheadDebug::new(&look_ahead), expected);
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
            let look_ahead = selection_look_ahead(&op.item.selection_set[0], &vars, &fragments);

            let expected = LookAheadDebug {
                name: "hero",
                alias: None,
                arguments: None,
                applies_for: Applies::All,
                children: vec![
                    LookAheadDebug {
                        name: "id",
                        alias: None,
                        arguments: None,
                        children: Vec::new(),
                        applies_for: Applies::All,
                    },
                    LookAheadDebug {
                        name: "height",
                        alias: None,
                        arguments: None,
                        children: Vec::new(),
                        applies_for: Applies::All,
                    },
                ],
            };
            assert_eq!(LookAheadDebug::new(&look_ahead), expected);
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
            let look_ahead = selection_look_ahead(&op.item.selection_set[0], &vars, &fragments);

            let expected = LookAheadDebug {
                name: "hero",
                alias: None,
                arguments: None,
                applies_for: Applies::All,
                children: vec![
                    LookAheadDebug {
                        name: "name",
                        alias: None,
                        arguments: None,
                        children: Vec::new(),
                        applies_for: Applies::All,
                    },
                    LookAheadDebug {
                        name: "primaryFunction",
                        alias: None,
                        arguments: None,
                        children: Vec::new(),
                        applies_for: Applies::OnlyType("Droid"),
                    },
                    LookAheadDebug {
                        name: "height",
                        alias: None,
                        arguments: None,
                        children: Vec::new(),
                        applies_for: Applies::OnlyType("Human"),
                    },
                ],
            };
            assert_eq!(LookAheadDebug::new(&look_ahead), expected);
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
            let look_ahead = selection_look_ahead(&op.item.selection_set[0], &vars, &fragments);

            let expected = LookAheadDebug {
                name: "hero",
                alias: None,
                arguments: None,
                applies_for: Applies::All,
                children: vec![LookAheadDebug {
                    name: "id",
                    alias: None,
                    arguments: None,
                    children: Vec::new(),
                    applies_for: Applies::All,
                }],
            };
            assert_eq!(LookAheadDebug::new(&look_ahead), expected);

            let look_ahead = selection_look_ahead(&op.item.selection_set[1], &vars, &fragments);

            let expected = LookAheadDebug {
                name: "human",
                alias: None,
                arguments: None,
                applies_for: Applies::All,
                children: vec![LookAheadDebug {
                    name: "name",
                    alias: None,
                    arguments: None,
                    children: Vec::new(),
                    applies_for: Applies::All,
                }],
            };
            assert_eq!(LookAheadDebug::new(&look_ahead), expected);
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
            let look_ahead = selection_look_ahead(&op.item.selection_set[0], &vars, &fragments);

            let expected = LookAheadDebug {
                name: "hero",
                alias: None,
                arguments: Some(vec![(
                    "id",
                    ValueDebug::Scalar(&DefaultScalarValue::Int(42)),
                )]),
                applies_for: Applies::All,
                children: vec![
                    LookAheadDebug {
                        name: "id",
                        alias: None,
                        arguments: None,
                        children: Vec::new(),
                        applies_for: Applies::All,
                    },
                    LookAheadDebug {
                        name: "__typename",
                        alias: None,
                        arguments: None,
                        children: Vec::new(),
                        applies_for: Applies::All,
                    },
                    LookAheadDebug {
                        name: "name",
                        alias: None,
                        arguments: None,
                        children: Vec::new(),
                        applies_for: Applies::All,
                    },
                    LookAheadDebug {
                        name: "appearsIn",
                        alias: None,
                        arguments: None,
                        children: Vec::new(),
                        applies_for: Applies::All,
                    },
                    LookAheadDebug {
                        name: "primaryFunction",
                        alias: None,
                        arguments: None,
                        children: Vec::new(),
                        applies_for: Applies::OnlyType("Droid"),
                    },
                    LookAheadDebug {
                        name: "height",
                        alias: None,
                        arguments: None,
                        children: Vec::new(),
                        applies_for: Applies::OnlyType("Human"),
                    },
                    LookAheadDebug {
                        name: "friends",
                        alias: None,
                        arguments: None,
                        applies_for: Applies::All,
                        children: vec![
                            LookAheadDebug {
                                name: "__typename",
                                alias: None,
                                arguments: None,
                                children: Vec::new(),
                                applies_for: Applies::All,
                            },
                            LookAheadDebug {
                                name: "name",
                                alias: None,
                                arguments: None,
                                children: Vec::new(),
                                applies_for: Applies::All,
                            },
                            LookAheadDebug {
                                name: "appearsIn",
                                alias: None,
                                arguments: None,
                                children: Vec::new(),
                                applies_for: Applies::All,
                            },
                            LookAheadDebug {
                                name: "primaryFunction",
                                alias: None,
                                arguments: None,
                                children: Vec::new(),
                                applies_for: Applies::OnlyType("Droid"),
                            },
                            LookAheadDebug {
                                name: "height",
                                alias: None,
                                arguments: None,
                                children: Vec::new(),
                                applies_for: Applies::OnlyType("Human"),
                            },
                        ],
                    },
                ],
            };
            assert_eq!(LookAheadDebug::new(&look_ahead), expected);
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
            let look_ahead = selection_look_ahead(&op.item.selection_set[0], &vars, &fragments);

            let expected = LookAheadDebug {
                name: "hero",
                alias: None,
                arguments: None,
                applies_for: Applies::All,
                children: vec![
                    LookAheadDebug {
                        name: "name",
                        alias: None,
                        arguments: None,
                        children: Vec::new(),
                        applies_for: Applies::All,
                    },
                    LookAheadDebug {
                        name: "height",
                        alias: None,
                        arguments: None,
                        children: Vec::new(),
                        applies_for: Applies::OnlyType("Human"),
                    },
                ],
            };
            assert_eq!(
                LookAheadDebug::new_filtered(&look_ahead, Applies::OnlyType("Human")),
                expected
            );
        } else {
            panic!("No Operation found");
        }
    }

    #[test]
    fn check_select_child() {
        let docs = parse_document_source::<DefaultScalarValue>(
            "
    query Hero {
        hero {
            id
            friends {
                id
                name
            }
        }
    }",
        )
        .unwrap();
        let fragments = extract_fragments(&docs);

        if let crate::ast::Definition::Operation(ref op) = docs[0] {
            let vars = graphql_vars! {};
            let look_ahead = selection_look_ahead(&op.item.selection_set[0], &vars, &fragments);

            let id = look_ahead.children().select("id").unwrap();
            let concrete_id = look_ahead
                .children_for_explicit_type("does not matter")
                .select("id")
                .unwrap();
            let expected = LookAheadDebug {
                name: "id",
                alias: None,
                arguments: None,
                children: Vec::new(),
                applies_for: Applies::All,
            };
            assert_eq!(LookAheadDebug::new(&id), expected);
            assert_eq!(LookAheadDebug::new(&concrete_id), expected);

            let friends = look_ahead.children().select("friends").unwrap();
            let concrete_friends = look_ahead
                .children_for_explicit_type("does not matter")
                .select("friends")
                .unwrap();
            let expected = LookAheadDebug {
                name: "friends",
                alias: None,
                arguments: None,
                applies_for: Applies::All,
                children: vec![
                    LookAheadDebug {
                        name: "id",
                        alias: None,
                        arguments: None,
                        children: Vec::new(),
                        applies_for: Applies::All,
                    },
                    LookAheadDebug {
                        name: "name",
                        alias: None,
                        arguments: None,
                        children: Vec::new(),
                        applies_for: Applies::All,
                    },
                ],
            };
            assert_eq!(LookAheadDebug::new(&friends), expected);
            assert_eq!(LookAheadDebug::new(&concrete_friends), expected);
        }
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
            let look_ahead = selection_look_ahead(&op.item.selection_set[0], &vars, &fragments);

            let expected = LookAheadDebug {
                name: "hero",
                alias: None,
                arguments: None,
                applies_for: Applies::All,
                children: vec![LookAheadDebug {
                    name: "friends",
                    alias: None,
                    arguments: None,
                    applies_for: Applies::All,
                    children: vec![LookAheadDebug {
                        name: "name",
                        alias: None,
                        arguments: None,
                        children: Vec::new(),
                        applies_for: Applies::All,
                    }],
                }],
            };
            assert_eq!(LookAheadDebug::new(&look_ahead), expected);
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
            let look_ahead = selection_look_ahead(&op.item.selection_set[0], &vars, &fragments);

            assert_eq!(look_ahead.field_original_name(), "hero");
            assert!(look_ahead.field_alias().is_none());
            assert_eq!(look_ahead.field_name(), "hero");

            assert!(look_ahead.arguments().is_some());
            let arg = look_ahead.arguments().unwrap().iter().next().unwrap();
            assert_eq!(arg.name(), "episode");
            assert_eq!(ValueDebug::from(arg.value()), ValueDebug::Enum("EMPIRE"));

            let children = look_ahead.children();
            assert!(!children.is_empty());
            assert_eq!(
                children.names().collect::<Vec<_>>(),
                vec!["name", "aliasedName", "friends"]
            );
            let mut child_iter = children.iter();

            let name_child = child_iter.next().unwrap();
            assert!(children.has_child("name"));
            assert_eq!(
                LookAheadDebug::new(name_child),
                LookAheadDebug::new(&children.select("name").unwrap())
            );
            assert_eq!(name_child.field_original_name(), "name");
            assert_eq!(name_child.field_alias(), None);
            assert_eq!(name_child.field_name(), "name");
            assert!(name_child.arguments().is_none());
            assert!(name_child.children().is_empty());

            let aliased_name_child = child_iter.next().unwrap();
            assert!(children.has_child("aliasedName"));
            assert_eq!(
                LookAheadDebug::new(aliased_name_child),
                LookAheadDebug::new(&children.select("aliasedName").unwrap())
            );
            assert_eq!(aliased_name_child.field_original_name(), "name");
            assert_eq!(aliased_name_child.field_alias(), Some("aliasedName"));
            assert_eq!(aliased_name_child.field_name(), "aliasedName");
            assert!(aliased_name_child.arguments().is_none());
            assert!(aliased_name_child.children().is_empty());

            let friends_child = child_iter.next().unwrap();
            assert!(children.has_child("friends"));
            assert_eq!(
                LookAheadDebug::new(friends_child),
                LookAheadDebug::new(&children.select("friends").unwrap())
            );
            assert_eq!(friends_child.field_original_name(), "friends");
            assert_eq!(friends_child.field_alias(), None);
            assert_eq!(friends_child.field_name(), "friends");
            assert!(friends_child.arguments().is_none());
            assert!(!friends_child.children().is_empty());
            assert_eq!(
                friends_child.children().names().collect::<Vec<_>>(),
                vec!["name"]
            );

            assert!(child_iter.next().is_none());

            let friends_children = friends_child.children();
            let mut friends_child_iter = friends_children.iter();
            let child = friends_child_iter.next().unwrap();
            assert!(friends_children.has_child("name"));
            assert_eq!(
                LookAheadDebug::new(child),
                LookAheadDebug::new(&children.select("name").unwrap())
            );
            assert_eq!(child.field_original_name(), "name");
            assert_eq!(child.field_alias(), None);
            assert_eq!(child.field_name(), "name");
            assert!(child.arguments().is_none());
            assert!(child.children().is_empty());

            assert!(friends_child_iter.next().is_none());
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
            let look_ahead = selection_look_ahead(&op.item.selection_set[0], &vars, &fragments);

            let mut children = look_ahead.children_for_explicit_type("Human").into_iter();
            let heights_child = children.next().unwrap();
            assert_eq!(heights_child.field_original_name(), "height");
            assert_eq!(heights_child.applies_for, Applies::OnlyType("Human"));
            assert_eq!(heights_child.applies_for().unwrap(), "Human");
        } else {
            panic!("No Operation found");
        }
    }
}
