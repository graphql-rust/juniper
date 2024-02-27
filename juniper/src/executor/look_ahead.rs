use std::{collections::HashMap, vec};

use crate::{
    ast::{Directive, Field, Fragment, InputValue, Selection},
    parser::{Span, Spanning},
    value::ScalarValue,
};

use super::Variables;

/// Indication whether a field is available in all types of an interface or only in a certain
/// subtype.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Applies<'a> {
    /// Field is always available, independently from the type.
    All,

    /// Field is only available for the type with the specified typename.
    OnlyType(&'a str),
}

/// Shortcut for a [`Spanning`] containing a borrowed [`Span`].
type BorrowedSpanning<'a, T> = Spanning<T, &'a Span>;

/// JSON-like value performing [look-ahead][0] operations on an executed GraphQL query.
///
/// In contrast to an [`InputValue`], these values do only contain constants, meaning that GraphQL
/// variables get automatically resolved.
///
/// [0]: https://en.wikipedia.org/wiki/Look-ahead_(backtracking)
#[derive(Clone, Debug, PartialEq)]
#[allow(missing_docs)]
#[must_use]
pub enum LookAheadValue<'a, S: ScalarValue + 'a> {
    Null,
    Scalar(&'a S),
    Enum(&'a str),
    List(LookAheadList<'a, S>),
    Object(LookAheadObject<'a, S>),
}

// Implemented manually to omit redundant `S: Copy` trait bound, imposed by `#[derive(Copy)]`.
impl<'a, S: ScalarValue + 'a> Copy for LookAheadValue<'a, S> where Self: Clone {}

impl<'a, S: ScalarValue + 'a> LookAheadValue<'a, S> {
    fn from_input_value(
        input_value: BorrowedSpanning<'a, &'a InputValue<S>>,
        vars: Option<&'a Variables<S>>,
    ) -> BorrowedSpanning<'a, Self> {
        let Spanning {
            item: input_value,
            span: input_span,
        } = input_value;
        Spanning {
            item: match input_value {
                InputValue::Null => Self::Null,
                InputValue::Scalar(s) => Self::Scalar(s),
                InputValue::Enum(e) => Self::Enum(e),
                InputValue::Variable(name) => vars
                    .and_then(|vars| vars.get(name))
                    .map(|item| {
                        Self::from_input_value(
                            BorrowedSpanning {
                                item,
                                span: input_span,
                            },
                            vars,
                        )
                        .item
                    })
                    .unwrap_or(Self::Null),
                InputValue::List(input_list) => Self::List(LookAheadList { input_list, vars }),
                InputValue::Object(input_object) => Self::Object(LookAheadObject {
                    input_object: input_object.as_slice(),
                    vars,
                }),
            },
            span: input_span,
        }
    }
}

/// [Lazy][2]-evaluated [list] used in [look-ahead][0] operations on an executed GraphQL query.
///
/// [0]: https://en.wikipedia.org/wiki/Look-ahead_(backtracking)
/// [2]: https://en.wikipedia.org/wiki/Lazy_evaluation
/// [list]: https://spec.graphql.org/October2021#sec-List
#[derive(Debug)]
#[must_use]
pub struct LookAheadList<'a, S> {
    input_list: &'a [Spanning<InputValue<S>>],
    vars: Option<&'a Variables<S>>,
}

// Implemented manually to omit redundant `S: Clone` trait bound, imposed by `#[derive(Clone)]`.
impl<'a, S> Clone for LookAheadList<'a, S> {
    fn clone(&self) -> Self {
        *self
    }
}

// Implemented manually to omit redundant `S: Copy` trait bound, imposed by `#[derive(Copy)]`.
impl<'a, S> Copy for LookAheadList<'a, S> {}

// Implemented manually to omit redundant `S: Default` trait bound, imposed by `#[derive(Default)]`.
impl<'a, S> Default for LookAheadList<'a, S> {
    fn default() -> Self {
        Self {
            input_list: &[],
            vars: None,
        }
    }
}

// Implemented manually to omit redundant `S: PartialEq` trait bound, imposed by
// `#[derive(PartialEq)]`.
impl<'a, S: ScalarValue> PartialEq for LookAheadList<'a, S> {
    fn eq(&self, other: &Self) -> bool {
        self.iter().eq(other.iter())
    }
}

impl<'a, S: ScalarValue> LookAheadList<'a, S> {
    /// Returns an [`Iterator`] over the items of this [list].
    ///
    /// [list]: https://spec.graphql.org/October2021#sec-List
    pub fn iter(&self) -> <&Self as IntoIterator>::IntoIter {
        self.into_iter()
    }
}

impl<'a, S: ScalarValue> IntoIterator for LookAheadList<'a, S> {
    type Item = BorrowedSpanning<'a, LookAheadValue<'a, S>>;
    type IntoIter = look_ahead_list::Iter<'a, S>;

