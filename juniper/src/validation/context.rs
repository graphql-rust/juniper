use std::{
    collections::HashSet,
    fmt::{self, Debug},
};

use crate::ast::{Definition, Document, Type};

use crate::schema::{meta::MetaType, model::SchemaType};

use crate::parser::SourcePosition;

/// Query validation error
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct RuleError {
    locations: Vec<SourcePosition>,
    message: String,
}

#[doc(hidden)]
pub struct ValidatorContext<'a, S: Debug + 'a> {
    pub schema: &'a SchemaType<'a, S>,
    errors: Vec<RuleError>,
    type_stack: Vec<Option<&'a MetaType<'a, S>>>,
    type_literal_stack: Vec<Option<Type<'a>>>,
    input_type_stack: Vec<Option<&'a MetaType<'a, S>>>,
    input_type_literal_stack: Vec<Option<Type<'a>>>,
    parent_type_stack: Vec<Option<&'a MetaType<'a, S>>>,
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

impl fmt::Display for RuleError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // This is fine since all `RuleError`s should have at least one source
        // position.
        let locations = self
            .locations
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ");
        write!(f, "{}. At {locations}", self.message)
    }
}

impl std::error::Error for RuleError {}

impl<'a, S: Debug> ValidatorContext<'a, S> {
    #[doc(hidden)]
    pub fn new(schema: &'a SchemaType<S>, document: &Document<'a, S>) -> ValidatorContext<'a, S> {
        ValidatorContext {
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
    pub fn with_pushed_type<F, R>(&mut self, t: Option<&Type<'a>>, f: F) -> R
    where
        F: FnOnce(&mut ValidatorContext<'a, S>) -> R,
    {
        if let Some(t) = t {
            self.type_stack
                .push(self.schema.concrete_type_by_name(t.innermost_name()));
        } else {
            self.type_stack.push(None);
        }

        self.type_literal_stack.push(t.cloned());

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
    pub fn with_pushed_input_type<F, R>(&mut self, t: Option<&Type<'a>>, f: F) -> R
    where
        F: FnOnce(&mut ValidatorContext<'a, S>) -> R,
    {
        if let Some(t) = t {
            self.input_type_stack
                .push(self.schema.concrete_type_by_name(t.innermost_name()));
        } else {
            self.input_type_stack.push(None);
        }

        self.input_type_literal_stack.push(t.cloned());

        let res = f(self);

        self.input_type_literal_stack.pop();
        self.input_type_stack.pop();

        res
    }

    #[doc(hidden)]
    pub fn current_type(&self) -> Option<&'a MetaType<'a, S>> {
        *self.type_stack.last().unwrap_or(&None)
    }

    #[doc(hidden)]
    pub fn current_type_literal(&self) -> Option<&Type<'a>> {
        match self.type_literal_stack.last() {
            Some(Some(t)) => Some(t),
            _ => None,
        }
    }

    #[doc(hidden)]
    pub fn parent_type(&self) -> Option<&'a MetaType<'a, S>> {
        *self.parent_type_stack.last().unwrap_or(&None)
    }

    #[doc(hidden)]
    pub fn current_input_type_literal(&self) -> Option<&Type<'a>> {
        match self.input_type_literal_stack.last() {
            Some(Some(t)) => Some(t),
            _ => None,
        }
    }

    #[doc(hidden)]
    pub fn is_known_fragment(&self, name: &str) -> bool {
        self.fragment_names.contains(name)
    }
}
