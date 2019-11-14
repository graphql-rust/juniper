use std::{collections::HashMap, sync::RwLock};

use crate::{
    ast::Fragment, executor::FieldPath, parser::Spanning,
    schema::model::{SchemaType, TypeType}, DefaultScalarValue,
    ExecutionError, Executor, Selection, Variables,
    Value, ValuesStream, FieldError, ScalarRefValue, ScalarValue
};

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

impl<'a, CtxT, S> ExecutorDataVariables<'a, CtxT, S>
where
    S: Clone,
{
    /// Create new executor using inner variables
    pub fn create_executor(self_ty: &'a Self) -> Executor<'a, CtxT, S> {
        Executor {
            fragments: &self_ty.fragments,
            variables: &self_ty.variables,
            current_selection_set: if let Some(s) = &self_ty.current_selection_set {
                Some(&s[..])
            } else {
                None
            },
            parent_selection_set: if let Some(s) = &self_ty.parent_selection_set {
                Some(&s[..])
            } else {
                None
            },
            current_type: self_ty.current_type.clone(),
            schema: self_ty.schema,
            context: self_ty.context,
            errors: &self_ty.errors,
            field_path: self_ty.field_path.clone(),
        }
    }
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

    /// Create new executor using inner data
    pub fn create_executor(&'a self) -> Result<Executor<'a, CtxT, S>, ()> {
        if let Some(ref s) = self._data {
            Ok(ExecutorDataVariables::create_executor(s))
        } else {
            Err(())
        }
    }

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
where
    S: std::clone::Clone,
{
    /// Keeps ownership of all `Executor`'s variables
    /// because `Executor` only keeps references
    pub(crate) executor_variables: ExecutorDataVariables<'a, CtxT, S>,

    /// `Executor` instance
    pub(crate) executor: OptionalExecutor<'a, CtxT, S>,
}


impl<'a, CtxT, S> SubscriptionsExecutor<'a, CtxT, S>
where
    S: std::clone::Clone,
{
    /// Create new empty `SubscriptionsExecutor`
    pub fn new(executor_variables: ExecutorDataVariables<'a, CtxT, S>) -> Self {
        let mut x = Self {
            executor_variables,
            executor: OptionalExecutor::new(),
        };
//        x.executor.set(
//            ExecutorDataVariables::create_executor(
//                x.executor_variables
//            )
//        );

        x
    }
}


impl<'a, CtxT, S> SubscriptionsExecutor<'a, CtxT, S>
where
    S: std::clone::Clone,
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
    ) -> Value<ValuesStream<'a, S>>
        where
            'i: 'a,
            'v: 'a,
            's: 'a,
            T: crate::GraphQLSubscriptionTypeAsync<S, Context = CtxT> + Send + Sync,
            T::TypeInfo: Send + Sync,
            CtxT: Send + Sync,
            S: Send + Sync + 'static,
    {
        match self.subscribe(info, value).await {
            Ok(v) => v,
            Err(e) => {
//                self.push_error(e);
                Value::Null
            }
        }
    }

    /// Resolve a single arbitrary value into `Value<ValuesStream>`
    #[cfg(feature = "async")]
    pub async fn subscribe<'s, 'i, 'v, T>(
        self,
        info: &'i T::TypeInfo,
        value: &'v T,
    ) -> Result<Value<ValuesStream<'a, S>>, FieldError<S>>
        where
            'i: 'a,
            'v: 'a,
            's: 'a,
            T: crate::GraphQLSubscriptionTypeAsync<S, Context = CtxT>,
            T::TypeInfo: Send + Sync,
            CtxT: Send + Sync,
            S: Send + Sync + 'static,
    {
        Ok(value
            .resolve_into_stream(
                info,
//                self.executor.current_selection_set,
                self)
            .await)
    }

}