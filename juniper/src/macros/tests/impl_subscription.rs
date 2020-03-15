use super::util;
use crate::{graphql_value, EmptyMutation, RootNode};
use std::pin::Pin;
use serde::export::PhantomData;

#[derive(Default)]
struct Context {
    flag1: bool,
}

impl crate::Context for Context {}

struct WithLifetime<'a> {
    value: &'a str,
}

#[crate::graphql_object_internal(Context=Context)]
impl<'a> WithLifetime<'a> {
    fn value(&'a self) -> &'a str {
        self.value
    }
}

struct WithContext;

#[crate::graphql_object_internal(Context=Context)]
impl WithContext {
    fn ctx(ctx: &Context) -> bool {
        ctx.flag1
    }
}

struct Query;

#[crate::graphql_object_internal(
    Context = Context,
)]
impl Query {}

type Stream<I> = Pin<Box<dyn futures::Stream<Item = I> + Send>>;

struct Subscription {
    b: bool,
}

#[crate::graphql_subscription_internal(
    scalar = crate::DefaultScalarValue,
    name = "Subscription",
    context = Context,
)]
/// Subscription Description.
impl Subscription {
    #[graphql(description = "With Self Description")]
    async fn with_self(&self) -> Stream<bool> {
        let b = self.b;
        Box::pin(futures::stream::once(
            async move { b }
        ))
    }

    async fn independent() -> Stream<i32> {
        Box::pin(futures::stream::once(
            async { 100 }
        ))
    }

    async fn with_executor(_exec: &Executor<Context>) -> Stream<bool> {
        Box::pin(futures::stream::once(
            async { true }
        ))
    }

    async fn with_executor_and_self(&self, _exec: &Executor<Context>) -> Stream<bool> {
        Box::pin(futures::stream::once(
            async { true }
        ))
    }

    async fn with_context(_context: &Context) -> Stream<bool> {
        Box::pin(futures::stream::once(
            async { true }
        ))
    }

    async fn with_context_and_self(&self, _context: &Context) -> Stream<bool> {
        Box::pin(futures::stream::once(
            async { true }
        ))
    }

    #[graphql(name = "renamed")]
    async fn has_custom_name() -> Stream<bool> {
        Box::pin(futures::stream::once(
            async { true }
        ))

    }

    #[graphql(description = "attr")]
    async fn has_description_attr() -> Stream<bool> {
        Box::pin(futures::stream::once(
            async { true }
        ))
    }

    /// Doc description
    async fn has_description_doc_comment() -> Stream<bool> {
        Box::pin(futures::stream::once(
            async { true }
        ))
    }

    async fn has_argument(arg1: bool) -> Stream<bool> {
        Box::pin(futures::stream::once(
            async move { arg1 }
        ))

    }

    #[graphql(arguments(default_arg(default = true)))]
    async fn default_argument(default_arg: bool) -> Stream<bool> {
        Box::pin(futures::stream::once(
            async move { default_arg }
        ))
    }

    #[graphql(arguments(arg(description = "my argument description")))]
    async fn arg_with_description(arg: bool) -> Stream<bool> {
        Box::pin(futures::stream::once(
            async move { arg }
        ))
    }

    async fn with_context_child(&self) -> Stream<WithContext> {
        Box::pin(futures::stream::once(
            async { WithContext }
        ))
    }

// todo: support lifetimes (?)
//    async fn with_lifetime_child(&self) -> Stream<WithLifetime<'res>> {
//        Box::pin(futures::stream::once(
//            async { WithLifetime { value: "blub" } }
//        ))
//    }

    async fn with_mut_arg(mut arg: bool) -> Stream<bool> {
        if arg {
            arg = !arg;
        }

        Box::pin(futures::stream::once(
            async move { arg }
        ))
    }

    async fn without_type_alias() -> Pin<Box<dyn futures::Stream<Item = bool> + Send>> {
        Box::pin(futures::stream::once(
            async { true }
        ))
    }
}