    fn into_iter(self) -> Self::IntoIter {
        (&self).into_iter()
    }
}

impl<'a, S: ScalarValue> IntoIterator for &LookAheadList<'a, S> {
    type Item = BorrowedSpanning<'a, LookAheadValue<'a, S>>;
    type IntoIter = look_ahead_list::Iter<'a, S>;

    fn into_iter(self) -> Self::IntoIter {
        look_ahead_list::Iter {
            slice_iter: self.input_list.iter(),
            vars: self.vars,
        }
    }
}

pub mod look_ahead_list {
    //! [`LookAheadList`] helper definitions.

    use std::slice;

    #[cfg(doc)]
    use super::LookAheadList;
    use super::{BorrowedSpanning, InputValue, LookAheadValue, ScalarValue, Spanning, Variables};

    /// [`Iterator`] over [`LookAheadList`] items ([`LookAheadValue`]s) by value.
    ///
    /// GraphQL variables are resolved lazily as this [`Iterator`] advances.
    #[must_use]
    pub struct Iter<'a, S> {
        pub(super) slice_iter: slice::Iter<'a, Spanning<InputValue<S>>>,
        pub(super) vars: Option<&'a Variables<S>>,
    }

    impl<'a, S: ScalarValue> Iterator for Iter<'a, S> {
        type Item = BorrowedSpanning<'a, LookAheadValue<'a, S>>;

        fn next(&mut self) -> Option<Self::Item> {
            let vars = self.vars;
            self.slice_iter
                .next()
                .map(move |val| LookAheadValue::from_input_value(val.as_ref(), vars))
        }
    }

    impl<'a, S: ScalarValue> DoubleEndedIterator for Iter<'a, S> {
        fn next_back(&mut self) -> Option<Self::Item> {
            let vars = self.vars;
            self.slice_iter
                .next_back()
                .map(move |val| LookAheadValue::from_input_value(val.as_ref(), vars))
        }
    }
}

/// [Lazy][2]-evaluated [input object] used in [look-ahead][0] operations on an executed GraphQL
/// query.
///
/// [0]: https://en.wikipedia.org/wiki/Look-ahead_(backtracking)
/// [2]: https://en.wikipedia.org/wiki/Lazy_evaluation
/// [input object]: https://spec.graphql.org/October2021#sec-Input-Objects
#[derive(Debug)]
#[must_use]
pub struct LookAheadObject<'a, S> {
    input_object: &'a [(Spanning<String>, Spanning<InputValue<S>>)],
    vars: Option<&'a Variables<S>>,
}

// Implemented manually to omit redundant `S: Clone` trait bound, imposed by `#[derive(Clone)]`.
impl<'a, S> Clone for LookAheadObject<'a, S> {
    fn clone(&self) -> Self {
        *self
    }
}

// Implemented manually to omit redundant `S: Copy` trait bound, imposed by `#[derive(Copy)]`.
impl<'a, S> Copy for LookAheadObject<'a, S> {}

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

