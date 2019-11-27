use std::{collections::HashMap, sync::RwLock};

use crate::executor::FieldPath;
use crate::parser::SourcePosition;
use crate::{
    ast::Fragment,
    schema::model::{SchemaType, TypeType},
    DefaultScalarValue, ExecutionError, Executor, FieldError, ScalarRefValue, ScalarValue,
    Selection, Value, ValuesResultStream, Variables,
};
use std::sync::Arc;

/// Struct owning `Executor`'s variables
pub struct ExecutorDataVariables<'a, CtxT, S = DefaultScalarValue>
where
    CtxT: 'a,
    S: 'a,
{
    pub(crate) fragments: HashMap<&'a str, Fragment<'a, S>>,
    pub(crate) variables: Variables<S>,
    pub(crate) current_selection_set: Option<Vec<Selection<'a, S>>>,
    pub(crate) parent_selection_set: Option<Vec<Selection<'a, S>>>,
    pub(crate) current_type: TypeType<'a, S>,
    pub(crate) schema: &'a SchemaType<'a, S>,
    pub(crate) context: &'a CtxT,
    pub(crate) errors: RwLock<Vec<ExecutionError<S>>>,
    pub(crate) field_path: Arc<FieldPath<'a>>,
}

/// `Executor` wrapper to keep all `Executor`'s data
/// and `Executor` instance
pub struct SubscriptionsExecutor<'a, CtxT, S> {
    // todo: move all variables to here
    pub(crate) variables: ExecutorDataVariables<'a, CtxT, S>,
}

impl<'a, CtxT, S> SubscriptionsExecutor<'a, CtxT, S>
where
    S: std::clone::Clone,
{
    pub fn from_data(data: ExecutorDataVariables<'a, CtxT, S>) -> Self {
        Self { variables: data }
    }

    pub fn clone(&self) -> Self {
        Self {
            variables: ExecutorDataVariables {
                fragments: self.variables.fragments.clone(),
                variables: self.variables.variables.clone(),
                current_selection_set: self.variables.current_selection_set.clone(),
                parent_selection_set: self.variables.parent_selection_set.clone(),
                current_type: self.variables.current_type.clone(),
                schema: self.variables.schema.clone(),
                context: self.variables.context.clone(),
                errors: RwLock::new(vec![]),
                field_path: self.variables.field_path.clone(),
            },
        }
    }
}

//todo: do something with a lot of cloning
impl<'a, CtxT, S> SubscriptionsExecutor<'a, CtxT, S>
where
    S: Clone,
{
    #[doc(hidden)]
    pub fn type_sub_executor(
        &self,
        type_name: Option<&str>,
        selection_set: Option<Vec<Selection<'a, S>>>,
    ) -> SubscriptionsExecutor<'a, CtxT, S> {
        SubscriptionsExecutor {
            variables: ExecutorDataVariables {
                fragments: self.variables.fragments.clone(),
                variables: self.variables.variables.clone(),
                current_selection_set: selection_set,
                parent_selection_set: self.variables.current_selection_set.clone(),
                current_type: match type_name {
                    Some(type_name) => self
                        .variables
                        .schema
                        .type_by_name(type_name)
                        .expect("Type not found"),
                    None => self.variables.current_type.clone(),
                },
                schema: self.variables.schema,
                context: self.variables.context,
                errors: RwLock::new(vec![]),
                field_path: self.variables.field_path.clone(),
            },
        }
    }

    #[doc(hidden)]
    pub fn variables(&self) -> Variables<S> {
        self.variables.variables.clone()
    }

    #[doc(hidden)]
    pub fn fragment_by_name<'b>(&'b self, name: &str) -> Option<&'b Fragment<'a, S>> {
        self.variables.fragments.get(name)
    }

    #[doc(hidden)]
    pub fn field_sub_executor(
        &self,
        field_alias: &'a str,
        field_name: &'a str,
        location: SourcePosition,
        selection_set: Option<Vec<Selection<'a, S>>>,
    ) -> SubscriptionsExecutor<'a, CtxT, S> {
        SubscriptionsExecutor {
            variables: ExecutorDataVariables {
                fragments: self.variables.fragments.clone(),
                variables: self.variables.variables.clone(),
                current_selection_set: selection_set,
                parent_selection_set: self.variables.current_selection_set.clone(),
                current_type: self.variables.schema.make_type(
                    &self
                        .variables
                        .current_type
                        .innermost_concrete()
                        .field_by_name(field_name)
                        .expect("Field not found on inner type")
                        .field_type,
                ),
                schema: self.variables.schema,
                context: self.variables.context,
                errors: RwLock::new(vec![]),
                field_path: Arc::new(FieldPath::Field(
                    field_alias,
                    location,
                    Arc::clone(&self.variables.field_path),
                )),
            },
        }
    }

    #[doc(hidden)]
    pub fn context(&self) -> &'a CtxT {
        self.variables.context
    }

    #[doc(hidden)]
    pub fn schema(&self) -> &'a SchemaType<S> {
        self.variables.schema
    }

    #[doc(hidden)]
    pub fn as_executor<'e>(&'e self) -> Executor<'e, 'e, CtxT, S> {
        Executor {
            fragments: &self.variables.fragments,
            variables: &self.variables.variables,
            current_selection_set: if let Some(s) = &self.variables.current_selection_set {
                Some(&s[..])
            } else {
                None
            },
            parent_selection_set: if let Some(s) = &self.variables.parent_selection_set {
                Some(&s[..])
            } else {
                None
            },
            current_type: self.variables.current_type.clone(),
            schema: self.variables.schema,
            context: self.variables.context,
            errors: &self.variables.errors,
            field_path: Arc::clone(&self.variables.field_path),
        }
    }

    #[doc(hidden)]
    /// The current location of the executor
    pub fn location(&self) -> &SourcePosition {
        self.variables.field_path.location()
    }
}
