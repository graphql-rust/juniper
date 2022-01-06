use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use crate::{
    ast::Fragment,
    executor::FieldPath,
    parser::SourcePosition,
    schema::model::{SchemaType, TypeType},
    ExecutionError, Executor, Selection, Variables,
};

/// [`Executor`] owning all its variables. Can be used after [`Executor`] was
/// destroyed.
pub struct OwnedExecutor<'a, CtxT, S> {
    pub(super) fragments: HashMap<&'a str, Fragment<'a, S>>,
    pub(super) variables: Variables<S>,
    pub(super) current_selection_set: Option<Vec<Selection<'a, S>>>,
    pub(super) parent_selection_set: Option<Vec<Selection<'a, S>>>,
    pub(super) current_type: TypeType<'a, S>,
    pub(super) schema: &'a SchemaType<'a, S>,
    pub(super) context: &'a CtxT,
    pub(super) errors: RwLock<Vec<ExecutionError<S>>>,
    pub(super) field_path: Arc<FieldPath<'a>>,
}

impl<'a, CtxT, S> Clone for OwnedExecutor<'a, CtxT, S>
where
    S: Clone,
{
    fn clone(&self) -> Self {
        Self {
            fragments: self.fragments.clone(),
            variables: self.variables.clone(),
            current_selection_set: self.current_selection_set.clone(),
            parent_selection_set: self.parent_selection_set.clone(),
            current_type: self.current_type.clone(),
            schema: self.schema,
            context: self.context,
            errors: RwLock::new(vec![]),
            field_path: self.field_path.clone(),
        }
    }
}

impl<'a, CtxT, S> OwnedExecutor<'a, CtxT, S>
where
    S: Clone,
{
    #[doc(hidden)]
    #[must_use]
    pub fn type_sub_executor(
        &self,
        type_name: Option<&str>,
        selection_set: Option<Vec<Selection<'a, S>>>,
    ) -> OwnedExecutor<'a, CtxT, S> {
        OwnedExecutor {
            fragments: self.fragments.clone(),
            variables: self.variables.clone(),
            current_selection_set: selection_set,
            parent_selection_set: self.current_selection_set.clone(),
            current_type: match type_name {
                Some(type_name) => self.schema.type_by_name(type_name).expect("Type not found"),
                None => self.current_type.clone(),
            },
            schema: self.schema,
            context: self.context,
            errors: RwLock::new(vec![]),
            field_path: self.field_path.clone(),
        }
    }

    #[doc(hidden)]
    pub fn variables(&self) -> Variables<S> {
        self.variables.clone()
    }

    #[doc(hidden)]
    #[must_use]
    pub fn field_sub_executor(
        &self,
        field_alias: &'a str,
        field_name: &'a str,
        location: SourcePosition,
        selection_set: Option<Vec<Selection<'a, S>>>,
    ) -> OwnedExecutor<'a, CtxT, S> {
        OwnedExecutor {
            fragments: self.fragments.clone(),
            variables: self.variables.clone(),
            current_selection_set: selection_set,
            parent_selection_set: self.current_selection_set.clone(),
            current_type: self.schema.make_type(
                &self
                    .current_type
                    .innermost_concrete()
                    .field_by_name(field_name)
                    .expect("Field not found on inner type")
                    .field_type,
            ),
            schema: self.schema,
            context: self.context,
            errors: RwLock::new(vec![]),
            field_path: Arc::new(FieldPath::Field(
                field_alias,
                location,
                Arc::clone(&self.field_path),
            )),
        }
    }

    #[doc(hidden)]
    pub fn as_executor(&self) -> Executor<'_, '_, CtxT, S> {
        Executor {
            fragments: &self.fragments,
            variables: &self.variables,
            current_selection_set: self.current_selection_set.as_deref(),
            parent_selection_set: self.parent_selection_set.as_deref(),
            current_type: self.current_type.clone(),
            schema: self.schema,
            context: self.context,
            errors: &self.errors,
            field_path: Arc::clone(&self.field_path),
        }
    }
}

impl<'a, CtxT, S> OwnedExecutor<'a, CtxT, S> {
    #[doc(hidden)]
    pub fn fragment_by_name<'b>(&'b self, name: &str) -> Option<&'b Fragment<'a, S>> {
        self.fragments.get(name)
    }

    #[doc(hidden)]
    pub fn context(&self) -> &'a CtxT {
        self.context
    }

    #[doc(hidden)]
    pub fn schema(&self) -> &'a SchemaType<S> {
        self.schema
    }

    #[doc(hidden)]
    pub fn location(&self) -> &SourcePosition {
        self.field_path.location()
    }
}
