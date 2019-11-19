use std::{collections::HashMap, sync::RwLock};

use crate::{ast::Fragment, executor::FieldPath, parser::Spanning, schema::model::{SchemaType, TypeType}, DefaultScalarValue, ExecutionError, Executor, Selection, Variables, Value, FieldError, ScalarRefValue, ScalarValue, FieldResult, ValuesResultStream};
use crate::parser::SourcePosition;
use crate::ast::Operation;

/// `Executor` wrapper to keep all `Executor`'s data
/// and `Executor` instance
pub struct SubscriptionsExecutor<'a, CtxT, S>
{
    pub(crate) fragments: Vec<Spanning<Fragment<'a, S>>>,
    pub(crate) operation: Spanning<Operation<'a, S>>,
    pub(crate) variables: Variables<S>,
    pub(crate) context: &'a CtxT,
}


//impl<'a, CtxT, S> SubscriptionsExecutor<'a, CtxT, S>
//where
//    S: std::clone::Clone,
//{}

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
    ) -> FieldResult<Value<ValuesResultStream<'a, S>>, S>
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
    ) -> Result<Value<ValuesResultStream<'a, S>>, FieldError<S>>
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

////todo add #[doc(hidden)] to all functions
//impl<'a, CtxT, S> SubscriptionsExecutor<'a, CtxT, S> {
//
//    //#[doc(hidden)]
//    pub fn type_sub_executor(
//        &self,
//        type_name: Option<&'a str>,
//        selection_set: Option<Vec<Selection<S>>>,
//    ) -> SubscriptionsExecutor<CtxT, S> {
//        SubscriptionsExecutor {
//            variables: ExecutorDataVariables {
//                fragments: self.variables.fragments,
//                variables: self.variables.variables,
//                current_selection_set: selection_set,
//                parent_selection_set: self.variables.current_selection_set,
//                current_type: match type_name {
//                    Some(type_name) => self.variables.schema.type_by_name(type_name).expect("Type not found"),
//                    None => self.variables.current_type.clone(),
//                },
//                schema: self.variables.schema,
//                context: self.variables.context,
//                errors: self.variables.errors,
//                field_path: self.variables.field_path.clone(),
//            }
//        }
//    }
//
//
////    #[doc(hidden)]
//    pub fn variables(&self) -> Variables<S> {
//        self.variables.variables.clone()
//    }
//
//    pub fn fragment_by_name(&'a self, name: &str) -> Option<&'a Fragment<'a, S>> {
//        self.variables.fragments.get(name)
//    }
//
//    pub fn field_sub_executor(
//        &self,
//        field_alias: &'a str,
//        field_name: &'a str,
//        location: SourcePosition,
//        selection_set: Option<Vec<Selection<'a, S>>>,
//    ) -> SubscriptionsExecutor<'a, CtxT, S> {
//        SubscriptionsExecutor {
//            variables: ExecutorDataVariables {
//                fragments: self.variables.fragments,
//                variables: self.variables.variables,
//                current_selection_set: selection_set,
//                parent_selection_set: self.variables.current_selection_set,
//                current_type: self.variables.schema.make_type(
//                    &self
//                        .variables
//                        .current_type
//                        .innermost_concrete()
//                        .field_by_name(field_name)
//                        .expect("Field not found on inner type")
//                        .field_type,
//                ),
//                schema: self.variables.schema,
//                context: self.variables.context,
//                errors: self.variables.errors,
//                field_path: FieldPath::Field(field_alias, location, &self.variables.field_path),
//            }
//        }
//    }
//
//    pub fn context(&self) -> &'a CtxT {
//        self.variables.context
//    }
//
//    pub fn schema(&self) -> &'a SchemaType<S> {
//        self.variables.schema
//    }
//}