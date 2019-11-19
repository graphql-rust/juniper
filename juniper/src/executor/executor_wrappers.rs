use std::{collections::HashMap, sync::RwLock};

use crate::{ast::Fragment, executor::FieldPath, parser::Spanning, schema::model::{SchemaType, TypeType}, DefaultScalarValue, ExecutionError, Executor, Selection, Variables, Value, FieldError, ScalarRefValue, ScalarValue, FieldResult, ValuesResultStream};
use crate::parser::SourcePosition;
use std::sync::Arc;

/// Struct owning `Executor`'s variables
pub struct ExecutorDataVariables<'a, CtxT, S = DefaultScalarValue>
where
    CtxT: 'a,
    S: 'a,
{
    pub(crate) fragments: Arc<HashMap<&'a str, Fragment<'a, S>>>,
    pub(crate) variables: Variables<S>,
    pub(crate) current_selection_set: Option<Vec<Selection<'a, S>>>,
    pub(crate) parent_selection_set: Option<Vec<Selection<'a, S>>>,
    pub(crate) current_type: TypeType<'a, S>,
    pub(crate) schema: &'a SchemaType<'a, S>,
    pub(crate) context: &'a CtxT,
    pub(crate) errors: RwLock<Vec<ExecutionError<S>>>,
    pub(crate) field_path: FieldPath<'a>,
}

impl<'a, CtxT, S> ExecutorDataVariables<'a, CtxT, S>
where
    S: Clone,
{
//    /// Create new executor using inner variables
//    pub fn create_executor(self_ty: &'a Self) -> Executor<'a, CtxT, S> {
//        Executor {
//            fragments: &self_ty.fragments,
//            variables: &self_ty.variables,
//            current_selection_set: if let Some(s) = &self_ty.current_selection_set {
//                Some(&s[..])
//            } else {
//                None
//            },
//            parent_selection_set: if let Some(s) = &self_ty.parent_selection_set {
//                Some(&s[..])
//            } else {
//                None
//            },
//            current_type: self_ty.current_type.clone(),
//            schema: Arc::new(self_ty.schema),
//            context: self_ty.context,
//            errors: &self_ty.errors,
//            field_path: self_ty.field_path.clone(),
//        }
//    }
}

/// `ExecutorDataVariables` wrapper
pub(crate) struct ExecutorData<'a, CtxT, S = DefaultScalarValue>
where
    CtxT: 'a,
    S: Clone + 'a,
{
    /// Struct owning variables
    _data: Option<ExecutorDataVariables<'a, CtxT, S>>,
}

