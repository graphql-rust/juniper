//! GraphQL support for [`serde_json::Value`].

use graphql_parser::{
    parse_schema,
    query::{Text, Type},
    schema::{Definition, TypeDefinition},
};
use juniper::{
    meta::{Field, MetaType},
    Arguments, ExecutionResult, Executor, FieldError, GraphQLType, GraphQLValue, Registry,
    ScalarValue, Selection, Value,
};
use serde_json::Value as Json;

use crate::{types::base::resolve_selection_set_into, GraphQLValueAsync};

// Used to describe the graphql type of a `serde_json::Value` using the GraphQL
// schema definition language.
/// [`GraphQLValue::TypeInfo`] of [`Json`] using the GraphQL schema definition
/// language.
#[derive(Debug, Clone, PartialEq)]
pub struct TypeInfo {
    /// Schema definition language containing a definition of the type name.
    pub schema: Option<String>,

    /// Type name of the GraphQL value
    pub name: String,
}

impl TypeInfo {
    fn meta<'r, S>(&self, registry: &mut Registry<'r, S>) -> MetaType<'r, S>
    where
        S: ScalarValue + 'r,
    {
        let mut fields = Vec::new();
        let s = self.schema.clone().unwrap_or_default();
        let ast = parse_schema::<&str>(s.as_str()).unwrap();
        for d in &ast.definitions {
            match &d {
                Definition::TypeDefinition(d) => match d {
                    TypeDefinition::Object(d) => {
                        if d.name == self.name {
                            for field in &d.fields {
                                fields.push(self.build_field(
                                    registry,
                                    field.name,
                                    field.field_type.clone(),
                                    true,
                                ));
                            }
                        }
                    }
                    _ => todo!(),
                },
                _ => {}
            }
        }
        registry
            .build_object_type::<Json>(self, &fields)
            .into_meta()
    }

    fn build_field<'r, 't, S, T>(
        &self,
        registry: &mut Registry<'r, S>,
        field_name: &str,
        type_ref: Type<'t, T>,
        nullable: bool,
    ) -> Field<'r, S>
    where
        S: 'r + ScalarValue,
        T: Text<'t>,
    {
        match type_ref {
            Type::NamedType(type_name) => match type_name.as_ref() {
                "String" => {
                    if nullable {
                        registry.field::<Option<String>>(field_name, &())
                    } else {
                        registry.field::<String>(field_name, &())
                    }
                }
                "Int" => {
                    if nullable {
                        registry.field::<Option<i32>>(field_name, &())
                    } else {
                        registry.field::<i32>(field_name, &())
                    }
                }
                "Float" => {
                    if nullable {
                        registry.field::<Option<f64>>(field_name, &())
                    } else {
                        registry.field::<f64>(field_name, &())
                    }
                }
                "Boolean" => {
                    if nullable {
                        registry.field::<Option<bool>>(field_name, &())
                    } else {
                        registry.field::<bool>(field_name, &())
                    }
                }
                _ => {
                    let field_node_type_info = &TypeInfo {
                        schema: self.schema.clone(),
                        name: type_name.clone().as_ref().to_string(),
                    };
                    if nullable {
                        registry.field::<Option<Json>>(field_name, field_node_type_info)
                    } else {
                        registry.field::<Json>(field_name, field_node_type_info)
                    }
                }
            },
            Type::ListType(nested_type) => {
                let mut field = self.build_field(registry, field_name, *nested_type, true);
                field.field_type = juniper::Type::List(Box::new(field.field_type), None);
                field
            }
            Type::NonNullType(nested_type) => {
                self.build_field(registry, field_name, *nested_type, false)
            }
        }
    }
}

impl<S: ScalarValue> GraphQLType<S> for Json {
    fn name(info: &Self::TypeInfo) -> Option<&str> {
        Some(info.name.as_str())
    }

    fn meta<'r>(info: &Self::TypeInfo, registry: &mut Registry<'r, S>) -> MetaType<'r, S>
    where
        S: 'r,
    {
        info.meta(registry)
    }
}

impl<S: ScalarValue> GraphQLValue<S> for Json {
    type Context = ();
    type TypeInfo = TypeInfo;

