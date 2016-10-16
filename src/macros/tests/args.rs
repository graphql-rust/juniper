use std::collections::HashMap;

use executor::FieldResult;
use value::Value;
use schema::model::RootNode;

struct Root;

/*

Syntax to validate:

* No args at all
* Executor arg vs. no executor arg
* Single arg vs. multi arg
* Trailing comma vs. no trailing comma
* Default value vs. no default value
* Description vs. no description

*/

graphql_object!(Root: () as "Root" |&self| {
    field simple() -> FieldResult<i64> { Ok(0) }
    field exec_arg(&mut executor) -> FieldResult<i64> { Ok(0) }
    field exec_arg_and_more(&mut executor, arg: i64) -> FieldResult<i64> { Ok(0) }

    field single_arg(arg: i64) -> FieldResult<i64> { Ok(0) }
    field multi_args(
        arg1: i64,
        arg2: i64
    ) -> FieldResult<i64> { Ok(0) }
    field multi_args_trailing_comma(
        arg1: i64,
        arg2: i64,
    ) -> FieldResult<i64> { Ok(0) }

    field single_arg_descr(arg: i64 as "The arg") -> FieldResult<i64> { Ok(0) }
    field multi_args_descr(
        arg1: i64 as "The first arg",
        arg2: i64 as "The second arg"
    ) -> FieldResult<i64> { Ok(0) }
    field multi_args_descr_trailing_comma(
        arg1: i64 as "The first arg",
        arg2: i64 as "The second arg",
    ) -> FieldResult<i64> { Ok(0) }

    field arg_with_default(arg = 123: i64) -> FieldResult<i64> { Ok(0) }
    field multi_args_with_default(
        arg1 = 123: i64,
        arg2 = 456: i64
    ) -> FieldResult<i64> { Ok(0) }
    field multi_args_with_default_trailing_comma(
        arg1 = 123: i64,
        arg2 = 456: i64,
    ) -> FieldResult<i64> { Ok(0) }

    field arg_with_default_descr(arg = 123: i64 as "The arg") -> FieldResult<i64> { Ok(0) }
    field multi_args_with_default_descr(
        arg1 = 123: i64 as "The first arg",
        arg2 = 456: i64 as "The second arg"
    ) -> FieldResult<i64> { Ok(0) }
    field multi_args_with_default_trailing_comma_descr(
        arg1 = 123: i64 as "The first arg",
        arg2 = 456: i64 as "The second arg",
    ) -> FieldResult<i64> { Ok(0) }
});

fn run_args_info_query<F>(field_name: &str, f: F)
    where F: Fn(&Vec<Value>) -> ()
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
    let schema = RootNode::new(Root {}, ());

    let (result, errs) = ::execute(doc, None, &schema, &HashMap::new(), &())
        .expect("Execution failed");

    assert_eq!(errs, []);

    println!("Result: {:?}", result);

    let type_info = result
        .as_object_value().expect("Result is not an object")
        .get("__type").expect("__type field missing")
        .as_object_value().expect("__type field not an object value");

    let fields = type_info
        .get("fields").expect("fields field missing")
        .as_list_value().expect("fields not a list");

    let field = fields
        .into_iter().filter(
            |f| f.as_object_value().expect("Field not an object")
                .get("name").expect("name field missing from field")
                .as_string_value().expect("name is not a string")
                == field_name)
        .next().expect("Field not found")
        .as_object_value().expect("Field is not an object");

    println!("Field: {:?}", field);

    let args = field
        .get("args").expect("args missing from field")
        .as_list_value().expect("args is not a list");

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

        assert!(args.contains(&Value::object(vec![
            ("name", Value::string("arg")),
            ("description", Value::null()),
            ("defaultValue", Value::null()),
            ("type", Value::object(vec![
                ("name", Value::null()),
                ("ofType", Value::object(vec![
                    ("name", Value::string("Int")),
                ].into_iter().collect())),
            ].into_iter().collect())),
        ].into_iter().collect())));
    });
}

#[test]
fn introspect_field_single_arg() {
    run_args_info_query("singleArg", |args| {
        assert_eq!(args.len(), 1);

        assert!(args.contains(&Value::object(vec![
            ("name", Value::string("arg")),
            ("description", Value::null()),
            ("defaultValue", Value::null()),
            ("type", Value::object(vec![
                ("name", Value::null()),
                ("ofType", Value::object(vec![
                    ("name", Value::string("Int")),
                ].into_iter().collect())),
            ].into_iter().collect())),
        ].into_iter().collect())));
    });
}

