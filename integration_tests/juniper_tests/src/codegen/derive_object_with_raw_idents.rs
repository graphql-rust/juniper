#[cfg(test)]
use fnv::FnvHashMap;
#[cfg(test)]
use juniper::Object;

#[cfg(test)]
use juniper::{
    self, execute, graphql_value, DefaultScalarValue, EmptyMutation, GraphQLInputObject,
    GraphQLType, RootNode, Value, Variables,
};

pub struct Query;

#[juniper::object]
impl Query {
    fn r#type(r#fn: MyInputType) -> Vec<String> {
        unimplemented!()
    }
}

#[derive(GraphQLInputObject, Debug, PartialEq)]
struct MyInputType {
    r#trait: String,
}

#[test]
fn supports_raw_idents_in_types_and_args() {
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

    let value = run_type_info_query(&doc);

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

#[test]
fn supports_raw_idents_in_fields_of_input_types() {
    let doc = r#"
    {
        __type(name: "MyInputType") {
            inputFields {
              name
            }
        }
    }
    "#;

    let value = run_type_info_query(&doc);

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

#[cfg(test)]
fn run_type_info_query(doc: &str) -> Value {
    let schema = RootNode::new(Query, EmptyMutation::<()>::new());

    let (result, errs) =
        execute(doc, None, &schema, &Variables::new(), &()).expect("Execution failed");

    assert_eq!(errs, []);

    println!("Result: {:#?}", result);
    result
}
