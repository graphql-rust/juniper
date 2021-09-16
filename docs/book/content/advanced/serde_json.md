# Json Support

## Using Json with a Dynamic GraphQL Schema

The following example shows you how to run a GraphQL query against json data held in a
`serde_json::Value`.  To make this work you have to construct the `RootNode` using the
`new_with_info` method so that you can describe the GraphQL schema of the json data.

```rust
use serde_json::json;
use juniper::{
    integrations::serde_json::TypeInfo,
    graphql_value, RootNode, EmptyMutation, EmptySubscription, Variables,
};

fn main() {

    // Use SDL to define the structure of the JSON data
    let type_info = TypeInfo {
        name: "Query".to_string(),
        schema: Some(r#"
            type Query {
                bar: Bar
            }
            type Bar {
                name: String
                capacity: Int
                open: Boolean!
            }
        "#.to_string()),
    };

    // some example json data the matches the SDL
    let data = json!({
            "bar": {
                    "name": "Cheers",
                    "capacity": 80,
                    "open": true,
                },
        });


    // create a root node using the json data and the SDL info
    let root = <RootNode<_, _, _>>::new_with_info(
        data,
        EmptyMutation::new(),
        EmptySubscription::new(),
        type_info,
        (),
        (),
    );

    // Run the executor.
    let (res, _errors) = juniper::execute_sync(
        "query { bar { name} }",
        None,
        &root,
        &Variables::new(),
        &(),
    ).unwrap();

    // Ensure the value matches.
    assert_eq!(
        res,
        graphql_value!({
            "bar": { "name": "Cheers"},
        })
    );
}
```

## Using Json Fields in a GraphQL Object

If you know the schema definition at compile time for a value that your want to hold as a json
field of normal juniper graphql object, you can use the `TypedJson` wrapper struct to provide the 
type information of the wrapped `serde_json::Value`.

```rust
use serde_json::json;
use juniper::{
    graphql_object, EmptyMutation, EmptySubscription, FieldResult,
    Variables, graphql_value,
    integrations::serde_json::{TypedJsonInfo,TypedJson},
};

// Defines schema for the Person Graphql Type
struct Person;
impl TypedJsonInfo for Person {
    fn type_name() -> &'static str {
        "Person"
    }
    fn schema() -> &'static str {
        r#"
            type Person {
                name: String
                age: Int
            }
        "#
    }
}

// You can also define graphql input types this way
struct DetailInput;
impl TypedJsonInfo for DetailInput {
    fn type_name() -> &'static str {
        "DetailInput"
    }
    fn schema() -> &'static str {
        r#"
            input DetailInput {
                ever: Boolean!
            }
        "#
    }
}

struct Query;

#[graphql_object()]
impl Query {
    
    // define a field that uses both Json input type and output type.
    pub fn favorite_person(details: TypedJson<DetailInput>) -> FieldResult<TypedJson<Person>> {
        let ever = details.json.get("ever").unwrap().as_bool().unwrap();
        let data = if ever {
            json!({"name": "Destiny", "age":29})
        } else {
            json!({"name": "David", "age":45})
        };
        Ok(TypedJson::new(data))
    }
}

fn main() {

    let root_node = &juniper::RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());

    // Run the executor.
    let (res, _errors) = juniper::execute_sync(r#"
            query {
                favoritePerson( details: { ever: true }) {
                    name, age
                }
            }"#,
                                               None,
                                               root_node,
                                               &Variables::new(),
                                               &(),
    ).unwrap();

    // Ensure the value matches.
    assert_eq!(
        res,
        graphql_value!({
                "favoritePerson": {"name": "Destiny", "age":29},
            })
    );
}
```