#[test]
fn introspect_field_multi_args() {
    run_args_info_query("multiArgs", |args| {
        assert_eq!(args.len(), 2);

        assert!(args.contains(&Value::object(vec![
            ("name", Value::string("arg1")),
            ("description", Value::null()),
            ("defaultValue", Value::null()),
            ("type", Value::object(vec![
                ("name", Value::null()),
                ("ofType", Value::object(vec![
                    ("name", Value::string("Int")),
                ].into_iter().collect())),
            ].into_iter().collect())),
        ].into_iter().collect())));

        assert!(args.contains(&Value::object(vec![
            ("name", Value::string("arg2")),
            ("description", Value::null()),
            ("defaultValue", Value::null()),
            ("type", Value::object(vec![
                ("name", Value::null()),
                ("ofType", Value::object(vec![
                    ("name", Value::string("Int")),
                ].into_iter().collect())),
            ].into_iter().collect())),
        ].into_iter().collect())));
    });
}

#[test]
fn introspect_field_multi_args_trailing_comma() {
    run_args_info_query("multiArgsTrailingComma", |args| {
        assert_eq!(args.len(), 2);

        assert!(args.contains(&Value::object(vec![
            ("name", Value::string("arg1")),
            ("description", Value::null()),
            ("defaultValue", Value::null()),
            ("type", Value::object(vec![
                ("name", Value::null()),
                ("ofType", Value::object(vec![
                    ("name", Value::string("Int")),
                ].into_iter().collect())),
            ].into_iter().collect())),
        ].into_iter().collect())));

        assert!(args.contains(&Value::object(vec![
            ("name", Value::string("arg2")),
            ("description", Value::null()),
            ("defaultValue", Value::null()),
            ("type", Value::object(vec![
                ("name", Value::null()),
                ("ofType", Value::object(vec![
                    ("name", Value::string("Int")),
                ].into_iter().collect())),
            ].into_iter().collect())),
        ].into_iter().collect())));
    });
}

#[test]
fn introspect_field_single_arg_descr() {
    run_args_info_query("singleArgDescr", |args| {
        assert_eq!(args.len(), 1);

        assert!(args.contains(&Value::object(vec![
            ("name", Value::string("arg")),
            ("description", Value::string("The arg")),
            ("defaultValue", Value::null()),
            ("type", Value::object(vec![
                ("name", Value::null()),
                ("ofType", Value::object(vec![
                    ("name", Value::string("Int")),
                ].into_iter().collect())),
            ].into_iter().collect())),
        ].into_iter().collect())));
    });
}

#[test]
fn introspect_field_multi_args_descr() {
    run_args_info_query("multiArgsDescr", |args| {
        assert_eq!(args.len(), 2);

        assert!(args.contains(&Value::object(vec![
            ("name", Value::string("arg1")),
            ("description", Value::string("The first arg")),
            ("defaultValue", Value::null()),
            ("type", Value::object(vec![
                ("name", Value::null()),
                ("ofType", Value::object(vec![
                    ("name", Value::string("Int")),
                ].into_iter().collect())),
            ].into_iter().collect())),
        ].into_iter().collect())));

        assert!(args.contains(&Value::object(vec![
            ("name", Value::string("arg2")),
            ("description", Value::string("The second arg")),
            ("defaultValue", Value::null()),
            ("type", Value::object(vec![
                ("name", Value::null()),
                ("ofType", Value::object(vec![
                    ("name", Value::string("Int")),
                ].into_iter().collect())),
            ].into_iter().collect())),
        ].into_iter().collect())));
    });
}

#[test]
fn introspect_field_multi_args_descr_trailing_comma() {
    run_args_info_query("multiArgsDescrTrailingComma", |args| {
        assert_eq!(args.len(), 2);

        assert!(args.contains(&Value::object(vec![
            ("name", Value::string("arg1")),
            ("description", Value::string("The first arg")),
            ("defaultValue", Value::null()),
            ("type", Value::object(vec![
                ("name", Value::null()),
                ("ofType", Value::object(vec![
                    ("name", Value::string("Int")),
                ].into_iter().collect())),
            ].into_iter().collect())),
        ].into_iter().collect())));

        assert!(args.contains(&Value::object(vec![
            ("name", Value::string("arg2")),
            ("description", Value::string("The second arg")),
            ("defaultValue", Value::null()),
            ("type", Value::object(vec![
                ("name", Value::null()),
                ("ofType", Value::object(vec![
                    ("name", Value::string("Int")),
                ].into_iter().collect())),
            ].into_iter().collect())),
        ].into_iter().collect())));
    });
}

