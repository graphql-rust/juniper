//! GraphQL support for [`serde_json::Value`].

use std::marker::PhantomData;

use graphql_parser::{
    parse_schema,
    query::{Text, Type},
    schema::{Definition, TypeDefinition},
};
use serde_json::Value as Json;

use juniper::{
    Arguments,
    BoxFuture,
    ExecutionResult,
    Executor, FieldError, FromInputValue, GraphQLType, GraphQLValue, GraphQLValueAsync, InputValue,
    marker::{IsInputType, IsOutputType}, meta::{Argument, Field, MetaType}, Registry, ScalarValue, Selection, types::base::resolve_selection_set_into, Value,
};

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
        let mut input_fields = Vec::new();
        let s = self.schema.clone().unwrap_or_default();
        let ast = parse_schema::<&str>(s.as_str()).unwrap();
        let mut is_input_object = false;
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
                    TypeDefinition::InputObject(d) => {
                        if d.name == self.name {
                            is_input_object = true;
                            for field in &d.fields {
                                let f = self.build_field(
                                    registry,
                                    field.name,
                                    field.value_type.clone(),
                                    true,
                                );

                                input_fields.push(Argument {
                                    name: field.name.to_string(),
                                    description: field.description.clone(),
                                    arg_type: f.field_type,
                                    default_value: None,
                                });
                            }
                        }
                    }
                    _ => todo!(),
                },
                _ => {}
            }
        }
        if is_input_object {
            registry
                .build_input_object_type::<Json>(self, &input_fields)
                .into_meta()
        } else {
            registry
                .build_object_type::<Json>(self, &fields)
                .into_meta()
        }
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
                if nullable {
                    field.field_type = juniper::Type::List(Box::new(field.field_type), None);
                } else {
                    field.field_type = juniper::Type::NonNullList(Box::new(field.field_type), None);
                }
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

impl<S> IsOutputType<S> for Json where S: ScalarValue {}

impl<S> IsInputType<S> for Json where S: ScalarValue {}

impl<S> FromInputValue<S> for Json where S: ScalarValue
{
    fn from_input_value(v: &InputValue<S>) -> Option<Self> {
        match v {
            InputValue::Null => {
                Some(Json::Null)
            }
            InputValue::Scalar(x) => {
                Some(if let Some(i) = x.as_int() {
                    Json::Number(serde_json::Number::from(i))
                } else if let Some(f) = x.as_float() {
                    Json::Number(serde_json::Number::from_f64(f).expect("f64 to convert"))
                } else if let Some(b) = x.as_boolean() {
                    Json::Bool(b)
                } else if let Some(s) = x.as_str() {
                    Json::String(s.to_string())
                } else {
                    unreachable!("`ScalarValue` must represent at least one of the GraphQL spec types")
                })
            }
            InputValue::Enum(x) => {
                Some(Json::String(x.clone()))
            }
            InputValue::List(ls) => {
                let v: Vec<Json> = ls.iter().filter_map(|i| i.item.convert()).collect();
                Some(Json::Array(v))
            }
            InputValue::Object(fields) => {
                let mut obj = serde_json::Map::new();
                for field in fields {
                    let v: Option<Json> = field.1.item.convert();
                    if let Some(v) = v {
                        obj.insert(field.0.item.clone(), v);
                    }
                }
                Some(Json::Object(obj))
            }
            InputValue::Variable(_) => {
                None
            }
        }
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


impl<S> GraphQLValueAsync<S> for Json
    where
        Self::TypeInfo: Sync,
        Self::Context: Sync,
        S: ScalarValue + Send + Sync,
{
    fn resolve_async<'a>(
        &'a self,
        info: &'a Self::TypeInfo,
        selection_set: Option<&'a [Selection<S>]>,
        executor: &'a Executor<Self::Context, S>,
    ) -> BoxFuture<'a, ExecutionResult<S>> {
        Box::pin(async move {
            <Json as GraphQLValue<S>>::resolve(self, info, selection_set, executor)
        })
    }

    fn resolve_field_async<'a>(
        &'a self,
        info: &'a Self::TypeInfo,
        field_name: &'a str,
        arguments: &'a Arguments<S>,
        executor: &'a Executor<Self::Context, S>,
    ) -> BoxFuture<'a, ExecutionResult<S>> {
        Box::pin(async move {
            <Json as GraphQLValue<S>>::resolve_field(self, info, field_name, arguments, executor)
        })
    }
}

trait TypedJsonInfo: Send + Sync {
    fn type_name() -> &'static str;
    fn schema() -> &'static str;
}

#[derive(Debug, Clone, PartialEq)]
struct TypedJson<T: TypedJsonInfo> {
    value: serde_json::Value,
    phantom: PhantomData<T>,
}

impl<T, S> IsOutputType<S> for TypedJson<T> where
    S: ScalarValue,
    T: TypedJsonInfo,
{}

impl<T, S> IsInputType<S> for TypedJson<T> where
    S: ScalarValue,
    T: TypedJsonInfo,
{}

