use arcstr::ArcStr;
use indexmap::IndexMap;

use crate::{
    executor::{ExecutionResult, Executor, Registry},
    graphql_value, graphql_vars,
    schema::{meta::MetaType, model::RootNode},
    types::{
        base::{Arguments, GraphQLType, GraphQLValue},
        scalars::{EmptyMutation, EmptySubscription},
    },
    value::ScalarValue,
};

pub struct NodeTypeInfo {
    name: ArcStr,
    attribute_names: Vec<ArcStr>,
}

pub struct Node {
    attributes: IndexMap<String, String>,
}

impl<S> GraphQLType<S> for Node
where
    S: ScalarValue,
{
    fn name(info: &Self::TypeInfo) -> Option<ArcStr> {
        Some(info.name.clone())
    }

    fn meta(info: &Self::TypeInfo, registry: &mut Registry<S>) -> MetaType<S> {
        let fields = info
            .attribute_names
            .iter()
            .map(|name| registry.field::<String>(name.clone(), &()))
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

    fn type_name<'i>(&self, info: &'i Self::TypeInfo) -> Option<ArcStr> {
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
        name: "MyNode".into(),
        attribute_names: vec!["foo".into(), "bar".into(), "baz".into()],
    };
    let mut node = Node {
        attributes: IndexMap::new(),
    };
    node.attributes.insert("foo".into(), "1".into());
    node.attributes.insert("bar".into(), "2".into());
    node.attributes.insert("baz".into(), "3".into());
    let schema: RootNode<_, _, _> = RootNode::new_with_info(
        node,
        EmptyMutation::new(),
        EmptySubscription::new(),
        node_info,
        (),
        (),
    );

    assert_eq!(
        crate::execute_sync(doc, None, &schema, &graphql_vars! {}, &()),
        Ok((
            graphql_value!({
                "foo": "1",
                "bar": "2",
                "baz": "3",
            }),
            vec![],
        )),
    );
}
