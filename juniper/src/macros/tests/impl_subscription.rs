use std::pin::Pin;

use futures::StreamExt as _;

use crate::{graphql_value, EmptyMutation, RootNode, Value};

use super::util;

#[derive(Default)]
struct Context {
    flag1: bool,
}

impl crate::Context for Context {}

struct WithLifetime<'a> {
    value: &'a str,
}

#[crate::graphql_object(Context = Context)]
impl<'a> WithLifetime<'a> {
    fn value(&'a self) -> &'a str {
        self.value
    }
}

struct WithContext;

#[crate::graphql_object(Context = Context)]
impl WithContext {
    fn ctx(ctx: &Context) -> bool {
        ctx.flag1
    }
}

#[derive(Default)]
struct Query;

#[crate::graphql_object(
    Context = Context,
)]
impl Query {
    fn empty() -> bool {
        true
    }
}

#[derive(Default)]
struct Mutation;

#[crate::graphql_object(context = Context)]
impl Mutation {
    fn empty() -> bool {
        true
    }
}

type Stream<I> = Pin<Box<dyn futures::Stream<Item = I> + Send>>;

#[derive(Default)]
struct Subscription {
    b: bool,
}

#[crate::graphql_subscription(
    scalar = crate::DefaultScalarValue,
    name = "Subscription",
    context = Context,
)]
/// Subscription Description.
impl Subscription {
    #[graphql(description = "With Self Description")]
    async fn with_self(&self) -> Stream<bool> {
        let b = self.b;
        Box::pin(futures::stream::once(async move { b }))
    }

    async fn independent() -> Stream<i32> {
        Box::pin(futures::stream::once(async { 100 }))
    }

    async fn with_executor(_exec: &Executor<Context>) -> Stream<bool> {
        Box::pin(futures::stream::once(async { true }))
    }

    async fn with_executor_and_self(&self, _exec: &Executor<Context>) -> Stream<bool> {
        Box::pin(futures::stream::once(async { true }))
    }

    async fn with_context(_context: &Context) -> Stream<bool> {
        Box::pin(futures::stream::once(async { true }))
    }

    async fn with_context_and_self(&self, _context: &Context) -> Stream<bool> {
        Box::pin(futures::stream::once(async { true }))
    }

    #[graphql(name = "renamed")]
    async fn has_custom_name() -> Stream<bool> {
        Box::pin(futures::stream::once(async { true }))
    }

    #[graphql(description = "attr")]
    async fn has_description_attr() -> Stream<bool> {
        Box::pin(futures::stream::once(async { true }))
    }

    /// Doc description
    async fn has_description_doc_comment() -> Stream<bool> {
        Box::pin(futures::stream::once(async { true }))
    }

    async fn has_argument(arg1: bool) -> Stream<bool> {
        Box::pin(futures::stream::once(async move { arg1 }))
    }

    #[graphql(arguments(default_arg(default = true)))]
    async fn default_argument(default_arg: bool) -> Stream<bool> {
        Box::pin(futures::stream::once(async move { default_arg }))
    }

    #[graphql(arguments(arg(description = "my argument description")))]
    async fn arg_with_description(arg: bool) -> Stream<bool> {
        Box::pin(futures::stream::once(async move { arg }))
    }

    async fn with_context_child(&self) -> Stream<WithContext> {
        Box::pin(futures::stream::once(async { WithContext }))
    }

    async fn with_implicit_lifetime_child(&self) -> Stream<WithLifetime<'_>> {
        Box::pin(futures::stream::once(async {
            WithLifetime { value: "blub" }
        }))
    }

    async fn with_mut_arg(mut arg: bool) -> Stream<bool> {
        if arg {
            arg = !arg;
        }

        Box::pin(futures::stream::once(async move { arg }))
    }

    async fn without_type_alias() -> Pin<Box<dyn futures::Stream<Item = &str> + Send>> {
        Box::pin(futures::stream::once(async { "abc" }))
    }
}