impl<'a, CtxT, S> ExecutorData<'a, CtxT, S>
where
    CtxT: 'a,
    S: Clone + 'a,
{
    /// Create new empty `ExecutorData`
    pub fn new() -> Self {
        Self { _data: None }
    }

    /// Set executor's data
    pub fn set_data(&mut self, data: ExecutorDataVariables<'a, CtxT, S>) {
        self._data = Some(data);
    }

//    /// Create new executor using inner data
//    pub fn create_executor(&'a self) -> Result<Executor<'a, CtxT, S>, ()> {
//        if let Some(ref s) = self._data {
//            Ok(ExecutorDataVariables::create_executor(s))
//        } else {
//            Err(())
//        }
//    }

    /// Get sorted errors vector
    pub fn errors(&'a mut self) -> Option<&'a Vec<ExecutionError<S>>>
    where
        S: PartialEq,
    {
        if let Some(ref mut s) = self._data {
            let errors = match s.errors.get_mut() {
                Ok(e) => e,
                Err(_) => return None,
            };
            errors.sort();
            Some(errors)
        } else {
            None
        }
    }
}

/// `Executor` which can be set later.
/// __Panics__ on `Deref` if `Executor` was not set.
pub(crate) struct OptionalExecutor<'a, CtxT, S = DefaultScalarValue>
where
    CtxT: 'a,
    S: 'a,
{
    /// `Executor` instance
    executor: Option<Executor<'a, CtxT, S>>,
}

impl<'a, CtxT, S> OptionalExecutor<'a, CtxT, S>
where
    CtxT: 'a,
    S: 'a,
{
    /// Create new `OptionalExecutor`
    pub fn new() -> Self {
        Self { executor: None }
    }

    /// Set `Executor` to dereference
    pub fn set(&mut self, e: Executor<'a, CtxT, S>) {
        self.executor = Some(e);
    }
}

impl<'a, CtxT, S> std::ops::Deref for OptionalExecutor<'a, CtxT, S>
where
    CtxT: 'a,
    S: 'a,
{
    type Target = Executor<'a, CtxT, S>;

    fn deref(&self) -> &Self::Target {
        if let Some(ref e) = self.executor {
            e
        } else {
            panic!("Tried dereferencing OptionalExecutor which was not set")
        }
    }
}


/// `Executor` wrapper to keep all `Executor`'s data
/// and `Executor` instance
pub struct SubscriptionsExecutor<'a, CtxT, S>
{
    pub(crate) variables: ExecutorDataVariables<'a, CtxT, S>,

//    /// Fragments vector.
//    /// Needed in as a separate field because `executor_variables`
//    /// contains a hashmap of references to `fragments`
//    pub(crate) fragments: Vec<Spanning<Fragment<'a, S>>>,
//
//    /// `Executor` instance
//    pub(crate) executor: OptionalExecutor<'a, CtxT, S>,
}


impl<'a, CtxT, S> SubscriptionsExecutor<'a, CtxT, S>
where
    S: std::clone::Clone,
{
//    /// Create new empty `SubscriptionsExecutor`
//    pub fn new() -> Self {
//        Self {
//            executor_variables: ExecutorData::new(),
//            fragments: vec![],
//            executor: OptionalExecutor::new(),
//        }
//    }

    pub fn from_data(data: ExecutorDataVariables<'a, CtxT, S>) -> Self {
        Self {
            variables: data,
        }
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
        value
            .resolve_into_stream(info,self).await
    }
}

//todo add #[doc(hidden)] to all functions
impl<'a, CtxT, S> SubscriptionsExecutor<'a, CtxT, S>
where S: Clone, {

    //#[doc(hidden)]
    pub fn type_sub_executor(
        &self,
        type_name: Option<&'a str>,
        selection_set: Option<Vec<Selection<'a, S>>>,
    ) -> SubscriptionsExecutor<'a, CtxT, S> {
        SubscriptionsExecutor {
            variables: ExecutorDataVariables {
                fragments: self.variables.fragments.clone(),
                variables: self.variables.variables.clone(),
                current_selection_set: selection_set,
                parent_selection_set: self.variables.current_selection_set.clone(),
                current_type: match type_name {
                    Some(type_name) => self.variables.schema.type_by_name(type_name).expect("Type not found"),
                    None => self.variables.current_type.clone(),
                },
                schema: self.variables.schema,
                context: self.variables.context,
                errors: RwLock::new(vec![]),
                field_path: self.variables.field_path.clone(),
            }
        }
    }


//    #[doc(hidden)]
    pub fn variables(&self) -> Variables<S> {
        self.variables.variables.clone()
    }

    pub fn fragment_by_name(&'a self, name: &str) -> Option<&'a Fragment<'a, S>> {
        self.variables.fragments.get(name)
    }

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
                //todo:
//                field_path: FieldPath::Field(field_alias, location, &self.variables.field_path),
                field_path: FieldPath::Root(location),
            }
        }
    }

    pub fn context(&self) -> &'a CtxT {
        self.variables.context
    }

    pub fn schema(&self) -> &'a SchemaType<S> {
        self.variables.schema
    }

    /// Generate an error to the execution engine at the current executor location
    pub fn generate_error(&self, error: FieldError<S>) -> ExecutionError<S> {
        self.generate_error_at(error, self.location().clone())
    }

    /// Add an error to the execution engine at a specific location
    pub fn generate_error_at(
        &self,
        error: FieldError<S>,
        location: SourcePosition
    ) -> ExecutionError<S> {
        let mut path = Vec::new();
        self.variables.field_path.construct_path(&mut path);

        ExecutionError {
            location,
            path,
            error,
        }
    }

    /// The current location of the executor
    pub fn location(&self) -> &SourcePosition {
        self.variables.field_path.location()
    }
}