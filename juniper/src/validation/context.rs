use std::{collections::HashSet, fmt::Debug};

use derive_more::with_trait::{Display, Error};
use itertools::Itertools as _;

use crate::{
    ast::{BorrowedType, Definition, Document},
    parser::SourcePosition,
    schema::{meta::MetaType, model::SchemaType},
};

/// Query validation error
#[derive(Clone, Debug, Display, Eq, Error, Ord, PartialEq, PartialOrd)]
#[display("{message}. At {}", locations.iter().format(", "))]
pub struct RuleError {
    locations: Vec<SourcePosition>,
    message: String,
}

#[doc(hidden)]
pub struct ValidatorContext<'a, S: Debug + 'a> {
    pub schema: &'a SchemaType<S>,
    errors: Vec<RuleError>,
    type_stack: Vec<Option<&'a MetaType<S>>>,
    type_literal_stack: Vec<Option<BorrowedType<'a>>>,
    input_type_stack: Vec<Option<&'a MetaType<S>>>,
    input_type_literal_stack: Vec<Option<BorrowedType<'a>>>,
    parent_type_stack: Vec<Option<&'a MetaType<S>>>,
    fragment_names: HashSet<&'a str>,
}

impl RuleError {
    #[doc(hidden)]
    pub fn new(message: &str, locations: &[SourcePosition]) -> Self {
        Self {
            message: message.into(),
            locations: locations.to_vec(),
        }
    }

    /// Access the message for a validation error
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Access the positions of the validation error
    ///
    /// All validation errors contain at least one source position, but some
    /// validators supply extra context through multiple positions.
    pub fn locations(&self) -> &[SourcePosition] {
        &self.locations
    }
}

impl<'a, S: Debug> ValidatorContext<'a, S> {
    #[doc(hidden)]
    pub fn new(schema: &'a SchemaType<S>, document: &Document<'a, S>) -> Self {
        Self {
            errors: Vec::new(),
            schema,
            type_stack: Vec::new(),
            type_literal_stack: Vec::new(),
            parent_type_stack: Vec::new(),
            input_type_stack: Vec::new(),
            input_type_literal_stack: Vec::new(),
            fragment_names: document
                .iter()
                .filter_map(|def| match *def {
                    Definition::Fragment(ref frag) => Some(frag.item.name.item),
                    _ => None,
                })
                .collect(),
        }
    }

    #[doc(hidden)]
    pub fn append_errors(&mut self, mut errors: Vec<RuleError>) {
        self.errors.append(&mut errors);
    }

    #[doc(hidden)]
    pub fn report_error(&mut self, message: &str, locations: &[SourcePosition]) {
        self.errors.push(RuleError::new(message, locations))
    }

    pub(crate) fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    #[doc(hidden)]
    pub fn into_errors(mut self) -> Vec<RuleError> {
        self.errors.sort();
        self.errors
    }

    #[doc(hidden)]
    pub fn with_pushed_type<F, R>(&mut self, t: Option<impl Into<BorrowedType<'a>>>, f: F) -> R
    where
        F: FnOnce(&mut ValidatorContext<'a, S>) -> R,
    {
        let t = t.map(Into::into);

        if let Some(t) = t {
            self.type_stack
                .push(self.schema.concrete_type_by_name(t.innermost_name()));
        } else {
            self.type_stack.push(None);
        }

        self.type_literal_stack.push(t);

        let res = f(self);

        self.type_literal_stack.pop();
        self.type_stack.pop();

        res
    }

    #[doc(hidden)]
    pub fn with_pushed_parent_type<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce(&mut ValidatorContext<'a, S>) -> R,
    {
        self.parent_type_stack
            .push(*self.type_stack.last().unwrap_or(&None));
        let res = f(self);
        self.parent_type_stack.pop();

        res
    }

    #[doc(hidden)]
    pub fn with_pushed_input_type<F, R>(
        &mut self,
        t: Option<impl Into<BorrowedType<'a>>>,
        f: F,
    ) -> R
    where
        F: FnOnce(&mut ValidatorContext<'a, S>) -> R,
    {
        let t = t.map(Into::into);

        if let Some(t) = t {
            self.input_type_stack
                .push(self.schema.concrete_type_by_name(t.innermost_name()));
        } else {
            self.input_type_stack.push(None);
        }

        self.input_type_literal_stack.push(t);

        let res = f(self);

        self.input_type_literal_stack.pop();
        self.input_type_stack.pop();

        res
    }

    #[doc(hidden)]
    pub fn current_type(&self) -> Option<&'a MetaType<S>> {
        *self.type_stack.last().unwrap_or(&None)
    }

    #[doc(hidden)]
    pub fn current_type_literal(&self) -> Option<BorrowedType<'a>> {
        match self.type_literal_stack.last() {
            Some(Some(t)) => Some(*t),
            _ => None,
        }
    }

    #[doc(hidden)]
    pub fn parent_type(&self) -> Option<&'a MetaType<S>> {
        *self.parent_type_stack.last().unwrap_or(&None)
    }

    #[doc(hidden)]
    pub fn current_input_type_literal(&self) -> Option<BorrowedType<'a>> {
        match self.input_type_literal_stack.last() {
            Some(Some(t)) => Some(*t),
            _ => None,
        }
    }

    #[doc(hidden)]
    pub fn is_known_fragment(&self, name: &str) -> bool {
        self.fragment_names.contains(name)
    }
}
