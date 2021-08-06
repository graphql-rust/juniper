use juniper::{
    execute, graphql_object, graphql_value, DefaultScalarValue, EmptyMutation, EmptySubscription,
    GraphQLObject, GraphQLType, Object, Registry, RootNode, Value, Variables,
};

#[derive(GraphQLObject, Debug, PartialEq)]
#[graphql(rename_all = "none")]
struct NoRenameObj {
    one_field: bool,
    another_field: i32,
}

struct Query;

#[graphql_object]
impl Query {
    fn no_rename_obj() -> NoRenameObj {
        NoRenameObj {
            one_field: true,
            another_field: 146,
        }
    }
}

struct NoRenameQuery;

#[graphql_object(rename_all = "none")]
impl NoRenameQuery {
    fn no_rename_obj() -> NoRenameObj {
        NoRenameObj {
            one_field: true,
            another_field: 146,
        }
    }
}

#[tokio::test]
async fn test_no_rename_root() {
    let doc = r#"{
        no_rename_obj {
            one_field
            another_field
        }
    }"#;

    let schema = RootNode::new(
        NoRenameQuery,
        EmptyMutation::<()>::new(),
        EmptySubscription::<()>::new(),
    );

    assert_eq!(
        execute(doc, None, &schema, &Variables::new(), &()).await,
        Ok((
            graphql_value!({
                "no_rename_obj": {
                    "one_field": true,
                    "another_field": 146,
                },
            }),
            vec![],
        )),
    );
}

#[tokio::test]
async fn test_no_rename_obj() {
    let doc = r#"{
        noRenameObj {
            one_field
            another_field
        }
    }"#;

    let schema = RootNode::new(
        Query,
        EmptyMutation::<()>::new(),
        EmptySubscription::<()>::new(),
    );

    assert_eq!(
        execute(doc, None, &schema, &Variables::new(), &()).await,
        Ok((
            graphql_value!({
                "noRenameObj": {
                    "one_field": true,
                    "another_field": 146,
                },
            }),
            vec![],
        )),
    );
}
