use executor::Variables;
use schema::model::RootNode;
use types::scalars::EmptyMutation;
use value::Value;

struct Root;

/*

Syntax to validate:

* No args at all
* Executor arg vs. no executor arg
* Single arg vs. multi arg
* Trailing comma vs. no trailing comma
* Default value vs. no default value
* Complex default value
* Description vs. no description

*/

#[derive(GraphQLInputObject)]
#[graphql(_internal)]
struct Point {
    x: i32,
}

graphql_object!(Root: () |&self| {
    field simple() -> i32 { 0 }
    field exec_arg(&executor) -> i32 { 0 }
    field exec_arg_and_more(&executor, arg: i32) -> i32 { 0 }

    field single_arg(arg: i32) -> i32 { 0 }
    field multi_args(
        arg1: i32,
        arg2: i32
    ) -> i32 { 0 }
    field multi_args_trailing_comma(
        arg1: i32,
        arg2: i32,
    ) -> i32 { 0 }

    field single_arg_descr(arg: i32 as "The arg") -> i32 { 0 }
    field multi_args_descr(
        arg1: i32 as "The first arg",
        arg2: i32 as "The second arg"
    ) -> i32 { 0 }
    field multi_args_descr_trailing_comma(
        arg1: i32 as "The first arg",
        arg2: i32 as "The second arg",
    ) -> i32 { 0 }

    field arg_with_default(arg = 123: i32) -> i32 { 0 }
    field multi_args_with_default(
        arg1 = 123: i32,
        arg2 = 456: i32
    ) -> i32 { 0 }
    field multi_args_with_default_trailing_comma(
        arg1 = 123: i32,
        arg2 = 456: i32,
    ) -> i32 { 0 }

    field arg_with_default_descr(arg = 123: i32 as "The arg") -> i32 { 0 }
    field multi_args_with_default_descr(
        arg1 = 123: i32 as "The first arg",
        arg2 = 456: i32 as "The second arg"
    ) -> i32 { 0 }
    field multi_args_with_default_trailing_comma_descr(
        arg1 = 123: i32 as "The first arg",
        arg2 = 456: i32 as "The second arg",
    ) -> i32 { 0 }

    field args_with_complex_default(
        arg1 = ("test".to_owned()): String as "A string default argument",
        arg2 = (Point { x: 1 }): Point as "An input object default argument",
    ) -> i32 { 0 }
});

fn run_args_info_query<F>(field_name: &str, f: F)
where
    F: Fn(&Vec<Value>) -> (),
{
    let doc = r#"
    {
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
    }
    "#;
    let schema = RootNode::new(Root {}, EmptyMutation::<()>::new());

    let (result, errs) =
        ::execute(doc, None, &schema, &Variables::new(), &()).expect("Execution failed");

    assert_eq!(errs, []);

    println!("Result: {:?}", result);

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
                .as_string_value()
                .expect("name is not a string") == field_name
        })
        .next()
        .expect("Field not found")
        .as_object_value()
        .expect("Field is not an object");

    println!("Field: {:?}", field);

    let args = field
        .get_field_value("args")
        .expect("args missing from field")
        .as_list_value()
        .expect("args is not a list");

    println!("Args: {:?}", args);

    f(args);
}

#[test]
fn introspect_field_simple() {
    run_args_info_query("simple", |args| {
        assert_eq!(args.len(), 0);
    });
}

#[test]
fn introspect_field_exec_arg() {
    run_args_info_query("execArg", |args| {
        assert_eq!(args.len(), 0);
    });
}

#[test]
fn introspect_field_exec_arg_and_more() {
    run_args_info_query("execArgAndMore", |args| {
        assert_eq!(args.len(), 1);

        assert!(
            args.contains(&Value::object(
                vec![
                    ("name", Value::string("arg")),
                    ("description", Value::null()),
                    ("defaultValue", Value::null()),
                    (
                        "type",
                        Value::object(
                            vec![
                                ("name", Value::null()),
                                (
                                    "ofType",
                                    Value::object(
                                        vec![("name", Value::string("Int"))].into_iter().collect(),
                                    ),
                                ),
                            ].into_iter()
                                .collect(),
                        ),
                    ),
                ].into_iter()
                    .collect(),
            ))
        );
    });
}

