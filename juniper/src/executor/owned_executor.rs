use std::{collections::HashMap, sync::RwLock};

use crate::{
    ExecutionError, Executor, Selection, Variables,
    ast::Fragment,
    executor::FieldPath,
    parser::SourcePosition,
    schema::model::{SchemaType, TypeType},
};

/// [`Executor`] owning all its variables. Can be used after [`Executor`] was
/// destroyed.
pub struct OwnedExecutor<'a, CtxT, S> {
    pub(super) fragments: HashMap<&'a str, Fragment<'a, S>>,
    pub(super) variables: Variables<S>,
    pub(super) current_selection_set: Option<Vec<Selection<'a, S>>>,
    pub(super) parent_selection_set: Option<Vec<Selection<'a, S>>>,
    pub(super) current_type: TypeType<'a, S>,
    pub(super) schema: &'a SchemaType<S>,
    pub(super) context: &'a CtxT,
    pub(super) errors: RwLock<Vec<ExecutionError<S>>>,

    /// Response names from the root of the operation to the current field.
    pub(super) field_path: Vec<&'a str>,

    /// Location of the current field.
    pub(super) location: SourcePosition,
}

impl<CtxT, S> Clone for OwnedExecutor<'_, CtxT, S>
where
    S: Clone,
{
    fn clone(&self) -> Self {
        Self {
            fragments: self.fragments.clone(),
            variables: self.variables.clone(),
            current_selection_set: self.current_selection_set.clone(),
            parent_selection_set: self.parent_selection_set.clone(),
            current_type: self.current_type,
            schema: self.schema,
            context: self.context,
            errors: RwLock::new(vec![]),
            field_path: self.field_path.clone(),
            location: self.location,
        }
    }
}

impl<'a, CtxT, S> OwnedExecutor<'a, CtxT, S> {
    #[doc(hidden)]
    pub fn as_executor(&self) -> Executor<'_, '_, CtxT, S> {
        Executor {
            fragments: &self.fragments,
            variables: &self.variables,
            current_selection_set: self.current_selection_set.as_deref(),
            parent_selection_set: self.parent_selection_set.as_deref(),
            current_type: self.current_type,
            schema: self.schema,
            context: self.context,
            errors: &self.errors,
            field_path: FieldPath::Owned(&self.field_path, self.location),
            // Subscriptions execute their fields normally.
            serial: false,
        }
    }

    #[doc(hidden)]
    pub fn context(&self) -> &'a CtxT {
        self.context
    }
}