#[tokio::test]
async fn object_introspect() {
    let res = util::run_info_query::<Query, Mutation, Subscription>("Subscription").await;
    assert_eq!(
        res,
        crate::graphql_value!({
            "name": "Subscription",
            "description": "Subscription Description.",
            "fields": [
                {
                    "name": "withSelf",
                    "description": "With Self Description",
                    "args": [],
                },
                {
                    "name": "independent",
                    "description": None,
                    "args": [],
                },
                {
                    "name": "withExecutor",
                    "description": None,
                    "args": [],
                },
                {
                    "name": "withExecutorAndSelf",
                    "description": None,
                    "args": [],
                },
                {
                    "name": "withContext",
                    "description": None,
                    "args": [],
                },
                {
                    "name": "withContextAndSelf",
                    "description": None,
                    "args": [],
                },
                {
                    "name": "renamed",
                    "description": None,
                    "args": [],
                },
                {
                    "name": "hasDescriptionAttr",
                    "description": "attr",
                    "args": [],
                },
                {
                    "name": "hasDescriptionDocComment",
                    "description": "Doc description",
                    "args": [],
                },
                {
                    "name": "hasArgument",
                    "description": None,
                    "args": [
                        {
                            "name": "arg1",
                            "description": None,
                            "type": {
                                "name": None,
                            },
                        }
                    ],
                },
                {
                    "name": "defaultArgument",
                    "description": None,
                    "args": [
                        {
                            "name": "defaultArg",
                            "description": None,
                            "type": {
                                "name": "Boolean",
                            },
                        }
                    ],
                },
                {
                    "name": "argWithDescription",
                    "description": None,
                    "args": [
                        {
                            "name": "arg",
                            "description": "my argument description",
                            "type": {
                                "name": None
                            },
                        }
                    ],
                },
                {
                    "name": "withContextChild",
                    "description": None,
                    "args": [],
                },
                {
                    "name": "withImplicitLifetimeChild",
                    "description": None,
                    "args": [],
                },
                {
                    "name": "withMutArg",
                    "description": None,
                    "args": [
                        {
                            "name": "arg",
                            "description": None,
                            "type": {
                                "name": None,
                            },
                        }
                    ],
                },
                {
                    "name": "withoutTypeAlias",
                    "description": None,
                    "args": [],
                }
            ]
        })
    );
}

#[tokio::test]
async fn object_query() {
    let doc = r#"
    subscription {
        withSelf
        independent
        withExecutor
        withExecutorAndSelf
        withContext
        withContextAndSelf
        renamed
        hasArgument(arg1: true)
        defaultArgument
        argWithDescription(arg: true)
        withContextChild {
            ctx
        }
        withImplicitLifetimeChild {
            value
        }
        withMutArg(arg: true)
        withoutTypeAlias
    }
    "#;
    let schema = RootNode::new(
        Query,
        EmptyMutation::<Context>::new(),
        Subscription { b: true },
    );
    let vars = std::collections::HashMap::new();

    let (stream_val, errs) =
        crate::resolve_into_stream(doc, None, &schema, &vars, &Context { flag1: true })
            .await
            .expect("Execution failed");

    let result = if let Value::Object(obj) = stream_val {
        let mut result = Vec::new();
        for (name, mut val) in obj {
            if let Value::Scalar(ref mut stream) = val {
                let first = stream
                    .next()
                    .await
                    .expect("Stream does not have the first element")
                    .expect(&format!("Error resolving {} field", name));
                result.push((name, first))
            }
        }
        result
    } else {
        panic!("Expected to get Value::Object ")
    };

    assert_eq!(errs, []);
    assert_eq!(
        result,
        vec![
            ("withSelf".to_string(), graphql_value!(true)),
            ("independent".to_string(), graphql_value!(100)),
            ("withExecutor".to_string(), graphql_value!(true)),
            ("withExecutorAndSelf".to_string(), graphql_value!(true)),
            ("withContext".to_string(), graphql_value!(true)),
            ("withContextAndSelf".to_string(), graphql_value!(true)),
            ("renamed".to_string(), graphql_value!(true)),
            ("hasArgument".to_string(), graphql_value!(true)),
            ("defaultArgument".to_string(), graphql_value!(true)),
            ("argWithDescription".to_string(), graphql_value!(true)),
            (
                "withContextChild".to_string(),
                graphql_value!({"ctx": true})
            ),
            (
                "withImplicitLifetimeChild".to_string(),
                graphql_value!({ "value": "blub" })
            ),
            ("withMutArg".to_string(), graphql_value!(false)),
            ("withoutTypeAlias".to_string(), graphql_value!("abc")),
        ]
    );
}
