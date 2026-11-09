//! Resolution of synchronous fields and types during asynchronous execution.

use std::future;

use arcstr::ArcStr;

use crate::{
    BoxFuture, DefaultScalarValue, EmptyMutation, EmptySubscription, ExecutionResult, Executor,
    GraphQLType, GraphQLValue, GraphQLValueAsync, Registry, RootNode, Value, graphql,
    schema::meta::MetaType, types::base::Arguments,
};

/// Root type with fields of the `String` type and of the [`Child`] type.
///
/// Only the `marked` field is synchronous. The synchronous resolver returns `"sync"`, and the
/// asynchronous resolver returns `"async"`. A result shows which resolver the executor called.
struct Query;

impl GraphQLType<DefaultScalarValue> for Query {
    fn name(_: &Self::TypeInfo) -> Option<ArcStr> {
        Some(arcstr::literal!("Query"))
    }

    fn meta(
        info: &Self::TypeInfo,
        registry: &mut Registry<DefaultScalarValue>,
    ) -> MetaType<DefaultScalarValue> {
        let fields = &[
            registry
                .field::<String>(arcstr::literal!("marked"), &())
                .synchronous(),
            registry.field::<String>(arcstr::literal!("unmarked"), &()),
            registry.field::<Child<true>>(arcstr::literal!("syncChild"), &()),
            registry.field::<Child<false>>(arcstr::literal!("asyncChild"), &()),
        ];
        registry.build_object_type::<Self>(info, fields).into_meta()
    }
}

impl GraphQLValue<DefaultScalarValue> for Query {
    type Context = ();
    type TypeInfo = ();

    fn type_name(&self, info: &Self::TypeInfo) -> Option<ArcStr> {
        <Self as GraphQLType>::name(info)
    }

    fn resolve_field(
        &self,
        _: &Self::TypeInfo,
        field_name: &str,
        _: &Arguments<DefaultScalarValue>,
        executor: &Executor<Self::Context>,
    ) -> ExecutionResult {
        match field_name {
            "syncChild" => executor.resolve(&(), &Child::<true>),
            "asyncChild" => executor.resolve(&(), &Child::<false>),
            _ => Ok(Value::scalar("sync")),
        }
    }
}

impl GraphQLValueAsync<DefaultScalarValue> for Query {
    fn resolve_field_async<'a>(
        &'a self,
        _: &'a Self::TypeInfo,
        field_name: &'a str,
        _: Arguments<'a, DefaultScalarValue>,
        executor: Executor<'a, 'a, Self::Context>,
    ) -> BoxFuture<'a, ExecutionResult> {
        match field_name {
            "syncChild" => {
                Box::pin(async move { executor.resolve_async(&(), &Child::<true>).await })
            }
            "asyncChild" => {
                Box::pin(async move { executor.resolve_async(&(), &Child::<false>).await })
            }
            _ => Box::pin(future::ready(Ok(Value::scalar("async")))),
        }
    }
}

/// Object with a `value` field. The field is synchronous if `SYNCHRONOUS` is `true`.
///
/// The asynchronous `resolve_async()` returns `{"value": "async"}` without resolving the
/// field. The synchronous resolver of the field returns `"sync"`.
struct Child<const SYNCHRONOUS: bool>;

impl<const SYNCHRONOUS: bool> GraphQLType<DefaultScalarValue> for Child<SYNCHRONOUS> {
    fn name(_: &Self::TypeInfo) -> Option<ArcStr> {
        Some(if SYNCHRONOUS {
            arcstr::literal!("SyncChild")
        } else {
            arcstr::literal!("AsyncChild")
        })
    }

    fn meta(
        info: &Self::TypeInfo,
        registry: &mut Registry<DefaultScalarValue>,
    ) -> MetaType<DefaultScalarValue> {
        let mut field = registry.field::<String>(arcstr::literal!("value"), &());
        if SYNCHRONOUS {
            field = field.synchronous();
        }
        registry
            .build_object_type::<Self>(info, &[field])
            .into_meta()
    }
}

impl<const SYNCHRONOUS: bool> GraphQLValue<DefaultScalarValue> for Child<SYNCHRONOUS> {
    type Context = ();
    type TypeInfo = ();

    fn type_name(&self, info: &Self::TypeInfo) -> Option<ArcStr> {
        <Self as GraphQLType>::name(info)
    }

    fn resolve_field(
        &self,
        _: &Self::TypeInfo,
        _: &str,
        _: &Arguments<DefaultScalarValue>,
        _: &Executor<Self::Context>,
    ) -> ExecutionResult {
        Ok(Value::scalar("sync"))
    }
}

impl<const SYNCHRONOUS: bool> GraphQLValueAsync<DefaultScalarValue> for Child<SYNCHRONOUS> {
    fn resolve_async<'a>(
        &'a self,
        _: &'a Self::TypeInfo,
        _: Option<&'a [crate::Selection<DefaultScalarValue>]>,
        _: &'a Executor<Self::Context>,
    ) -> BoxFuture<'a, ExecutionResult> {
        Box::pin(future::ready(Ok(graphql::value!({"value": "async"}))))
    }
}

#[tokio::test]
async fn synchronous_field_uses_synchronous_resolver() {
    let schema = RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());

    let (value, errors) = crate::execute("{ marked }", None, &schema, &graphql::vars! {}, &())
        .await
        .unwrap();

    assert_eq!(errors, []);
    assert_eq!(value, graphql::value!({"marked": "sync"}));
}

#[tokio::test]
async fn unmarked_field_uses_asynchronous_resolver() {
    let schema = RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());

    let (value, errors) = crate::execute("{ unmarked }", None, &schema, &graphql::vars! {}, &())
        .await
        .unwrap();

    assert_eq!(errors, []);
    assert_eq!(value, graphql::value!({"unmarked": "async"}));
}

#[tokio::test]
async fn synchronous_type_uses_synchronous_resolver() {
    let schema = RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());

    let (value, errors) = crate::execute(
        "{ syncChild { value } }",
        None,
        &schema,
        &graphql::vars! {},
        &(),
    )
    .await
    .unwrap();

    assert_eq!(errors, []);
    assert_eq!(value, graphql::value!({"syncChild": {"value": "sync"}}));
}

#[tokio::test]
async fn asynchronous_type_uses_asynchronous_resolver() {
    let schema = RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());

    let (value, errors) = crate::execute(
        "{ asyncChild { value } }",
        None,
        &schema,
        &graphql::vars! {},
        &(),
    )
    .await
    .unwrap();

    assert_eq!(errors, []);
    assert_eq!(value, graphql::value!({"asyncChild": {"value": "async"}}));
}