impl<'a, S: ScalarValue> LookAheadObject<'a, S> {
    /// Returns an [`Iterator`] over this [input object]'s fields.
    ///
    /// [input object]: https://spec.graphql.org/October2021#sec-Input-Objects
    pub fn iter(&self) -> <&Self as IntoIterator>::IntoIter {
        self.into_iter()
    }
}

impl<'a, S: ScalarValue> IntoIterator for LookAheadObject<'a, S> {
    type Item = (
        BorrowedSpanning<'a, &'a str>,
        BorrowedSpanning<'a, LookAheadValue<'a, S>>,
    );
    type IntoIter = look_ahead_object::Iter<'a, S>;

    fn into_iter(self) -> Self::IntoIter {
        (&self).into_iter()
    }
}

impl<'a, S: ScalarValue> IntoIterator for &LookAheadObject<'a, S> {
    type Item = (
        BorrowedSpanning<'a, &'a str>,
        BorrowedSpanning<'a, LookAheadValue<'a, S>>,
    );
    type IntoIter = look_ahead_object::Iter<'a, S>;

    fn into_iter(self) -> Self::IntoIter {
        look_ahead_object::Iter {
            slice_iter: self.input_object.iter(),
            vars: self.vars,
        }
    }
}

pub mod look_ahead_object {
    //! [`LookAheadObject`] helper definitions.

    use std::slice;

    #[cfg(doc)]
    use super::LookAheadList;
    use super::{BorrowedSpanning, InputValue, LookAheadValue, ScalarValue, Spanning, Variables};

    /// [`Iterator`] over [`LookAheadObject`] fields (named [`LookAheadValue`]s) by value.
    ///
    /// GraphQL variables are resolved lazily as this [`Iterator`] advances.
    #[must_use]
    pub struct Iter<'a, S> {
        pub(super) slice_iter: slice::Iter<'a, (Spanning<String>, Spanning<InputValue<S>>)>,
        pub(super) vars: Option<&'a Variables<S>>,
    }

    impl<'a, S: ScalarValue> Iterator for Iter<'a, S> {
        type Item = (
            BorrowedSpanning<'a, &'a str>,
            BorrowedSpanning<'a, LookAheadValue<'a, S>>,
        );

        fn next(&mut self) -> Option<Self::Item> {
            let vars = self.vars;
            self.slice_iter.next().map(move |(key, val)| {
                (
                    Spanning {
                        span: &key.span,
                        item: key.item.as_str(),
                    },
                    LookAheadValue::from_input_value(val.as_ref(), vars),
                )
            })
        }
    }

    impl<'a, S: ScalarValue> DoubleEndedIterator for Iter<'a, S> {
        fn next_back(&mut self) -> Option<Self::Item> {
            let vars = self.vars;
            self.slice_iter.next_back().map(move |(key, val)| {
                (
                    Spanning {
                        span: &key.span,
                        item: key.item.as_str(),
                    },
                    LookAheadValue::from_input_value(val.as_ref(), vars),
                )
            })
        }
    }
}

/// [Lazy][2]-evaluated [argument] used in [look-ahead][0] operations on an executed GraphQL query.
///
/// [0]: https://en.wikipedia.org/wiki/Look-ahead_(backtracking)
/// [2]: https://en.wikipedia.org/wiki/Lazy_evaluation
/// [argument]: https://spec.graphql.org/October2021#sec-Language.Arguments
#[derive(Debug)]
#[must_use]
pub struct LookAheadArgument<'a, S> {
    name: &'a Spanning<&'a str>,
    input_value: &'a Spanning<InputValue<S>>,
    vars: &'a Variables<S>,
}

// Implemented manually to omit redundant `S: Clone` trait bound, imposed by `#[derive(Clone)]`.
impl<'a, S> Clone for LookAheadArgument<'a, S> {
    fn clone(&self) -> Self {
        *self
    }
}

// Implemented manually to omit redundant `S: Copy` trait bound, imposed by `#[derive(Copy)]`.
impl<'a, S> Copy for LookAheadArgument<'a, S> {}

