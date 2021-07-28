use crate::{
    graphql_object, graphql_value, DefaultScalarValue, EmptyMutation, EmptySubscription, Executor,
    RootNode,
};

use super::util;

#[derive(Default)]
struct Context {
    flag1: bool,
}

impl crate::Context for Context {}

struct WithLifetime<'a> {
    value: &'a str,
}

#[graphql_object(context = Context)]
impl<'a> WithLifetime<'a> {
    fn value(&self) -> &str {
        self.value
    }
}

struct WithContext;

#[graphql_object(context = Context)]
impl WithContext {
    fn ctx(ctx: &Context) -> bool {
        ctx.flag1
    }
}

#[derive(Default)]
struct Query {
    b: bool,
}

#[graphql_object(
    name = "Query",
    scalar = DefaultScalarValue,
    context = Context,
)]
/// Query Description.
impl Query {
    #[graphql(description = "With Self Description")]
    fn with_self(&self) -> bool {
        self.b
    }

    fn independent() -> i32 {
        100
    }

    fn with_executor(_executor: &Executor<'_, '_, Context>) -> bool {
        true
    }

    fn with_executor_and_self(&self, _executor: &Executor<'_, '_, Context>) -> bool {
        true
    }

    fn with_context(_context: &Context) -> bool {
        true
    }

    fn with_context_and_self(&self, _context: &Context) -> bool {
        true
    }

    #[graphql(name = "renamed")]
    fn has_custom_name() -> bool {
        true
    }

    #[graphql(description = "attr")]
    fn has_description_attr() -> bool {
        true
    }

    /// Doc description
    fn has_description_doc_comment() -> bool {
        true
    }

    fn has_argument(arg1: bool) -> bool {
        arg1
    }

    fn default_argument(#[graphql(default = true)] default_arg: bool) -> bool {
        default_arg
    }

    fn arg_with_description(#[graphql(description = "my argument description")] arg: bool) -> bool {
        arg
    }

    fn with_context_child(&self) -> WithContext {
        WithContext
    }

    fn with_lifetime_child(&self) -> WithLifetime<'static> {
        WithLifetime { value: "blub" }
    }

    fn with_mut_arg(mut arg: bool) -> bool {
        if arg {
            arg = !arg;
        }
        arg
    }
}

#[derive(Default)]
struct Mutation;

#[graphql_object(context = Context)]
impl Mutation {
    fn empty() -> bool {
        true
    }
}

#[derive(Default)]
struct Subscription;

#[graphql_object(context = Context)]
impl Subscription {
    fn empty() -> bool {
        true
    }
}

#[tokio::test]
async fn object_introspect() {
    let res = util::run_info_query::<Query, Mutation, Subscription>("Query").await;
    assert_eq!(
        res,
        graphql_value!({
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