//#[tokio::test]
//async fn object_introspect() {
//    let res = util::run_info_query::<Query, Mutation, Subscription, Context>("Query").await;
//    assert_eq!(
//        res,
//        crate::graphql_value!({
//            "name": "Query",
//            "description": "Query Description.",
//            "fields": [
//                {
//                    "name": "withSelf",
//                    "description": "With Self Description",
//                    "args": [],
//                },
//                {
//                    "name": "independent",
//                    "description": None,
//                    "args": [],
//                },
//                {
//                    "name": "withExecutor",
//                    "description": None,
//                    "args": [],
//                },
//                {
//                    "name": "withExecutorAndSelf",
//                    "description": None,
//                    "args": [],
//                },
//                {
//                    "name": "withContext",
//                    "description": None,
//                    "args": [],
//                },
//                {
//                    "name": "withContextAndSelf",
//                    "description": None,
//                    "args": [],
//                },
//                {
//                    "name": "renamed",
//                    "description": None,
//                    "args": [],
//                },
//                {
//                    "name": "hasDescriptionAttr",
//                    "description": "attr",
//                    "args": [],
//                },
//                {
//                    "name": "hasDescriptionDocComment",
//                    "description": "Doc description",
//                    "args": [],
//                },
//                {
//                    "name": "hasArgument",
//                    "description": None,
//                    "args": [
//                        {
//                            "name": "arg1",
//                            "description": None,
//                            "type": {
//                                "name": None,
//                            },
//                        }
//                    ],
//                },
//                {
//                    "name": "defaultArgument",
//                    "description": None,
//                    "args": [
//                        {
//                            "name": "defaultArg",
//                            "description": None,
//                            "type": {
//                                "name": "Boolean",
//                            },
//                        }
//                    ],
//                },
//                {
//                    "name": "argWithDescription",
//                    "description": None,
//                    "args": [
//                        {
//                            "name": "arg",
//                            "description": "my argument description",
//                            "type": {
//                                "name": None
//                            },
//                        }
//                    ],
//                },
//                {
//                    "name": "withContextChild",
//                    "description": None,
//                    "args": [],
//                },
//                {
//                    "name": "withLifetimeChild",
//                    "description": None,
//                    "args": [],
//                },
//                {
//                    "name": "withMutArg",
//                    "description": None,
//                    "args": [
//                        {
//                            "name": "arg",
//                            "description": None,
//                            "type": {
//                                "name": None,
//                            },
//                        }
//                    ],
//                },
//            ]
//        })
//    );
//}
//
//#[tokio::test]
//async fn object_query() {
//    let doc = r#"
//    query {
//        withSelf
//        independent
//        withExecutor
//        withExecutorAndSelf
//        withContext
//        withContextAndSelf
//        renamed
//        hasArgument(arg1: true)
//        defaultArgument
//        argWithDescription(arg: true)
//        withContextChild {
//            ctx
//        }
//        withLifetimeChild {
//            value
//        }
//        withMutArg(arg: true)
//    }
//    "#;
//    let schema = RootNode::new(
//        Query,
//        EmptyMutation::<Context>::new(),
//        Subscription,
//    );
//    let vars = std::collections::HashMap::new();
//
//    let (result, errs) = crate::execute(
//        doc,
//        None,
//        &schema,
//        &vars,
//        &Context {
//            flag1: true
//        })
//        .await
//        .expect("Execution failed");
//
//    assert_eq!(errs, []);
//    assert_eq!(
//        result,
//        graphql_value!({
//            "withSelf": true,
//            "independent": 100,
//            "withExecutor": true,
//            "withExecutorAndSelf": true,
//            "withContext": true,
//            "withContextAndSelf": true,
//            "renamed": true,
//            "hasArgument": true,
//            "defaultArgument": true,
//            "argWithDescription": true,
//            "withContextChild": { "ctx": true },
//            "withLifetimeChild": { "value": "blub" },
//            "withMutArg": false,
//        })
//    );
//}