#[test]
fn introspect_field_single_arg() {
    run_args_info_query("singleArg", |args| {
        assert_eq!(args.len(), 1);

        assert!(
            args.contains(&Value::object(
                vec![
                    ("name", Value::string("arg")),
                    ("description", Value::null()),
                    ("defaultValue", Value::null()),
                    (
                        "type",
                        Value::object(
                            vec![
                                ("name", Value::null()),
                                (
                                    "ofType",
                                    Value::object(
                                        vec![("name", Value::string("Int"))].into_iter().collect(),
                                    ),
                                ),
                            ].into_iter()
                                .collect(),
                        ),
                    ),
                ].into_iter()
                    .collect(),
            ))
        );
    });
}

#[test]
fn introspect_field_multi_args() {
    run_args_info_query("multiArgs", |args| {
        assert_eq!(args.len(), 2);

        assert!(
            args.contains(&Value::object(
                vec![
                    ("name", Value::string("arg1")),
                    ("description", Value::null()),
                    ("defaultValue", Value::null()),
                    (
                        "type",
                        Value::object(
                            vec![
                                ("name", Value::null()),
                                (
                                    "ofType",
                                    Value::object(
                                        vec![("name", Value::string("Int"))].into_iter().collect(),
                                    ),
                                ),
                            ].into_iter()
                                .collect(),
                        ),
                    ),
                ].into_iter()
                    .collect(),
            ))
        );

        assert!(
            args.contains(&Value::object(
                vec![
                    ("name", Value::string("arg2")),
                    ("description", Value::null()),
                    ("defaultValue", Value::null()),
                    (
                        "type",
                        Value::object(
                            vec![
                                ("name", Value::null()),
                                (
                                    "ofType",
                                    Value::object(
                                        vec![("name", Value::string("Int"))].into_iter().collect(),
                                    ),
                                ),
                            ].into_iter()
                                .collect(),
                        ),
                    ),
                ].into_iter()
                    .collect(),
            ))
        );
    });
}

#[test]
fn introspect_field_multi_args_trailing_comma() {
    run_args_info_query("multiArgsTrailingComma", |args| {
        assert_eq!(args.len(), 2);

        assert!(
            args.contains(&Value::object(
                vec![
                    ("name", Value::string("arg1")),
                    ("description", Value::null()),
                    ("defaultValue", Value::null()),
                    (
                        "type",
                        Value::object(
                            vec![
                                ("name", Value::null()),
                                (
                                    "ofType",
                                    Value::object(
                                        vec![("name", Value::string("Int"))].into_iter().collect(),
                                    ),
                                ),
                            ].into_iter()
                                .collect(),
                        ),
                    ),
                ].into_iter()
                    .collect(),
            ))
        );

        assert!(
            args.contains(&Value::object(
                vec![
                    ("name", Value::string("arg2")),
                    ("description", Value::null()),
                    ("defaultValue", Value::null()),
                    (
                        "type",
                        Value::object(
                            vec![
                                ("name", Value::null()),
                                (
                                    "ofType",
                                    Value::object(
                                        vec![("name", Value::string("Int"))].into_iter().collect(),
                                    ),
                                ),
                            ].into_iter()
                                .collect(),
                        ),
                    ),
                ].into_iter()
                    .collect(),
            ))
        );
    });
}

#[test]
fn introspect_field_single_arg_descr() {
    run_args_info_query("singleArgDescr", |args| {
        assert_eq!(args.len(), 1);

        assert!(
            args.contains(&Value::object(
                vec![
                    ("name", Value::string("arg")),
                    ("description", Value::string("The arg")),
                    ("defaultValue", Value::null()),
                    (
                        "type",
                        Value::object(
                            vec![
                                ("name", Value::null()),
                                (
                                    "ofType",
                                    Value::object(
                                        vec![("name", Value::string("Int"))].into_iter().collect(),
                                    ),
                                ),
                            ].into_iter()
                                .collect(),
                        ),
                    ),
                ].into_iter()
                    .collect(),
            ))
        );
    });
}