impl<'a, S> LookAheadArgument<'a, S> {
    /// Returns the name of this [argument].
    ///
    /// [argument]: https://spec.graphql.org/October2021#sec-Language.Arguments
    #[must_use]
    pub fn name(&self) -> &'a str {
        self.name.item
    }

    /// Returns the [`Span`] of this [argument]'s [`name`].
    ///
    /// [`name`]: LookAheadArgument::name()
    /// [argument]: https://spec.graphql.org/October2021#sec-Language.Arguments
    #[must_use]
    pub fn name_span(&self) -> &'a Span {
        &self.name.span
    }

    /// Evaluates and returns the value of this [argument].
    ///
    /// [argument]: https://spec.graphql.org/October2021#sec-Language.Arguments
    pub fn value(&self) -> LookAheadValue<'a, S>
    where
        S: ScalarValue,
    {
        LookAheadValue::from_input_value(self.input_value.as_ref(), Some(self.vars)).item
    }

    /// Returns the [`Span`] of this [argument]'s [`value`].
    ///
    /// [`value`]: LookAheadArgument::value()
    /// [argument]: https://spec.graphql.org/October2021#sec-Language.Arguments
    #[must_use]
    pub fn value_span(&self) -> &'a Span {
        &self.input_value.span
    }
}

/// Children of a [`LookAheadSelection`].
#[derive(Debug)]
#[must_use]
pub struct LookAheadChildren<'a, S> {
    children: Vec<LookAheadSelection<'a, S>>,
}

// Implemented manually to omit redundant `S: Clone` trait bound, imposed by `#[derive(Clone)]`.
impl<'a, S> Clone for LookAheadChildren<'a, S> {
    fn clone(&self) -> Self {
        Self {
            children: self.children.clone(),
        }
    }
}

// Implemented manually to omit redundant `S: Default` trait bound, imposed by `#[derive(Default)]`.
impl<'a, S> Default for LookAheadChildren<'a, S> {
    fn default() -> Self {
        Self { children: vec![] }
    }
}

impl<'a, S> LookAheadChildren<'a, S> {
    /// Returns the number of children present.
    #[must_use]
    pub fn len(&self) -> usize {
        self.children.len()
    }

    /// Indicates whether the current [selection] has any children.
    ///
    /// [selection]: https://spec.graphql.org/October2021#sec-Selection-Sets
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.children.is_empty()
    }

    /// Returns the child [selection] for the specified [field].
    ///
    /// If a child has an alias, it will only match if the alias matches the specified `name`.
    ///
    /// [field]: https://spec.graphql.org/October2021#sec-Language.Fields
    /// [selection]: https://spec.graphql.org/October2021#sec-Selection-Sets
    #[must_use]
    pub fn select(&self, name: &str) -> Option<LookAheadSelection<'a, S>> {
        self.children
            .iter()
            .find(|child| child.field_name() == name)
            .copied()
    }

    /// Checks if the child [selection] with the specified `name` exists.
    ///
    /// If a child has an alias, it will only match if the alias matches the specified `name`.
    ///
    /// [selection]: https://spec.graphql.org/October2021#sec-Selection-Sets
    #[must_use]
    pub fn has_child(&self, name: &str) -> bool {
        self.select(name).is_some()
    }

    /// Returns the possibly aliased names of the top-level children from the current [selection].
    ///
    /// [selection]: https://spec.graphql.org/October2021#sec-Selection-Sets
    pub fn names(&self) -> impl DoubleEndedIterator<Item = &'a str> + '_ {
        self.children.iter().map(|sel| sel.field_name())
    }

    /// Returns an [`Iterator`] over these children, by reference.
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &LookAheadSelection<'a, S>> + '_ {
        self.children.iter()
    }
}

impl<'a, S: ScalarValue> IntoIterator for LookAheadChildren<'a, S> {
    type Item = LookAheadSelection<'a, S>;
    type IntoIter = vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.children.into_iter()
    }
}

