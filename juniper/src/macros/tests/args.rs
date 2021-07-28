use crate::{
    executor::Variables,
    graphql_object, graphql_value,
    schema::model::RootNode,
    types::scalars::{EmptyMutation, EmptySubscription},
    DefaultScalarValue, Executor, GraphQLInputObject, ScalarValue, Value,
};

#[derive(GraphQLInputObject, Debug)]
struct Point {
    x: i32,
}

struct Root;

#[graphql_object]
impl Root {
    fn simple() -> i32 {
        0
    }

    fn exec_arg<S: ScalarValue>(_executor: &Executor<'_, '_, (), S>) -> i32 {
        0
    }
    fn exec_arg_and_more<S: ScalarValue>(_executor: &Executor<'_, '_, (), S>, arg: i32) -> i32 {
        arg
    }

    fn single_arg(arg: i32) -> i32 {
        arg
    }

    fn multi_args(arg1: i32, arg2: i32) -> i32 {
        arg1 + arg2
    }

    fn multi_args_trailing_comma(arg1: i32, arg2: i32) -> i32 {
        arg1 + arg2
    }

    fn single_arg_descr(#[graphql(description = "The arg")] arg: i32) -> i32 {
        arg
    }

    fn single_arg_descr_raw_idents(#[graphql(description = "The arg")] r#arg: i32) -> i32 {
        r#arg
    }

    fn multi_args_descr(
        #[graphql(description = "The first arg")] arg1: i32,
        #[graphql(description = "The second arg")] arg2: i32,
    ) -> i32 {
        arg1 + arg2
    }

    fn multi_args_descr_raw_idents(
        #[graphql(description = "The first arg")] r#arg1: i32,
        #[graphql(description = "The second arg")] r#arg2: i32,
    ) -> i32 {
        r#arg1 + r#arg2
    }

    fn attr_arg_descr(#[graphql(description = "The arg")] _arg: i32) -> i32 {
        0
    }

    fn arg_with_default(#[graphql(default = 123)] arg: i32) -> i32 {
        arg
    }

    fn multi_args_with_default(
        #[graphql(default = 123)] arg1: i32,
        #[graphql(default = 456)] arg2: i32,
    ) -> i32 {
        arg1 + arg2
    }

    fn arg_with_default_descr(#[graphql(default = 123, description = "The arg")] arg: i32) -> i32 {
        arg
    }

    fn arg_with_default_descr_raw_ident(
        #[graphql(default = 123, description = "The arg")] r#arg: i32,
    ) -> i32 {
        r#arg
    }

    fn multi_args_with_default_descr(
        #[graphql(default = 123, description = "The first arg")] arg1: i32,
        #[graphql(default = 456, description = "The second arg")] arg2: i32,
    ) -> i32 {
        arg1 + arg2
    }

    fn multi_args_with_default_descr_raw_ident(
        #[graphql(default = 123, description = "The first arg")] r#arg1: i32,
        #[graphql(default = 456, description = "The second arg")] r#arg2: i32,
    ) -> i32 {
        r#arg1 + r#arg2
    }

    fn args_with_complex_default(
        #[graphql(
            default = "test",
            description = "A string default argument",
        )]
        arg1: String,
        #[graphql(
            default = Point { x: 1 },
            description = "An input object default argument",
        )]
        arg2: Point,
    ) -> i32 {
        let _ = arg1;
        let _ = arg2;
        0
    }
}

async fn run_args_info_query<F>(field_name: &str, f: F)
where
    F: Fn(&Vec<Value<DefaultScalarValue>>) -> (),
{
    let doc = r#"{
        __type(name: "Root") {
            fields {
                name
                args {
                    name
                    description
                    defaultValue
                    type {
                        name
                        ofType {
                            name
                        }
                    }
                }
            }
        }
    }"#;
    let schema = RootNode::new(
        Root {},
        EmptyMutation::<()>::new(),
        EmptySubscription::<()>::new(),
    );

    let (result, errs) = crate::execute(doc, None, &schema, &Variables::new(), &())
        .await
        .expect("Execution failed");

    assert_eq!(errs, []);

    println!("Result: {:#?}", result);

    let type_info = result
        .as_object_value()
        .expect("Result is not an object")
        .get_field_value("__type")
        .expect("__type field missing")
        .as_object_value()
        .expect("__type field not an object value");

    let fields = type_info
        .get_field_value("fields")
        .expect("fields field missing")
        .as_list_value()
        .expect("fields not a list");

    let field = fields
        .into_iter()
        .filter(|f| {
            f.as_object_value()
                .expect("Field not an object")
                .get_field_value("name")
                .expect("name field missing from field")
                .as_scalar_value::<String>()
                .expect("name is not a string")
                == field_name
        })
        .next()
        .expect("Field not found")
        .as_object_value()
        .expect("Field is not an object");

    println!("Field: {:#?}", field);

    let args = field
        .get_field_value("args")
        .expect("args missing from field")
        .as_list_value()
        .expect("args is not a list");

    println!("Args: {:#?}", args);

    f(args);
}

