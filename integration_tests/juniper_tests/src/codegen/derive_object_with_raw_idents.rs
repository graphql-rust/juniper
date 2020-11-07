use juniper::{
    execute, graphql_object, graphql_value, EmptyMutation, EmptySubscription, GraphQLInputObject,
    RootNode, Value, Variables,
};

pub struct Query;

#[graphql_object]
impl Query {
    fn r#type(r#fn: MyInputType) -> Vec<String> {
        let _ = r#fn;
        unimplemented!()
    }
}

#[derive(GraphQLInputObject, Debug, PartialEq)]
struct MyInputType {
    r#trait: String,
}

#[tokio::test]
async fn supports_raw_idents_in_types_and_args() {
    let doc = r#"
    {
        __type(name: "Query") {
            fields {
                name
                args {
                    name
                }
            }
        }
    }
    "#;

    let value = run_type_info_query(&doc).await;

    assert_eq!(
        value,
        graphql_value!(
            {
                "__type": {
                    "fields": [
                        {
                            "name": "type",
                            "args": [
                                {
                                    "name": "fn"
                                }
                            ]
                        }
                    ]
                }
            }
        ),
    );
}

#[tokio::test]
async fn supports_raw_idents_in_fields_of_input_types() {
    let doc = r#"
    {
        __type(name: "MyInputType") {
            inputFields {
              name
            }
        }
    }
    "#;

    let value = run_type_info_query(&doc).await;

    assert_eq!(
        value,
        graphql_value!(
            {
                "__type": {
                    "inputFields": [
                        {
                            "name": "trait",
                        }
                    ]
                }
            }
        ),
    );
}

async fn run_type_info_query(doc: &str) -> Value {
    let schema = RootNode::new(
        Query,
        EmptyMutation::<()>::new(),
        EmptySubscription::<()>::new(),
    );

    let (result, errs) = execute(doc, None, &schema, &Variables::new(), &())
        .await
        .expect("Execution failed");

    assert_eq!(errs, []);

    println!("Result: {:#?}", result);
    result
}