#[derive(Debug)]
pub(super) enum SelectionSource<'a, S> {
    Field(&'a Field<'a, S>),
    Spread {
        field_name: &'a str,
        set: Option<&'a [Selection<'a, S>]>,
    },
}

// Implemented manually to omit redundant `S: Clone` trait bound, imposed by `#[derive(Clone)]`.
impl<'a, S> Clone for SelectionSource<'a, S> {
    fn clone(&self) -> Self {
        *self
    }
}

// Implemented manually to omit redundant `S: Copy` trait bound, imposed by `#[derive(Copy)]`.
impl<'a, S> Copy for SelectionSource<'a, S> {}

/// [Selection] of an executed GraphQL query, used in [look-ahead][0] operations.
///
/// [0]: https://en.wikipedia.org/wiki/Look-ahead_(backtracking)
/// [2]: https://en.wikipedia.org/wiki/Lazy_evaluation
/// [Selection]: https://spec.graphql.org/October2021#sec-Selection-Sets
#[derive(Debug)]
#[must_use]
pub struct LookAheadSelection<'a, S> {
    source: SelectionSource<'a, S>,
    applies_for: Applies<'a>,
    vars: &'a Variables<S>,
    fragments: &'a HashMap<&'a str, Fragment<'a, S>>,
}

// Implemented manually to omit redundant `S: Clone` trait bound, imposed by `#[derive(Clone)]`.
impl<'a, S> Clone for LookAheadSelection<'a, S> {
    fn clone(&self) -> Self {
        *self
    }
}

// Implemented manually to omit redundant `S: Copy` trait bound, imposed by `#[derive(Copy)]`.
impl<'a, S> Copy for LookAheadSelection<'a, S> {}

impl<'a, S> LookAheadSelection<'a, S> {
    /// Constructs a new [`LookAheadSelection`] out of the provided params.
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