    fn type_name<'i>(&self, info: &'i Self::TypeInfo) -> Option<&'i str> {
        Some(info.name.as_str())
    }

    fn resolve(
        &self,
        info: &Self::TypeInfo,
        selection: Option<&[Selection<S>]>,
        executor: &Executor<Self::Context, S>,
    ) -> ExecutionResult<S> {
        if let Some(sel) = selection {
            // resolve this value as an object
            let mut res = juniper::Object::with_capacity(sel.len());
            Ok(
                if resolve_selection_set_into(self, info, sel, executor, &mut res) {
                    Value::Object(res)
                } else {
                    Value::null()
                },
            )
        } else {
            // resolve this value as leaf
            match self {
                Json::Null => Ok(Value::null()),
                Json::Bool(value) => executor.resolve::<bool>(&(), value),
                Json::Number(value) => {
                    if value.is_f64() {
                        executor.resolve::<f64>(&(), &value.as_f64().unwrap())
                    } else if value.is_i64() {
                        executor.resolve::<i32>(&(), &(value.as_i64().unwrap() as i32))
                    } else if value.is_u64() {
                        executor.resolve::<i32>(&(), &(value.as_u64().unwrap() as i32))
                    } else {
                        panic!("invalid number")
                    }
                }
                Json::String(value) => executor.resolve::<String>(&(), value),
                _ => Err(FieldError::new("not a leaf value", Value::Null)),
            }
        }
    }

    fn resolve_field(
        &self,
        _info: &Self::TypeInfo,
        field_name: &str,
        _: &Arguments<S>,
        executor: &Executor<Self::Context, S>,
    ) -> ExecutionResult<S> {
        match self {
            Json::Object(fields) => {
                let field_value = fields.get(field_name);
                match field_value {
                    None => Ok(Value::null()),
                    Some(field_value) => {
                        let current_type = executor.current_type();
                        let field_info = &TypeInfo {
                            schema: None,
                            name: current_type
                                .innermost_concrete()
                                .name()
                                .unwrap()
                                .to_string(),
                        };
                        if current_type.list_contents().is_some() {
                            match field_value {
                                Json::Null => Ok(Value::null()),
                                Json::Array(field_value) => {
                                    executor.resolve::<Vec<Json>>(field_info, field_value)
                                }
                                _ => Err(FieldError::new("not an array", Value::Null)),
                            }
                        } else {
                            executor.resolve::<Json>(field_info, &field_value)
                        }
                    }
                }
            }
            _ => Err(FieldError::new("not an object value", Value::Null)),
        }
    }
}

#[cfg(test)]
mod tests {
    use juniper::{
        execute_sync, graphql_value, EmptyMutation, EmptySubscription, RootNode, Variables,
    };
    use serde_json::json;

    use super::TypeInfo;

    #[test]
    fn sdl_type_info() {
        let sdl = r#"
            type Bar {
                location: String
                capacity: Int
                open: Boolean!
                rating: Float
                foo: Foo
            }
            type Foo {
                message: String
                bar: Bar
            }
        "#;

        let info = TypeInfo {
            name: "Foo".to_string(),
            schema: Some(sdl.to_string()),
        };

        let data = json!({
            "message": "hello world",
            "bar": {
                    "location": "downtown",
                    "capacity": 80,
                    "open": true,
                    "rating": 4.5,
                    "foo": {
                        "message": "drink more"
                    }
                },
        });

        let schema = <RootNode<_, _, _>>::new_with_info(
            data,
            EmptyMutation::new(),
            EmptySubscription::new(),
            info,
            (),
            (),
        );

        // print!("{}", schema.as_schema_language());

        let query = r#"{
            message
            bar {
                location
                capacity
                open
                rating
                foo {
                    message
                }
            }
        }"#;

        assert_eq!(
            execute_sync(query, None, &schema, &Variables::new(), &()),
            Ok((
                graphql_value!({
                    "message": "hello world",
                    "bar": {
                        "location": "downtown",
                        "capacity": 80,
                        "open": true,
                        "rating": 4.5,
                        "foo": {
                            "message": "drink more"
                        }
                    }
                }),
                vec![],
            )),
        );
    }

    #[test]
    fn required_field() {
        let sdl = r#"
            type Bar {
                location: String
                open: Boolean!
            }
            type Foo {
                message: String
                bar: Bar
            }
        "#;

        let info = TypeInfo {
            name: "Foo".to_string(),
            schema: Some(sdl.to_string()),
        };

        let data = json!({
            "message": "hello world",
            "bar": {
                "capacity": 80,
            },
        });

        let schema = <RootNode<_, _, _>>::new_with_info(
            data,
            EmptyMutation::new(),
            EmptySubscription::new(),
            info,
            (),
            (),
        );

        let query = r#"{
            message
            bar {
                location
                open
            }
        }"#;

        assert_eq!(
            execute_sync(query, None, &schema, &Variables::new(), &()),
            Ok((
                graphql_value!({
                    "message": "hello world",
                    "bar": None,
                }),
                vec![],
            )),
        );
    }

    #[test]
    fn array_field() {
        let sdl = r#"
            type Bar {
                location: [String]
                open: [Boolean!]
            }
            type Foo {
                message: [String]
                bar: [Bar]
            }
        "#;

        let info = TypeInfo {
            name: "Foo".to_string(),
            schema: Some(sdl.to_string()),
        };

        let data = json!({
            "message": ["hello world"],
            "bar": [{
                "location": ["Tampa"],
            }],
        });

        let schema: RootNode<_, _, _> = RootNode::new_with_info(
            data,
            EmptyMutation::new(),
            EmptySubscription::new(),
            info,
            (),
            (),
        );

        // print!("{}", schema.as_schema_language());

        let query = r#"{
            message
            bar {
                location
            }
        }"#;

        assert_eq!(
            execute_sync(query, None, &schema, &Variables::new(), &()),
            Ok((
                graphql_value!({
                    "message": ["hello world"],
                    "bar": [{
                        "location": ["Tampa"],
                    }],
                }),
                vec![],
            )),
        );
    }
}
