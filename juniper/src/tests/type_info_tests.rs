use indexmap::IndexMap;

use crate::{
    executor::{ExecutionResult, Executor, Registry, Variables},
    schema::{meta::MetaType, model::RootNode},
    types::{
        base::{Arguments, GraphQLType, GraphQLValue},
        scalars::{EmptyMutation, EmptySubscription},
    },
    value::{ScalarValue, Value},
};

pub struct NodeTypeInfo {
    name: String,
    attribute_names: Vec<String>,
}

pub struct Node {
    attributes: IndexMap<String, String>,
}

impl<S> GraphQLType<S> for Node
where
    S: ScalarValue,
{
    fn name(info: &Self::TypeInfo) -> Option<&str> {
        Some(&info.name)
    }

    fn meta<'r>(info: &Self::TypeInfo, registry: &mut Registry<'r, S>) -> MetaType<'r, S>
    where
        S: 'r,
    {
        let fields = info
            .attribute_names
            .iter()
            .map(|name| registry.field::<String>(name, &()))
            .collect::<Vec<_>>();

        registry
            .build_object_type::<Node>(info, &fields)
            .into_meta()
    }
}

impl<S> GraphQLValue<S> for Node
where
    S: ScalarValue,
{
    type Context = ();
    type TypeInfo = NodeTypeInfo;

    fn type_name<'i>(&self, info: &'i Self::TypeInfo) -> Option<&'i str> {
        <Self as GraphQLType<S>>::name(info)
    }

    fn resolve_field(
        &self,
        _: &Self::TypeInfo,
        field_name: &str,
        _: &Arguments<S>,
        executor: &Executor<Self::Context, S>,
    ) -> ExecutionResult<S> {
        executor.resolve(&(), &self.attributes.get(field_name).unwrap())
    }
}

#[test]
fn test_node() {
    let doc = r#"
        {
            foo,
            bar,
            baz
        }"#;
    let node_info = NodeTypeInfo {
        name: "MyNode".to_string(),
        attribute_names: vec!["foo".to_string(), "bar".to_string(), "baz".to_string()],
    };
    let mut node = Node {
        attributes: IndexMap::new(),
    };
    node.attributes.insert("foo".to_string(), "1".to_string());
    node.attributes.insert("bar".to_string(), "2".to_string());
    node.attributes.insert("baz".to_string(), "3".to_string());
    let schema: RootNode<_, _, _> = RootNode::new_with_info(
        node,
        EmptyMutation::new(),
        EmptySubscription::new(),
        node_info,
        (),
        (),
    );

    assert_eq!(
        crate::execute_sync(doc, None, &schema, &Variables::new(), &()),
        Ok((
            Value::object(
                vec![
                    ("foo", Value::scalar("1")),
                    ("bar", Value::scalar("2")),
                    ("baz", Value::scalar("3")),
                ]
                .into_iter()
                .collect()
            ),
            vec![]
        ))
    );
}