    /// Returns the original name of the [field], represented by the current [selection].
    ///
    /// [field]: https://spec.graphql.org/October2021#sec-Language.Fields
    /// [selection]: https://spec.graphql.org/October2021#sec-Selection-Sets
    #[must_use]
    pub fn field_original_name(&self) -> &'a str {
        match self.source {
            SelectionSource::Field(f) => f.name.item,
            SelectionSource::Spread { field_name, .. } => field_name,
        }
    }

    /// Returns the alias of the [field], represented by the current [selection], if any is present.
    ///
    /// [field]: https://spec.graphql.org/October2021#sec-Language.Fields
    /// [selection]: https://spec.graphql.org/October2021#sec-Selection-Sets
    #[must_use]
    pub fn field_alias(&self) -> Option<&'a str> {
        match self.source {
            SelectionSource::Field(f) => f.alias.map(|a| a.item),
            SelectionSource::Spread { .. } => None,
        }
    }

    /// Returns the potentially aliased name of the [field], represented by the current [selection].
    ///
    /// [field]: https://spec.graphql.org/October2021#sec-Language.Fields
    /// [selection]: https://spec.graphql.org/October2021#sec-Selection-Sets
    #[must_use]
    pub fn field_name(&self) -> &'a str {
        self.field_alias()
            .unwrap_or_else(|| self.field_original_name())
    }

    /// Indicates whether the current [selection] has any [arguments].
    ///
    /// [arguments]: https://spec.graphql.org/October2021#sec-Language.Arguments
    /// [selection]: https://spec.graphql.org/October2021#sec-Selection-Sets
    #[must_use]
    pub fn has_arguments(&self) -> bool {
        match self.source {
            SelectionSource::Field(f) => match &f.arguments {
                Some(args) => !args.item.items.is_empty(),
                None => false,
            },
            _ => false,
        }
    }

    /// Returns an [`Iterator`] over the top-level [arguments] from the current [selection], if any
    /// are present.
    ///
    /// [arguments]: https://spec.graphql.org/October2021#sec-Language.Arguments
    /// [selection]: https://spec.graphql.org/October2021#sec-Selection-Sets
    pub fn arguments(&self) -> impl DoubleEndedIterator<Item = LookAheadArgument<'a, S>> {
        let opt_arguments = match self.source {
            SelectionSource::Field(f) => f.arguments.as_ref(),
            _ => None,
        };

        opt_arguments
            .into_iter()
            .flat_map(|args| args.item.iter())
            .map(|(name, arg)| LookAheadArgument {
                name,
                input_value: arg,
                vars: self.vars,
            })
    }

    /// Returns the top-level [argument] from the current [selection] by its `name`, if any is
    /// present.
    ///
    /// [argument]: https://spec.graphql.org/October2021#sec-Language.Arguments
    /// [selection]: https://spec.graphql.org/October2021#sec-Selection-Sets
    #[must_use]
    pub fn argument(&self, name: &str) -> Option<LookAheadArgument<'a, S>> {
        self.arguments().find(|arg| arg.name() == name)
    }

    /// Returns the children from the current [selection].
    ///
    /// [selection]: https://spec.graphql.org/October2021#sec-Selection-Sets
    pub fn children(&self) -> LookAheadChildren<'a, S>
    where
        S: ScalarValue,
    {
        self.build_children(Applies::All)
    }

    /// Returns the children from the current [selection] applying to the specified [type] only.
    ///
    /// [selection]: https://spec.graphql.org/October2021#sec-Selection-Sets
    /// [type]: https://spec.graphql.org/October2021#sec-Types
    pub fn children_for_explicit_type(&self, type_name: &str) -> LookAheadChildren<'a, S>
    where
        S: ScalarValue,
    {
        self.build_children(Applies::OnlyType(type_name))
    }

    fn build_children(&self, type_filter: Applies) -> LookAheadChildren<'a, S>
    where
        S: ScalarValue,
    {
        let mut builder = ChildrenBuilder {
            vars: self.vars,
            fragments: self.fragments,
            type_filter,
            output: vec![],
        };
        match &self.source {
            SelectionSource::Field(f) => {
                builder.visit_parent_field(f, Applies::All);
            }
            SelectionSource::Spread {
                set: Some(selections),
                ..
            } => {
                for s in selections.iter() {
                    builder.visit_parent_selection(s, Applies::All);
                }
            }
            SelectionSource::Spread { set: None, .. } => {}
        }
        LookAheadChildren {
            children: builder.output,
        }
    }

    /// Returns the name of parent [type], in case there is any for the current [selection].
    ///
    /// [selection]: https://spec.graphql.org/October2021#sec-Selection-Sets
    /// [type]: https://spec.graphql.org/October2021#sec-Types
    #[must_use]
    pub fn applies_for(&self) -> Option<&str> {
        match self.applies_for {
            Applies::OnlyType(name) => Some(name),
            Applies::All => None,
        }
    }
}

struct ChildrenBuilder<'a, 'f, S> {
    vars: &'a Variables<S>,
    fragments: &'a HashMap<&'a str, Fragment<'a, S>>,
    type_filter: Applies<'f>,
    output: Vec<LookAheadSelection<'a, S>>,
}

impl<'a, 'f, S: ScalarValue> ChildrenBuilder<'a, 'f, S> {
    fn visit_parent_selection(
        &mut self,
        selection: &'a Selection<'a, S>,
        applies_for: Applies<'a>,
    ) {
        match selection {
            Selection::Field(f) => {
                self.visit_parent_field(&f.item, applies_for);
            }
            Selection::FragmentSpread(frag_sp) => {
                let fragment = self
                    .fragments
                    .get(&frag_sp.item.name.item)
                    .expect("a fragment");
                for sel in &fragment.selection_set {
                    self.visit_parent_selection(sel, applies_for);
                }
            }
            Selection::InlineFragment(inl_frag) => {
                for sel in &inl_frag.item.selection_set {
                    self.visit_parent_selection(sel, applies_for);
                }
            }
        }
    }

