use juniper::{execute, GraphQLType, Value, Variables, EmptyMutation, RootNode};

#[derive(GraphQLObject, Debug, PartialEq)]
#[graphql(name="MyObj", description="obj descr")]
struct Obj {
  regular_field: bool,
  #[graphql(name="renamedField", description="descr", deprecation="field descr")]
  c: i32,
}

struct Query;

graphql_object!(Query: () |&self| {
    field obj() -> Obj {
      Obj{
        regular_field: true,
        c: 22,
      }
    }
});

#[test]
fn test_derived_object() {
  assert_eq!(Obj::name(), Some("MyObj"));
  let doc = r#"
        {
            obj {
                regularField
                renamedField
            }
        }"#;

  let schema = RootNode::new(Query, EmptyMutation::<()>::new());

  assert_eq!(
    execute(doc, None, &schema, &Variables::new(), &()),
    Ok((Value::object(vec![
      ("obj", Value::object(vec![
        ("regularField", Value::boolean(true)),
        ("renamedField", Value::int(22)),
      ].into_iter().collect())),
    ].into_iter().collect()),
        vec![])));
}
