use super::util;
use crate::{graphql_value, EmptyMutation, EmptySubscription, RootNode};

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
    async fn value(&'a self) -> &'a str {
        self.value
    }
}

struct WithContext;

#[crate::graphql_object_internal(Context=Context)]
impl WithContext {
    async fn ctx(ctx: &Context) -> bool {
        ctx.flag1
    }
}

#[derive(Default)]
struct Query {
    b: bool,
}

#[crate::graphql_object_internal(
    scalar = crate::DefaultScalarValue,
    name = "Query", 
    context = Context,
)]
/// Query Description.
impl<'a> Query {
    #[graphql(description = "With Self Description")]
    async fn with_self(&self) -> bool {
        self.b
    }

    async fn independent() -> i32 {
        100
    }

    async fn with_executor(_exec: &Executor<Context>) -> bool {
        true
    }

    async fn with_executor_and_self(&self, _exec: &Executor<Context>) -> bool {
        true
    }

    async fn with_context(_context: &Context) -> bool {
        true
    }

    async fn with_context_and_self(&self, _context: &Context) -> bool {
        true
    }

    #[graphql(name = "renamed")]
    async fn has_custom_name() -> bool {
        true
    }

    #[graphql(description = "attr")]
    async fn has_description_attr() -> bool {
        true
    }

    /// Doc description
    async fn has_description_doc_comment() -> bool {
        true
    }

    async fn has_argument(arg1: bool) -> bool {
        arg1
    }

    #[graphql(arguments(default_arg(default = true)))]
    async fn default_argument(default_arg: bool) -> bool {
        default_arg
    }

    #[graphql(arguments(arg(description = "my argument description")))]
    async fn arg_with_description(arg: bool) -> bool {
        arg
    }

    async fn with_context_child(&self) -> WithContext {
        WithContext
    }

    async fn with_lifetime_child(&self) -> WithLifetime<'a> {
        WithLifetime { value: "blub" }
    }

    async fn with_mut_arg(mut arg: bool) -> bool {
        if arg {
            arg = !arg;
        }
        arg
    }
}

#[derive(Default)]
struct Mutation;

#[crate::graphql_object_internal(context = Context)]
impl Mutation {
    async fn empty() -> bool {
        true
    }
}

#[derive(Default)]
struct Subscription;

#[crate::graphql_object_internal(context = Context)]
impl Subscription {
    async fn empty() -> bool {
        true
    }
}

#[tokio::test]
async fn object_introspect() {
    let res = util::run_info_query::<Query, Mutation, Subscription, Context>("Query").await;
    assert_eq!(
        res,
        crate::graphql_value!({
            "name": "Query",
            "description": "Query Description.",
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
                    "name": "withLifetimeChild",
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
            ]
        })
    );
}

#[tokio::test]
async fn object_query() {
    let doc = r#"
    query {
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
        withLifetimeChild {
            value
        }
        withMutArg(arg: true)
    }
    "#;
    let schema = RootNode::new(
        Query { b: true },
        EmptyMutation::<Context>::new(),
        EmptySubscription::<Context>::new(),
    );
    let vars = std::collections::HashMap::new();

    let (result, errs) = crate::execute(doc, None, &schema, &vars, &Context { flag1: true })
        .await
        .expect("Execution failed");
    assert_eq!(errs, []);
    assert_eq!(
        result,
        graphql_value!({
            "withSelf": true,
            "independent": 100,
            "withExecutor": true,
            "withExecutorAndSelf": true,
            "withContext": true,
            "withContextAndSelf": true,
            "renamed": true,
            "hasArgument": true,
            "defaultArgument": true,
            "argWithDescription": true,
            "withContextChild": { "ctx": true },
            "withLifetimeChild": { "value": "blub" },
            "withMutArg": false,
        })
    );
}