#[test]
fn introspect_field_multi_args_descr() {
    run_args_info_query("multiArgsDescr", |args| {
        assert_eq!(args.len(), 2);

        assert!(
            args.contains(&Value::object(
                vec![
                    ("name", Value::string("arg1")),
                    ("description", Value::string("The first arg")),
                    ("defaultValue", Value::null()),
                    (
                        "type",
                        Value::object(
                            vec![
                                ("name", Value::null()),
                                (
                                    "ofType",
                                    Value::object(
                                        vec![("name", Value::string("Int"))].into_iter().collect(),
                                    ),
                                ),
                            ].into_iter()
                                .collect(),
                        ),
                    ),
                ].into_iter()
                    .collect(),
            ))
        );

        assert!(
            args.contains(&Value::object(
                vec![
                    ("name", Value::string("arg2")),
                    ("description", Value::string("The second arg")),
                    ("defaultValue", Value::null()),
                    (
                        "type",
                        Value::object(
                            vec![
                                ("name", Value::null()),
                                (
                                    "ofType",
                                    Value::object(
                                        vec![("name", Value::string("Int"))].into_iter().collect(),
                                    ),
                                ),
                            ].into_iter()
                                .collect(),
                        ),
                    ),
                ].into_iter()
                    .collect(),
            ))
        );
    });
}

#[test]
fn introspect_field_multi_args_descr_trailing_comma() {
    run_args_info_query("multiArgsDescrTrailingComma", |args| {
        assert_eq!(args.len(), 2);

        assert!(
            args.contains(&Value::object(
                vec![
                    ("name", Value::string("arg1")),
                    ("description", Value::string("The first arg")),
                    ("defaultValue", Value::null()),
                    (
                        "type",
                        Value::object(
                            vec![
                                ("name", Value::null()),
                                (
                                    "ofType",
                                    Value::object(
                                        vec![("name", Value::string("Int"))].into_iter().collect(),
                                    ),
                                ),
                            ].into_iter()
                                .collect(),
                        ),
                    ),
                ].into_iter()
                    .collect(),
            ))
        );

        assert!(
            args.contains(&Value::object(
                vec![
                    ("name", Value::string("arg2")),
                    ("description", Value::string("The second arg")),
                    ("defaultValue", Value::null()),
                    (
                        "type",
                        Value::object(
                            vec![
                                ("name", Value::null()),
                                (
                                    "ofType",
                                    Value::object(
                                        vec![("name", Value::string("Int"))].into_iter().collect(),
                                    ),
                                ),
                            ].into_iter()
                                .collect(),
                        ),
                    ),
                ].into_iter()
                    .collect(),
            ))
        );
    });
}

#[test]
fn introspect_field_arg_with_default() {
    run_args_info_query("argWithDefault", |args| {
        assert_eq!(args.len(), 1);

        assert!(
            args.contains(&Value::object(
                vec![
                    ("name", Value::string("arg")),
                    ("description", Value::null()),
                    ("defaultValue", Value::string("123")),
                    (
                        "type",
                        Value::object(
                            vec![("name", Value::string("Int")), ("ofType", Value::null())]
                                .into_iter()
                                .collect(),
                        ),
                    ),
                ].into_iter()
                    .collect(),
            ))
        );
    });
}

#[test]
fn introspect_field_multi_args_with_default() {
    run_args_info_query("multiArgsWithDefault", |args| {
        assert_eq!(args.len(), 2);

        assert!(
            args.contains(&Value::object(
                vec![
                    ("name", Value::string("arg1")),
                    ("description", Value::null()),
                    ("defaultValue", Value::string("123")),
                    (
                        "type",
                        Value::object(
                            vec![("name", Value::string("Int")), ("ofType", Value::null())]
                                .into_iter()
                                .collect(),
                        ),
                    ),
                ].into_iter()
                    .collect(),
            ))
        );

        assert!(
            args.contains(&Value::object(
                vec![
                    ("name", Value::string("arg2")),
                    ("description", Value::null()),
                    ("defaultValue", Value::string("456")),
                    (
                        "type",
                        Value::object(
                            vec![("name", Value::string("Int")), ("ofType", Value::null())]
                                .into_iter()
                                .collect(),
                        ),
                    ),
                ].into_iter()
                    .collect(),
            ))
        );
    });
}

#[test]
fn introspect_field_multi_args_with_default_trailing_comma() {
    run_args_info_query("multiArgsWithDefaultTrailingComma", |args| {
        assert_eq!(args.len(), 2);

        assert!(
            args.contains(&Value::object(
                vec![
                    ("name", Value::string("arg1")),
                    ("description", Value::null()),
                    ("defaultValue", Value::string("123")),
                    (
                        "type",
                        Value::object(
                            vec![("name", Value::string("Int")), ("ofType", Value::null())]
                                .into_iter()
                                .collect(),
                        ),
                    ),
                ].into_iter()
                    .collect(),
            ))
        );

        assert!(
            args.contains(&Value::object(
                vec![
                    ("name", Value::string("arg2")),
                    ("description", Value::null()),
                    ("defaultValue", Value::string("456")),
                    (
                        "type",
                        Value::object(
                            vec![("name", Value::string("Int")), ("ofType", Value::null())]
                                .into_iter()
                                .collect(),
                        ),
                    ),
                ].into_iter()
                    .collect(),
            ))
        );
    });
}

