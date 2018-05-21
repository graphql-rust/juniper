use indexmap::IndexMap;

use executor::{ExecutionResult, Executor, Registry, Variables};
use schema::meta::MetaType;
use schema::model::RootNode;
use types::base::{Arguments, GraphQLType};
use types::scalars::EmptyMutation;
use value::Value;

pub struct NodeTypeInfo {
    name: String,
    attribute_names: Vec<String>,
}

pub struct Node {
    attributes: IndexMap<String, String>,
}

impl GraphQLType for Node {
    type Context = ();
    type TypeInfo = NodeTypeInfo;

    fn name(info: &Self::TypeInfo) -> Option<&str> {
        Some(&info.name)
    }

    fn meta<'r>(info: &Self::TypeInfo, registry: &mut Registry<'r>) -> MetaType<'r> {
        let fields = info.attribute_names
            .iter()
            .map(|name| registry.field::<String>(name, &()))
            .collect::<Vec<_>>();

        registry
            .build_object_type::<Node>(info, &fields)
            .into_meta()
    }

    fn resolve_field(
        &self,
        _: &Self::TypeInfo,
        field_name: &str,
        _: &Arguments,
        executor: &Executor<Self::Context>,
    ) -> ExecutionResult {
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
    let schema = RootNode::new_with_info(node, EmptyMutation::new(), node_info, ());

    assert_eq!(
        ::execute(doc, None, &schema, &Variables::new(), &()),
        Ok((
            Value::object(
                vec![
                    ("foo", Value::string("1")),
                    ("bar", Value::string("2")),
                    ("baz", Value::string("3")),
                ].into_iter()
                    .collect()
            ),
            vec![]
        ))
    );
}