#[test]
fn introspect_field_arg_with_default() {
    run_args_info_query("argWithDefault", |args| {
        assert_eq!(args.len(), 1);

        assert!(args.contains(&Value::object(vec![
            ("name", Value::string("arg")),
            ("description", Value::null()),
            ("defaultValue", Value::string("123")),
            ("type", Value::object(vec![
                ("name", Value::string("Int")),
                ("ofType", Value::null()),
            ].into_iter().collect())),
        ].into_iter().collect())));
    });
}

#[test]
fn introspect_field_multi_args_with_default() {
    run_args_info_query("multiArgsWithDefault", |args| {
        assert_eq!(args.len(), 2);

        assert!(args.contains(&Value::object(vec![
            ("name", Value::string("arg1")),
            ("description", Value::null()),
            ("defaultValue", Value::string("123")),
            ("type", Value::object(vec![
                ("name", Value::string("Int")),
                ("ofType", Value::null()),
            ].into_iter().collect())),
        ].into_iter().collect())));

        assert!(args.contains(&Value::object(vec![
            ("name", Value::string("arg2")),
            ("description", Value::null()),
            ("defaultValue", Value::string("456")),
            ("type", Value::object(vec![
                ("name", Value::string("Int")),
                ("ofType", Value::null()),
            ].into_iter().collect())),
        ].into_iter().collect())));
    });
}

#[test]
fn introspect_field_multi_args_with_default_trailing_comma() {
    run_args_info_query("multiArgsWithDefaultTrailingComma", |args| {
        assert_eq!(args.len(), 2);

        assert!(args.contains(&Value::object(vec![
            ("name", Value::string("arg1")),
            ("description", Value::null()),
            ("defaultValue", Value::string("123")),
            ("type", Value::object(vec![
                ("name", Value::string("Int")),
                ("ofType", Value::null()),
            ].into_iter().collect())),
        ].into_iter().collect())));

        assert!(args.contains(&Value::object(vec![
            ("name", Value::string("arg2")),
            ("description", Value::null()),
            ("defaultValue", Value::string("456")),
            ("type", Value::object(vec![
                ("name", Value::string("Int")),
                ("ofType", Value::null()),
            ].into_iter().collect())),
        ].into_iter().collect())));
    });
}

#[test]
fn introspect_field_arg_with_default_descr() {
    run_args_info_query("argWithDefaultDescr", |args| {
        assert_eq!(args.len(), 1);

        assert!(args.contains(&Value::object(vec![
            ("name", Value::string("arg")),
            ("description", Value::string("The arg")),
            ("defaultValue", Value::string("123")),
            ("type", Value::object(vec![
                ("name", Value::string("Int")),
                ("ofType", Value::null()),
            ].into_iter().collect())),
        ].into_iter().collect())));
    });
}

#[test]
fn introspect_field_multi_args_with_default_descr() {
    run_args_info_query("multiArgsWithDefaultDescr", |args| {
        assert_eq!(args.len(), 2);

        assert!(args.contains(&Value::object(vec![
            ("name", Value::string("arg1")),
            ("description", Value::string("The first arg")),
            ("defaultValue", Value::string("123")),
            ("type", Value::object(vec![
                ("name", Value::string("Int")),
                ("ofType", Value::null()),
            ].into_iter().collect())),
        ].into_iter().collect())));

        assert!(args.contains(&Value::object(vec![
            ("name", Value::string("arg2")),
            ("description", Value::string("The second arg")),
            ("defaultValue", Value::string("456")),
            ("type", Value::object(vec![
                ("name", Value::string("Int")),
                ("ofType", Value::null()),
            ].into_iter().collect())),
        ].into_iter().collect())));
    });
}

#[test]
fn introspect_field_multi_args_with_default_trailing_comma_descr() {
    run_args_info_query("multiArgsWithDefaultTrailingCommaDescr", |args| {
        assert_eq!(args.len(), 2);

        assert!(args.contains(&Value::object(vec![
            ("name", Value::string("arg1")),
            ("description", Value::string("The first arg")),
            ("defaultValue", Value::string("123")),
            ("type", Value::object(vec![
                ("name", Value::string("Int")),
                ("ofType", Value::null()),
            ].into_iter().collect())),
        ].into_iter().collect())));

        assert!(args.contains(&Value::object(vec![
            ("name", Value::string("arg2")),
            ("description", Value::string("The second arg")),
            ("defaultValue", Value::string("456")),
            ("type", Value::object(vec![
                ("name", Value::string("Int")),
                ("ofType", Value::null()),
            ].into_iter().collect())),
        ].into_iter().collect())));
    });
}