#[test]
fn introspect_field_arg_with_default_descr() {
    run_args_info_query("argWithDefaultDescr", |args| {
        assert_eq!(args.len(), 1);

        assert!(
            args.contains(&Value::object(
                vec![
                    ("name", Value::string("arg")),
                    ("description", Value::string("The arg")),
                    ("defaultValue", Value::string("123")),
                    (
                        "type",
                        Value::object(
                            vec![("name", Value::string("Int")), ("ofType", Value::null())]
                                .into_iter()
                                .collect(),
                        ),
                    ),
                ].into_iter()
                    .collect(),
            ))
        );
    });
}

#[test]
fn introspect_field_multi_args_with_default_descr() {
    run_args_info_query("multiArgsWithDefaultDescr", |args| {
        assert_eq!(args.len(), 2);

        assert!(
            args.contains(&Value::object(
                vec![
                    ("name", Value::string("arg1")),
                    ("description", Value::string("The first arg")),
                    ("defaultValue", Value::string("123")),
                    (
                        "type",
                        Value::object(
                            vec![("name", Value::string("Int")), ("ofType", Value::null())]
                                .into_iter()
                                .collect(),
                        ),
                    ),
                ].into_iter()
                    .collect(),
            ))
        );

        assert!(
            args.contains(&Value::object(
                vec![
                    ("name", Value::string("arg2")),
                    ("description", Value::string("The second arg")),
                    ("defaultValue", Value::string("456")),
                    (
                        "type",
                        Value::object(
                            vec![("name", Value::string("Int")), ("ofType", Value::null())]
                                .into_iter()
                                .collect(),
                        ),
                    ),
                ].into_iter()
                    .collect(),
            ))
        );
    });
}

#[test]
fn introspect_field_multi_args_with_default_trailing_comma_descr() {
    run_args_info_query("multiArgsWithDefaultTrailingCommaDescr", |args| {
        assert_eq!(args.len(), 2);

        assert!(
            args.contains(&Value::object(
                vec![
                    ("name", Value::string("arg1")),
                    ("description", Value::string("The first arg")),
                    ("defaultValue", Value::string("123")),
                    (
                        "type",
                        Value::object(
                            vec![("name", Value::string("Int")), ("ofType", Value::null())]
                                .into_iter()
                                .collect(),
                        ),
                    ),
                ].into_iter()
                    .collect(),
            ))
        );

        assert!(
            args.contains(&Value::object(
                vec![
                    ("name", Value::string("arg2")),
                    ("description", Value::string("The second arg")),
                    ("defaultValue", Value::string("456")),
                    (
                        "type",
                        Value::object(
                            vec![("name", Value::string("Int")), ("ofType", Value::null())]
                                .into_iter()
                                .collect(),
                        ),
                    ),
                ].into_iter()
                    .collect(),
            ))
        );
    });
}

#[test]
fn introspect_field_args_with_complex_default() {
    run_args_info_query("argsWithComplexDefault", |args| {
        assert_eq!(args.len(), 2);

        assert!(
            args.contains(&Value::object(
                vec![
                    ("name", Value::string("arg1")),
                    ("description", Value::string("A string default argument")),
                    ("defaultValue", Value::string(r#""test""#)),
                    (
                        "type",
                        Value::object(
                            vec![("name", Value::string("String")), ("ofType", Value::null())]
                                .into_iter()
                                .collect(),
                        ),
                    ),
                ].into_iter()
                    .collect(),
            ))
        );

        assert!(
            args.contains(&Value::object(
                vec![
                    ("name", Value::string("arg2")),
                    (
                        "description",
                        Value::string("An input object default argument"),
                    ),
                    ("defaultValue", Value::string(r#"{x: 1}"#)),
                    (
                        "type",
                        Value::object(
                            vec![("name", Value::string("Point")), ("ofType", Value::null())]
                                .into_iter()
                                .collect(),
                        ),
                    ),
                ].into_iter()
                    .collect(),
            ))
        );
    });
}