impl<T, S> FromInputValue<S> for TypedJson<T> where
    S: ScalarValue,
    T: TypedJsonInfo,
{
    fn from_input_value(v: &InputValue<S>) -> Option<Self> {
        <serde_json::Value as FromInputValue<S>>::from_input_value(v).map(|x| TypedJson { value: x, phantom: PhantomData })
    }
}

impl<T, S> GraphQLValueAsync<S> for TypedJson<T> where
    S: ScalarValue + Send + Sync,
    T: TypedJsonInfo
{}

impl<T, S> GraphQLType<S> for TypedJson<T> where
    S: ScalarValue,
    T: TypedJsonInfo,
{
    fn name(_info: &Self::TypeInfo) -> Option<&str> {
        Some(T::type_name())
    }
    fn meta<'r>(_info: &Self::TypeInfo, registry: &mut Registry<'r, S>) -> MetaType<'r, S>
        where S: 'r,
    {
        TypeInfo
        {
            name: T::type_name().to_string(),
            schema: Some(T::schema().to_string()),
        }.meta(registry)
    }
}

impl<T, S> GraphQLValue<S> for TypedJson<T>
    where S: ScalarValue,
          T: TypedJsonInfo,
{
    type Context = ();
    type TypeInfo = ();
    fn type_name<'i>(&self, _info: &'i Self::TypeInfo) -> Option<&'i str> {
        Some(T::type_name())
    }
    fn resolve(
        &self,
        _info: &Self::TypeInfo,
        _selection: Option<&[Selection<S>]>,
        executor: &Executor<Self::Context, S>,
    ) -> ExecutionResult<S> {
        executor.resolve(&TypeInfo { schema: None, name: T::type_name().to_string() }, &self.value)
    }
}

#[cfg(test)]
mod tests {
    use std::marker::PhantomData;

    use serde_json::json;

    use juniper::{
        integrations::json::{TypedJson, TypedJsonInfo, TypeInfo},
        EmptyMutation, EmptySubscription, execute_sync, FieldResult, graphql_object, graphql_value,
        RootNode, ToInputValue, Variables,
    };

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

    #[tokio::test]
    async fn test_async() {
        let sdl = r#"
            type Query {
                hero: Hero
            }
            type Hero {
                id: String
                name: String
            }
        "#;
        let data = json!({
            "hero": {
                "id": "2001",
                "name": "R2-D2",
            },
        });

        let schema: RootNode<_, _, _> = RootNode::new_with_info(
            data,
            EmptyMutation::new(),
            EmptySubscription::new(),
            TypeInfo {
                name: "Query".to_string(),
                schema: Some(sdl.to_string()),
            },
            (),
            (),
        );

        let doc = r#"{
            hero {
                id
                name
            }
        }"#;
        assert_eq!(
            crate::execute(doc, None, &schema, &Variables::new(), &()).await,
            Ok((
                graphql_value!({"hero": {"id": "2001", "name": "R2-D2"}}),
                vec![],
            )),
        );
    }

    #[test]
    fn test_as_field_of_output_type() {
        // We need a Foo wrapper associate a static SDL to the Foo type which
        struct Foo;
        impl TypedJsonInfo for Foo
        {
            fn type_name() -> &'static str {
                "Foo"
            }
            fn schema() -> &'static str {
                r#"
                type Foo {
                    message: [String]
                }
                "#
            }
        }

        struct Query;
        #[graphql_object()]
        impl Query {
            fn foo() -> FieldResult<TypedJson<Foo>> {
                let data = json!({"message": ["Hello", "World"] });
                Ok(TypedJson { value: data, phantom: PhantomData })
            }
        }
        let schema = juniper::RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());
        // Run the executor.
        let (res, _errors) = juniper::execute_sync(
            "query { foo { message } }",
            None,
            &schema,
            &Variables::new(),
            &(),
        ).unwrap();

        // Ensure the value matches.
        assert_eq!(
            res,
            graphql_value!({
                "foo": {"message":["Hello", "World"]},
            })
        );
    }


    #[test]
    fn test_as_field_of_input_type() {
        #[derive(Debug, Clone, PartialEq)]
        struct Foo;
        impl TypedJsonInfo for Foo
        {
            fn type_name() -> &'static str {
                "Foo"
            }
            fn schema() -> &'static str {
                r#"
                input Foo {
                    message: [String]
                }
                "#
            }
        }

        struct Query;
        #[graphql_object()]
        impl Query {
            fn foo(value: TypedJson<Foo>) -> FieldResult<bool> {
                Ok(value == TypedJson { value: json!({"message":["Hello", "World"]}), phantom: PhantomData })
            }
        }
        let schema = juniper::RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());

        let vars = vec![("value".to_owned(), graphql_value!({
                "message":["Hello", "World"],
            }).to_input_value())]
            .into_iter()
            .collect();

        // Run the executor.
        let (res, _errors) = juniper::execute_sync(
            "query example($value:Foo!){ foo(value: $value) }",
            None,
            &schema,
            &vars,
            &(),
        ).unwrap();

        // Ensure the value matches.
        assert_eq!(
            res,
            graphql_value!({
                "foo": true,
            })
        );
    }
}