    fn visit_parent_field(&mut self, field: &'a Field<'a, S>, applies_for: Applies<'a>) {
        if let Some(selection_set) = &field.selection_set {
            for sel in selection_set {
                self.visit_child(sel, applies_for);
            }
        }
    }

    fn visit_child(&mut self, selection: &'a Selection<'a, S>, applies_for: Applies<'a>) {
        match selection {
            Selection::Field(f) => {
                let field = &f.item;
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
            Selection::FragmentSpread(frag_sp) => {
                if !self.should_include_child(frag_sp.item.directives.as_ref()) {
                    return;
                }
                let fragment = self
                    .fragments
                    .get(&frag_sp.item.name.item)
                    .expect("a fragment");
                for sel in &fragment.selection_set {
                    self.visit_child(sel, applies_for);
                }
            }
            Selection::InlineFragment(inl_frag) => {
                if !self.should_include_child(inl_frag.item.directives.as_ref()) {
                    return;
                }
                let applies_for = inl_frag
                    .item
                    .type_condition
                    .as_ref()
                    .map(|name| Applies::OnlyType(name.item))
                    .unwrap_or(applies_for);
                for sel in &inl_frag.item.selection_set {
                    self.visit_child(sel, applies_for);
                }
            }
        }
    }

    fn should_include_child<'b: 'a, 'c: 'a>(
        &self,
        directives: Option<&'b Vec<Spanning<Directive<S>>>>,
    ) -> bool {
        use std::ops::Not;

        directives
            .map(|d| {
                d.iter().all(|d| {
                    let directive = &d.item;
                    match (directive.name.item, &directive.arguments) {
                        ("include", Some(args)) => args
                            .item
                            .items
                            .iter()
                            .find(|i| i.0.item == "if")
                            .map(|(_, v)| {
                                if let LookAheadValue::Scalar(s) =
                                    LookAheadValue::from_input_value(v.as_ref(), Some(self.vars))
                                        .item
                                {
                                    s.as_bool().unwrap_or(false)
                                } else {
                                    false
                                }
                            })
                            .unwrap_or(false),
                        ("skip", Some(args)) => args
                            .item
                            .items
                            .iter()
                            .find(|i| i.0.item == "if")
                            .map(|(_, v)| {
                                if let LookAheadValue::Scalar(b) =
                                    LookAheadValue::from_input_value(v.as_ref(), Some(self.vars))
                                        .item
                                {
                                    b.as_bool().map(Not::not).unwrap_or(false)
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

    #[derive(Debug, PartialEq)]
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

    #[derive(Debug, PartialEq)]
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
                arguments: if look_ahead.has_arguments() {
                    Some(
                        look_ahead
                            .arguments()
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
            //language=GraphQL
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
            //language=GraphQL
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
            //language=GraphQL
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
            //language=GraphQL
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
            //language=GraphQL
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
            //language=GraphQL
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
            //language=GraphQL
            "
            query Hero {
                hero {
                    id @include(if: true)
                    name @include(if: false)
                    appearsIn @skip(if: true)
                    height @skip(if: false)
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
            //language=GraphQL
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
            //language=GraphQL
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
            //language=GraphQL
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
            }
            ",
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
            //language=GraphQL
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
            //language=GraphQL
            "
            query Hero {
                hero {
                    id
                    friends {
                        id
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
            //language=GraphQL
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
            //language=GraphQL
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

            assert!(look_ahead.has_arguments());
            let arg = look_ahead.arguments().next().unwrap();
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
            assert!(!name_child.has_arguments());
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
            assert!(!aliased_name_child.has_arguments());
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
            assert!(!friends_child.has_arguments());
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
            assert!(!child.has_arguments());
            assert!(child.children().is_empty());

            assert!(friends_child_iter.next().is_none());
        } else {
            panic!("No Operation found");
        }
    }

    #[test]
    fn check_resolves_applies_for() {
        let docs = parse_document_source::<DefaultScalarValue>(
            //language=GraphQL
            "
            query Hero {
                hero {
                    ... on Human {
                        height
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
