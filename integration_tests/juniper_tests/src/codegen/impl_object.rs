use juniper::{
    execute, graphql_object, DefaultScalarValue, EmptyMutation, EmptySubscription, Object,
    RootNode, Value, Variables,
};

pub struct MyObject;

#[graphql_object]
impl MyObject {
    #[graphql(arguments(arg(name = "test")))]
    fn test(&self, arg: String) -> String {
        arg
    }
}

#[tokio::test]
async fn check_argument_rename() {
    let doc = format!(
        r#"
    {{
        __type(name: "{}") {{
            name,
            description,
            fields {{
                name
                description
            }}
        }}
    }}
    "#,
        "MyObject"
    );

    run_type_info_query(&doc, |(_, values)| {
        assert_eq!(
            *values,
            vec![Value::object(
                vec![
                    ("name", Value::scalar("test")),
                    ("description", Value::null()),
                ]
                .into_iter()
                .collect(),
            )]
        );
    })
    .await;
}

async fn run_type_info_query<F>(doc: &str, f: F)
where
    F: Fn((&Object<DefaultScalarValue>, &Vec<Value>)) -> (),
{
    let schema = RootNode::new(
        MyObject,
        EmptyMutation::<()>::new(),
        EmptySubscription::<()>::new(),
    );

    let (result, errs) = execute(doc, None, &schema, &Variables::new(), &())
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

    f((type_info, fields));
}

mod fallible {
    use juniper::{graphql_object, FieldError};

    struct Obj;

    #[graphql_object]
    impl Obj {
        fn test(&self, arg: String) -> Result<String, FieldError> {
            Ok(arg)
        }
    }
}
