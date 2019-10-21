use std::collections::HashMap;
use crate::ast::Fragment;
use crate::{Variables, Selection, ExecutionError, Executor, DefaultScalarValue};
use crate::schema::model::{TypeType, SchemaType};
use std::sync::RwLock;
use crate::executor::FieldPath;
use crate::parser::Spanning;

/// Struct owning `Executor`'s variables
pub struct ExecutorDataVariables<'a, CtxT, S = DefaultScalarValue>
    where
        CtxT: 'a,
        S: 'a,
{
    pub(crate) fragments: HashMap<&'a str, &'a Fragment<'a, S>>,
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
    pub fn get_executor(self_ty: &'a Self) -> Executor<'a, CtxT, S> {
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
    /// Variables data
    _data: Option<ExecutorDataVariables<'a, CtxT, S>>,
}

impl<'a, CtxT, S> ExecutorData<'a, CtxT, S>
    where
        CtxT: 'a,
        S: Clone + 'a,
{
    pub fn new() -> Self {
        Self { _data: None }
    }

    pub fn set_data(&mut self, data: ExecutorDataVariables<'a, CtxT, S>) {
        self._data = Some(data);
    }

    pub fn get_executor(&'a self) -> Result<Executor<'a, CtxT, S>, ()> {
        if let Some(ref s) = self._data {
            Ok(ExecutorDataVariables::get_executor(s))
        } else {
            Err(())
        }
    }

    pub fn errors(&'a mut self) -> Option<&'a Vec<ExecutionError<S>>>
        where
            S: PartialEq,
    {
        if let Some(ref mut s) = self._data {
            //todo: maybe not unwrap
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
/// __Panics__ if `Executor` was not set.
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
    ///
    /// Variables are kept in a separate struct rather than this one
    /// because they have a hashmap referencing this struct's `fragments`
    pub(crate) executor_variables: ExecutorData<'a, CtxT, S>,

    /// Fragments vector.
    /// Needed in as a separate field because `executor_variables`
    /// contains a hashmap of references to `fragments`
    pub(crate) fragments: Vec<Spanning<Fragment<'a, S>>>,

    /// `Executor` instance
    pub(crate) executor: OptionalExecutor<'a, CtxT, S>,
}

impl<'a, CtxT, S> SubscriptionsExecutor<'a, CtxT, S>
    where
        S: std::clone::Clone,
{
    pub fn new() -> Self {
        Self {
            executor_variables: ExecutorData::new(),
            fragments: vec![],
            executor: OptionalExecutor::new(),
        }
    }

    pub fn errors(&'a mut self) -> Option<&'a Vec<ExecutionError<S>>>
        where
            S: PartialEq,
    {
        self.executor_variables.errors()
    }
}

//pub struct SubExecutorStorage<'a, CtxT, S> {
//    data: LinkedList<Executor<'a, CtxT, S>>
//}
//
//impl<'a, CtxT, S> SubExecutorStorage<'a, CtxT, S> {
//    pub fn new() -> Self {
//        Self {
//            data: LinkedList::new()
//        }
//    }
//
//    pub fn add(
//        &mut self,
//        executor: Executor<'a, CtxT, S>
//    ) -> &'a Executor<'a, CtxT, S> {
//        self.data.push_back(executor);
////        self.data.len() - 1
//        self.data.back()
//    }
//
//
//}

