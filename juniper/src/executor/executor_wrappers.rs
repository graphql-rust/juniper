use std::{collections::HashMap, sync::RwLock};

use crate::parser::SourcePosition;
use crate::{
    ast::Fragment,
    schema::model::{SchemaType, TypeType},
    DefaultScalarValue, ExecutionError, Executor, FieldError, ScalarRefValue,
    ScalarValue, Selection, Value, ValuesResultStream, Variables,
};
use crate::executor::FieldPath;

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
    pub(crate) field_path: FieldPath<'a>,
}

/// `Executor` wrapper to keep all `Executor`'s data
/// and `Executor` instance
pub struct SubscriptionsExecutor<'a, CtxT, S> {
    pub(crate) variables: ExecutorDataVariables<'a, CtxT, S>,
}

impl<'a, CtxT, S> SubscriptionsExecutor<'a, CtxT, S>
where
    S: std::clone::Clone,
{

    pub fn from_data(data: ExecutorDataVariables<'a, CtxT, S>) -> Self {
        Self { variables: data }
    }
}

impl<'a, CtxT, S> SubscriptionsExecutor<'a, CtxT, S>
where
    S: ScalarValue,
    for<'b> &'b S: ScalarRefValue<'b>,
{
    /// Resolve a single arbitrary value into a return stream
    ///
    /// If the field fails to resolve, `null` will be returned.
    #[cfg(feature = "async")]
    pub async fn resolve_into_stream<'s, 'i, 'v, T>(
        self,
        info: &'i T::TypeInfo,
        value: &'v T,
    ) -> Result<Value<ValuesResultStream<'a, S>>, ExecutionError<S>>
    where
        'i: 'a,
        'v: 'a,
        's: 'a,
        T: crate::GraphQLSubscriptionType<S, Context = CtxT> + Send + Sync,
        T::TypeInfo: Send + Sync,
        CtxT: Send + Sync,
        S: Send + Sync + 'static,
    {
        self.subscribe(info, value).await
    }

    /// Resolve a single arbitrary value into `Value<ValuesStream>`
    #[cfg(feature = "async")]
    pub async fn subscribe<'s, 'i, 'v, T>(
        self,
        info: &'i T::TypeInfo,
        value: &'v T,
    ) -> Result<Value<ValuesResultStream<'a, S>>, ExecutionError<S>>
    where
        'i: 'a,
        'v: 'a,
        's: 'a,
        T: crate::GraphQLSubscriptionType<S, Context = CtxT>,
        T::TypeInfo: Send + Sync,
        CtxT: Send + Sync,
        S: Send + Sync + 'static,
    {
        value.resolve_into_stream(info, self).await
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
                //  todo: not assume that field path is root
                //field_path: FieldPath::Field(field_alias, location, &self.variables.field_path),
                field_path: FieldPath::Root(location),
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
    pub fn as_executor<'e>(&'e self) -> Executor<'e, CtxT, S> {
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
            field_path: self.variables.field_path.clone(),
        }
    }

    #[doc(hidden)]
    /// Generate an error to the execution engine at the current executor location
    pub fn generate_error(&self, error: FieldError<S>) -> ExecutionError<S> {
        self.generate_error_at(error, self.location().clone())
    }

    #[doc(hidden)]
    /// Add an error to the execution engine at a specific location
    pub fn generate_error_at(
        &self,
        error: FieldError<S>,
        location: SourcePosition,
    ) -> ExecutionError<S> {
        let mut path = Vec::new();
        self.variables.field_path.construct_path(&mut path);

        ExecutionError {
            location,
            path,
            error,
        }
    }

    #[doc(hidden)]
    /// The current location of the executor
    pub fn location(&self) -> &SourcePosition {
        self.variables.field_path.location()
    }
}