#[tokio::test]
async fn introspect_field_simple() {
    run_args_info_query("simple", |args| {
        assert_eq!(args.len(), 0);
    })
    .await
}

#[tokio::test]
async fn introspect_field_exec_arg() {
    run_args_info_query("execArg", |args| {
        assert_eq!(args.len(), 0);
    })
    .await
}

#[tokio::test]
async fn introspect_field_exec_arg_and_more() {
    run_args_info_query("execArgAndMore", |args| {
        assert_eq!(args.len(), 1);
        assert!(args.contains(&graphql_value!({
            "name": "arg",
            "description": None,
            "defaultValue": None,
            "type": {
                "name": None,
                "ofType": { "name": "Int" },
            },
        })));
    })
    .await
}

#[tokio::test]
async fn introspect_field_single_arg() {
    run_args_info_query("singleArg", |args| {
        assert_eq!(args.len(), 1);
        assert!(args.contains(&graphql_value!({
            "name": "arg",
            "description": None,
            "defaultValue": None,
            "type": {
                "name": None,
                "ofType": { "name": "Int" },
            },
        })));
    })
    .await
}

#[tokio::test]
async fn introspect_field_multi_args() {
    run_args_info_query("multiArgs", |args| {
        assert_eq!(args.len(), 2);
        assert!(args.contains(&graphql_value!({
            "name": "arg1",
            "description": None,
            "defaultValue": None,
            "type": {
                "name": None,
                "ofType": { "name": "Int" },
            },
        })));
        assert!(args.contains(&graphql_value!({
            "name": "arg2",
            "description": None,
            "defaultValue": None,
            "type": {
                "name": None,
                "ofType": { "name": "Int" },
            },
        })));
    })
    .await
}

#[tokio::test]
async fn introspect_field_multi_args_trailing_comma() {
    run_args_info_query("multiArgsTrailingComma", |args| {
        assert_eq!(args.len(), 2);
        assert!(args.contains(&graphql_value!({
            "name": "arg1",
            "description": None,
            "defaultValue": None,
            "type": {
                "name": None,
                "ofType": { "name": "Int" },
            },
        })));
        assert!(args.contains(&graphql_value!({
            "name": "arg2",
            "description": None,
            "defaultValue": None,
            "type": {
                "name": None,
                "ofType": { "name": "Int" },
            },
        })));
    })
    .await
}

#[tokio::test]
async fn introspect_field_single_arg_descr() {
    run_args_info_query("singleArgDescr", |args| {
        assert_eq!(args.len(), 1);
        assert!(args.contains(&graphql_value!({
            "name": "arg",
            "description": "The arg",
            "defaultValue": None,
            "type": {
                "name": None,
                "ofType": { "name": "Int" },
            },
        })));
    })
    .await
}

#[tokio::test]
async fn introspect_field_single_arg_descr_raw_idents() {
    run_args_info_query("singleArgDescrRawIdents", |args| {
        assert_eq!(args.len(), 1);
        assert!(args.contains(&graphql_value!({
            "name": "arg",
            "description": "The arg",
            "defaultValue": None,
            "type": {
                "name": None,
                "ofType": { "name": "Int" },
            },
        })));
    })
    .await
}

#[tokio::test]
async fn introspect_field_multi_args_descr() {
    run_args_info_query("multiArgsDescr", |args| {
        assert_eq!(args.len(), 2);
        assert!(args.contains(&graphql_value!({
            "name": "arg1",
            "description": "The first arg",
            "defaultValue": None,
            "type": {
                "name": None,
                "ofType": { "name": "Int" },
            },
        })));
        assert!(args.contains(&graphql_value!({
            "name": "arg2",
            "description": "The second arg",
            "defaultValue": None,
            "type": {
                "name": None,
                "ofType": { "name": "Int" },
            },
        })));
    })
    .await
}

#[tokio::test]
async fn introspect_field_multi_args_descr_raw_idents() {
    run_args_info_query("multiArgsDescrRawIdents", |args| {
        assert_eq!(args.len(), 2);
        assert!(args.contains(&graphql_value!({
            "name": "arg1",
            "description": "The first arg",
            "defaultValue": None,
            "type": {
                "name": None,
                "ofType": { "name": "Int" },
            },
        })));
        assert!(args.contains(&graphql_value!({
            "name": "arg2",
            "description": "The second arg",
            "defaultValue": None,
            "type": {
                "name": None,
                "ofType": { "name": "Int" },
            },
        })));
    })
    .await
}

#[tokio::test]
async fn introspect_field_attr_arg_descr() {
    run_args_info_query("attrArgDescr", |args| {
        assert_eq!(args.len(), 1);
        assert!(args.contains(&graphql_value!({
            "name": "arg",
            "description": "The arg",
            "defaultValue": None,
            "type": {
                "name": None,
                "ofType": { "name": "Int" },
            },
        })));
    })
    .await
}

#[tokio::test]
async fn introspect_field_arg_with_default() {
    run_args_info_query("argWithDefault", |args| {
        assert_eq!(args.len(), 1);
        assert!(args.contains(&graphql_value!({
            "name": "arg",
            "description": None,
            "defaultValue": "123",
            "type": {
                "name": "Int",
                "ofType": None,
            },
        })));
    })
    .await
}

#[tokio::test]
async fn introspect_field_multi_args_with_default() {
    run_args_info_query("multiArgsWithDefault", |args| {
        assert_eq!(args.len(), 2);
        assert!(args.contains(&graphql_value!({
            "name": "arg1",
            "description": None,
            "defaultValue": "123",
            "type": {
                "name": "Int",
                "ofType": None,
            },
        })));
        assert!(args.contains(&graphql_value!({
            "name": "arg2",
            "description": None,
            "defaultValue": "456",
            "type": {
                "name": "Int",
                "ofType": None,
            },
        })));
    })
    .await
}

#[tokio::test]
async fn introspect_field_arg_with_default_descr() {
    run_args_info_query("argWithDefaultDescr", |args| {
        assert_eq!(args.len(), 1);
        assert!(args.contains(&graphql_value!({
            "name": "arg",
            "description": "The arg",
            "defaultValue": "123",
            "type": {
                "name": "Int",
                "ofType": None,
            },
        })));
    })
    .await
}

#[tokio::test]
async fn introspect_field_arg_with_default_descr_raw_ident() {
    run_args_info_query("argWithDefaultDescrRawIdent", |args| {
        assert_eq!(args.len(), 1);
        assert!(args.contains(&graphql_value!({
            "name": "arg",
            "description": "The arg",
            "defaultValue": "123",
            "type": {
                "name": "Int",
                "ofType": None,
            },
        })));
    })
    .await
}

#[tokio::test]
async fn introspect_field_multi_args_with_default_descr() {
    run_args_info_query("multiArgsWithDefaultDescr", |args| {
        assert_eq!(args.len(), 2);
        assert!(args.contains(&graphql_value!({
            "name": "arg1",
            "description": "The first arg",
            "defaultValue": "123",
            "type": {
                "name": "Int",
                "ofType": None,
            },
        })));
        assert!(args.contains(&graphql_value!({
            "name": "arg2",
            "description": "The second arg",
            "defaultValue": "456",
            "type": {
                "name": "Int",
                "ofType": None,
            },
        })));
    })
    .await
}

#[tokio::test]
async fn introspect_field_multi_args_with_default_descr_raw_ident() {
    run_args_info_query("multiArgsWithDefaultDescrRawIdent", |args| {
        assert_eq!(args.len(), 2);
        assert!(args.contains(&graphql_value!({
            "name": "arg1",
            "description": "The first arg",
            "defaultValue": "123",
            "type": {
                "name": "Int",
                "ofType": None,
            },
        })));
        assert!(args.contains(&graphql_value!({
            "name": "arg2",
            "description": "The second arg",
            "defaultValue": "456",
            "type": {
                "name": "Int",
                "ofType": None,
            },
        })));
    })
    .await
}

#[tokio::test]
async fn introspect_field_args_with_complex_default() {
    run_args_info_query("argsWithComplexDefault", |args| {
        assert_eq!(args.len(), 2);
        assert!(args.contains(&graphql_value!({
            "name": "arg1",
            "description": "A string default argument",
            "defaultValue": r#""test""#,
            "type": {
                "name": "String",
                "ofType": None,
            },
        })));
        assert!(args.contains(&graphql_value!({
            "name": "arg2",
            "description": "An input object default argument",
            "defaultValue": r#"{x: 1}"#,
            "type": {
                "name": "Point",
                "ofType": None,
            },
        })));
    })
    .await
}
