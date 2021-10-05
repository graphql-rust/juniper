//! GraphQL support for [`serde_json::Value`].

use std::{
    convert::{TryFrom as _, TryInto as _},
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use futures::future;
use graphql_parser::{
    parse_schema,
    query::{Text, Type},
    schema::{Definition, TypeDefinition},
};
use ref_cast::RefCast;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::{
    ast,
    marker::{IsInputType, IsOutputType},
    meta::MetaType,
    parser::ScalarToken,
    Arguments, BoxFuture, ExecutionResult, Executor, FieldError, FromInputValue, GraphQLType,
    GraphQLValue, GraphQLValueAsync, InputValue, IntoFieldError, ParseScalarResult,
    ParseScalarValue, Registry, ScalarValue, Selection, Spanning, ToInputValue,
};

pub use serde_json::{Error, Value};

impl<S: ScalarValue> IntoFieldError<S> for Error {
    fn into_field_error(self) -> FieldError<S> {
        self.into()
    }
}

impl<S: ScalarValue> IsInputType<S> for Value {}

impl<S: ScalarValue> IsOutputType<S> for Value {}

impl<S: ScalarValue> GraphQLType<S> for Value {
    fn name(_: &Self::TypeInfo) -> Option<&str> {
        Some("Json")
    }

    fn meta<'r>(info: &Self::TypeInfo, registry: &mut Registry<'r, S>) -> MetaType<'r, S>
    where
        S: 'r,
    {
        registry
            .build_scalar_type::<Self>(info)
            .description("Opaque JSON value.")
            .into_meta()
    }
}

impl<S: ScalarValue> GraphQLValue<S> for Value {
    type Context = ();
    type TypeInfo = ();

    fn type_name<'i>(&self, info: &'i Self::TypeInfo) -> Option<&'i str> {
        <Self as GraphQLType<S>>::name(info)
    }

    fn resolve(
        &self,
        info: &Self::TypeInfo,
        selection: Option<&[Selection<S>]>,
        executor: &Executor<Self::Context, S>,
    ) -> ExecutionResult<S> {
        use serde::ser::Error as _;

        match self {
            Self::Null => Ok(crate::Value::null()),
            Self::Bool(b) => executor.resolve(&(), &b),
            Self::Number(n) => {
                if let Some(n) = n.as_u64() {
                    executor.resolve::<i32>(&(), &n.try_into().map_err(serde_json::Error::custom)?)
                } else if let Some(n) = n.as_i64() {
                    executor.resolve::<i32>(&(), &n.try_into().map_err(serde_json::Error::custom)?)
                } else if let Some(n) = n.as_f64() {
                    executor.resolve(&(), &n)
                } else {
                    unreachable!("serde_json::Number has only 3 number variants")
                }
            }
            Self::String(s) => executor.resolve(&(), &s),
            Self::Array(arr) => Ok(crate::Value::list(
                arr.iter()
                    .map(|v| executor.resolve(info, v))
                    .collect::<Result<_, _>>()?,
            )),
            Self::Object(obj) => {
                // If selection set is none we should still output all the
                // object fields.
                let full_selection = selection
                    .is_none()
                    .then(|| {
                        obj.keys()
                            .map(|k| {
                                Selection::Field(Spanning::unlocated(ast::Field {
                                    alias: None,
                                    name: Spanning::unlocated(&*k),
                                    arguments: None,
                                    directives: None,
                                    selection_set: None,
                                }))
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                let selection = selection.unwrap_or(&full_selection);

                let mut out = crate::Object::with_capacity(selection.len());
                for sel in selection {
                    match sel {
                        Selection::Field(Spanning {
                            item: f,
                            start: start_pos,
                            ..
                        }) => {
                            let resp_name = f.alias.as_ref().unwrap_or(&f.name).item;
                            let sub_exec = executor.field_with_parent_type_sub_executor(
                                resp_name,
                                *start_pos,
                                f.selection_set.as_ref().map(|v| &v[..]),
                            );
                            let _ = out.add_field(
                                resp_name,
                                self.resolve_field(
                                    info,
                                    f.name.item,
                                    &Arguments::new(None, &None),
                                    &sub_exec,
                                )?,
                            );
                        }
                        _ => {
                            return Err(FieldError::new(
                                "spreading fragments on opaque JSON value is not supported",
                                crate::Value::null(),
                            ))
                        }
                    }
                }
                Ok(crate::Value::Object(out))
            }
        }
    }

    fn resolve_field(
        &self,
        info: &Self::TypeInfo,
        field_name: &str,
        _: &Arguments<S>,
        executor: &Executor<Self::Context, S>,
    ) -> ExecutionResult<S> {
        match self {
            Self::Object(obj) => match obj.get(field_name) {
                None => Ok(crate::Value::null()),
                Some(field) => executor.resolve(info, field),
            },
            _ => Err(FieldError::new("not an object value", crate::Value::null())),
        }
    }
}

impl<S: ScalarValue + Send + Sync> GraphQLValueAsync<S> for Value {
    fn resolve_async<'a>(
        &'a self,
        info: &'a Self::TypeInfo,
        selection: Option<&'a [Selection<S>]>,
        executor: &'a Executor<Self::Context, S>,
    ) -> BoxFuture<'a, ExecutionResult<S>> {
        Box::pin(future::ready(self.resolve(info, selection, executor)))
    }

    fn resolve_field_async<'a>(
        &'a self,
        info: &'a Self::TypeInfo,
        field_name: &'a str,
        arguments: &'a Arguments<S>,
        executor: &'a Executor<Self::Context, S>,
    ) -> BoxFuture<'a, ExecutionResult<S>> {
        Box::pin(future::ready(
            self.resolve_field(info, field_name, arguments, executor),
        ))
    }
}

impl<S: ScalarValue> ToInputValue<S> for Value {
    fn to_input_value(&self) -> InputValue<S> {
        match self {
            Self::Null => InputValue::null(),
            Self::Bool(b) => InputValue::scalar(*b),
            Self::Number(n) => {
                if let Some(n) = n.as_u64() {
                    InputValue::scalar(i32::try_from(n).expect("i32 number"))
                } else if let Some(n) = n.as_i64() {
                    InputValue::scalar(i32::try_from(n).expect("i32 number"))
                } else if let Some(n) = n.as_f64() {
                    InputValue::scalar(n)
                } else {
                    unreachable!("serde_json::Number has only 3 number variants")
                }
            }
            Self::String(s) => InputValue::scalar(s.clone()),
            Self::Array(arr) => InputValue::list(arr.iter().map(Self::to_input_value).collect()),
            Self::Object(obj) => {
                InputValue::object(obj.iter().map(|(k, v)| (k, v.to_input_value())).collect())
            }
        }
    }
}

impl<S: ScalarValue> FromInputValue<S> for Value {
    fn from_input_value(val: &InputValue<S>) -> Option<Self> {
        match val {
            InputValue::Null => Some(Self::Null),
            InputValue::Scalar(x) => Some(if let Some(i) = x.as_int() {
                Self::Number(serde_json::Number::from(i))
            } else if let Some(f) = x.as_float() {
                Self::Number(serde_json::Number::from_f64(f).expect("f64 to convert"))
            } else if let Some(b) = x.as_boolean() {
                Self::Bool(b)
            } else if let Some(s) = x.as_str() {
                Self::String(s.into())
            } else {
                unreachable!("`ScalarValue` must represent at least one of the GraphQL spec types")
            }),
            InputValue::Enum(x) => Some(Self::String(x.clone())),
            InputValue::List(ls) => Some(Self::Array(
                ls.iter().filter_map(|i| i.item.convert()).collect(),
            )),
            InputValue::Object(fs) => Some(Self::Object(
                fs.iter()
                    .filter_map(|(n, v)| Some((n.item.clone(), v.item.convert()?)))
                    .collect(),
            )),
            InputValue::Variable(_) => None,
        }
    }
}

impl<S: ScalarValue> ParseScalarValue<S> for Value {
    fn from_str(val: ScalarToken<'_>) -> ParseScalarResult<'_, S> {
        match val {
            ScalarToken::String(_) => <String as ParseScalarValue<S>>::from_str(val),
            ScalarToken::Float(_) => <f64 as ParseScalarValue<S>>::from_str(val),
            ScalarToken::Int(_) => <i32 as ParseScalarValue<S>>::from_str(val),
        }
    }
}

#[derive(Clone, Deserialize, Copy, Debug, RefCast, Serialize)]
#[repr(transparent)]
pub struct Json<T: ?Sized = Value>(pub T);

impl<T> From<T> for Json<T> {
    fn from(val: T) -> Self {
        Self(val)
    }
}

impl<T> Json<T> {
    /// Unwraps into the underlying value of this [`Json`] wrapper.
    #[must_use]
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T: ?Sized> Deref for Json<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: ?Sized> DerefMut for Json<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T, S> IsInputType<S> for Json<T>
where
    T: DeserializeOwned + Serialize,
    S: ScalarValue,
{
}

impl<T, S> IsOutputType<S> for Json<T>
where
    T: Serialize + ?Sized,
    S: ScalarValue,
{
}

impl<T, S> GraphQLType<S> for Json<T>
where
    T: Serialize + ?Sized,
    S: ScalarValue,
{
    fn name(info: &Self::TypeInfo) -> Option<&str> {
        <Value as GraphQLType<S>>::name(info)
    }

    fn meta<'r>(info: &Self::TypeInfo, registry: &mut Registry<'r, S>) -> MetaType<'r, S>
    where
        S: 'r,
    {
        <Value as GraphQLType<S>>::meta(info, registry)
    }
}

impl<T, S> GraphQLValue<S> for Json<T>
where
    T: Serialize + ?Sized,
    S: ScalarValue,
{
    type Context = ();
    type TypeInfo = ();

    fn type_name<'i>(&self, info: &'i Self::TypeInfo) -> Option<&'i str> {
        <Self as GraphQLType<S>>::name(info)
    }

    fn resolve(
        &self,
        info: &Self::TypeInfo,
        selection: Option<&[Selection<S>]>,
        executor: &Executor<Self::Context, S>,
    ) -> ExecutionResult<S> {
        serde_json::to_value(&self.0)?.resolve(info, selection, executor)
    }

    fn resolve_field(
        &self,
        info: &Self::TypeInfo,
        field_name: &str,
        args: &Arguments<S>,
        executor: &Executor<Self::Context, S>,
    ) -> ExecutionResult<S> {
        serde_json::to_value(&self.0)?.resolve_field(info, field_name, args, executor)
    }
}

impl<T, S> GraphQLValueAsync<S> for Json<T>
where
    T: Serialize + Sync + ?Sized,
    S: ScalarValue + Send + Sync,
{
    fn resolve_async<'a>(
        &'a self,
        info: &'a Self::TypeInfo,
        selection: Option<&'a [Selection<S>]>,
        executor: &'a Executor<Self::Context, S>,
    ) -> BoxFuture<'a, ExecutionResult<S>> {
        Box::pin(future::ready(self.resolve(info, selection, executor)))
    }

    fn resolve_field_async<'a>(
        &'a self,
        info: &'a Self::TypeInfo,
        field_name: &'a str,
        arguments: &'a Arguments<S>,
        executor: &'a Executor<Self::Context, S>,
    ) -> BoxFuture<'a, ExecutionResult<S>> {
        Box::pin(future::ready(
            self.resolve_field(info, field_name, arguments, executor),
        ))
    }
}

impl<T, S> ToInputValue<S> for Json<T>
where
    T: Serialize,
    S: ScalarValue,
{
    fn to_input_value(&self) -> InputValue<S> {
        serde_json::to_value(&self.0)
            .expect("Failed to serialize")
            .to_input_value()
    }
}

impl<T, S> FromInputValue<S> for Json<T>
where
    T: DeserializeOwned,
    S: ScalarValue,
{
    fn from_input_value(val: &InputValue<S>) -> Option<Self> {
        serde_json::from_value(<Value as FromInputValue<S>>::from_input_value(val)?)
            .ok()
            .map(Self)
    }
}

impl<T, S> ParseScalarValue<S> for Json<T>
where
    T: ?Sized,
    S: ScalarValue,
{
    fn from_str(val: ScalarToken<'_>) -> ParseScalarResult<'_, S> {
        <Value as ParseScalarValue<S>>::from_str(val)
    }
}

#[cfg(test)]
mod value_test {
    mod as_output {
        use futures::FutureExt as _;
        use serde_json::{json, Value};

        use crate::{
            execute, execute_sync, graphql_object, graphql_subscription, resolve_into_stream,
            tests::util::{extract_next, stream, Stream},
            EmptyMutation, EmptySubscription, FieldResult, RootNode, Variables,
        };

        struct Query;

        #[graphql_object]
        impl Query {
            fn null() -> Value {
                Value::Null
            }

            fn bool() -> Value {
                json!(true)
            }

            fn int() -> Value {
                json!(42)
            }

            fn float() -> Value {
                json!(3.14)
            }

            fn string() -> Value {
                json!("Galadriel")
            }

            fn array() -> Value {
                json!(["Ai", "Ambarendya!"])
            }

            fn object() -> Value {
                json!({"message": ["Ai", "Ambarendya!"]})
            }

            fn nullable() -> Option<Value> {
                Some(json!({"message": ["Ai", "Ambarendya!"]}))
            }

            fn fallible() -> FieldResult<Value> {
                Ok(json!({"message": ["Ai", "Ambarendya!"]}))
            }

            fn nested() -> Value {
                json!({"message": {
                    "header": "Ai",
                    "body": "Ambarendya!",
                }})
            }
        }

        struct Subscription;

        #[graphql_subscription]
        impl Subscription {
            async fn null() -> Stream<Value> {
                stream(Value::Null)
            }

            async fn bool() -> Stream<Value> {
                stream(json!(true))
            }

            async fn int() -> Stream<Value> {
                stream(json!(42))
            }

            async fn float() -> Stream<Value> {
                stream(json!(3.14))
            }

            async fn string() -> Stream<Value> {
                stream(json!("Galadriel"))
            }

            async fn array() -> Stream<Value> {
                stream(json!(["Ai", "Ambarendya!"]))
            }

            async fn object() -> Stream<Value> {
                stream(json!({"message": ["Ai", "Ambarendya!"]}))
            }

            async fn nullable() -> Stream<Option<Value>> {
                stream(Some(json!({"message": ["Ai", "Ambarendya!"]})))
            }

            async fn fallible() -> FieldResult<Stream<FieldResult<Value>>> {
                Ok(stream(Ok(json!({"message": ["Ai", "Ambarendya!"]}))))
            }

            async fn nested() -> Stream<Value> {
                stream(json!({"message": {
                    "header": "Ai",
                    "body": "Ambarendya!",
                }}))
            }
        }

        #[tokio::test]
        async fn resolves_null() {
            const QRY: &str = "{ null }";
            const SUB: &str = "subscription { null }";

            let schema = RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());
            assert_eq!(
                execute(QRY, None, &schema, &Variables::new(), &()).await,
                Ok((graphql_value!({ "null": None }), vec![])),
            );
            assert_eq!(
                execute_sync(QRY, None, &schema, &Variables::new(), &()),
                Ok((graphql_value!({ "null": None }), vec![])),
            );

            let schema = RootNode::new(Query, EmptyMutation::new(), Subscription);
            assert_eq!(
                resolve_into_stream(SUB, None, &schema, &Variables::new(), &())
                    .then(|s| extract_next(s))
                    .await,
                Ok((graphql_value!({ "null": None }), vec![])),
            );
        }

        #[tokio::test]
        async fn resolves_bool() {
            const QRY: &str = "{ bool }";
            const SUB: &str = "subscription { bool }";

            let schema = RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());
            assert_eq!(
                execute(QRY, None, &schema, &Variables::new(), &()).await,
                Ok((graphql_value!({"bool": true}), vec![])),
            );
            assert_eq!(
                execute_sync(QRY, None, &schema, &Variables::new(), &()),
                Ok((graphql_value!({"bool": true}), vec![])),
            );

            let schema = RootNode::new(Query, EmptyMutation::new(), Subscription);
            assert_eq!(
                resolve_into_stream(SUB, None, &schema, &Variables::new(), &())
                    .then(|s| extract_next(s))
                    .await,
                Ok((graphql_value!({"bool": true}), vec![])),
            );
        }

        #[tokio::test]
        async fn resolves_int() {
            const QRY: &str = "{ int }";
            const SUB: &str = "subscription { int }";

            let schema = RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());
            assert_eq!(
                execute(QRY, None, &schema, &Variables::new(), &()).await,
                Ok((graphql_value!({"int": 42}), vec![])),
            );
            assert_eq!(
                execute_sync(QRY, None, &schema, &Variables::new(), &()),
                Ok((graphql_value!({"int": 42}), vec![])),
            );

            let schema = RootNode::new(Query, EmptyMutation::new(), Subscription);
            assert_eq!(
                resolve_into_stream(SUB, None, &schema, &Variables::new(), &())
                    .then(|s| extract_next(s))
                    .await,
                Ok((graphql_value!({"int": 42}), vec![])),
            );
        }

        #[tokio::test]
        async fn resolves_float() {
            const QRY: &str = "{ float }";
            const SUB: &str = "subscription { float }";

            let schema = RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());
            assert_eq!(
                execute(QRY, None, &schema, &Variables::new(), &()).await,
                Ok((graphql_value!({"float": 3.14}), vec![])),
            );
            assert_eq!(
                execute_sync(QRY, None, &schema, &Variables::new(), &()),
                Ok((graphql_value!({"float": 3.14}), vec![])),
            );

            let schema = RootNode::new(Query, EmptyMutation::new(), Subscription);
            assert_eq!(
                resolve_into_stream(SUB, None, &schema, &Variables::new(), &())
                    .then(|s| extract_next(s))
                    .await,
                Ok((graphql_value!({"float": 3.14}), vec![])),
            );
        }

        #[tokio::test]
        async fn resolves_string() {
            const QRY: &str = "{ string }";
            const SUB: &str = "subscription { string }";

            let schema = RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());
            assert_eq!(
                execute(QRY, None, &schema, &Variables::new(), &()).await,
                Ok((graphql_value!({"string": "Galadriel"}), vec![])),
            );
            assert_eq!(
                execute_sync(QRY, None, &schema, &Variables::new(), &()),
                Ok((graphql_value!({"string": "Galadriel"}), vec![])),
            );

            let schema = RootNode::new(Query, EmptyMutation::new(), Subscription);
            assert_eq!(
                resolve_into_stream(SUB, None, &schema, &Variables::new(), &())
                    .then(|s| extract_next(s))
                    .await,
                Ok((graphql_value!({"string": "Galadriel"}), vec![])),
            );
        }

        #[tokio::test]
        async fn resolves_array() {
            const QRY: &str = "{ array }";
            const SUB: &str = "subscription { array }";

            let schema = RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());
            assert_eq!(
                execute(QRY, None, &schema, &Variables::new(), &()).await,
                Ok((graphql_value!({"array": ["Ai", "Ambarendya!"]}), vec![])),
            );
            assert_eq!(
                execute_sync(QRY, None, &schema, &Variables::new(), &()),
                Ok((graphql_value!({"array": ["Ai", "Ambarendya!"]}), vec![])),
            );

            let schema = RootNode::new(Query, EmptyMutation::new(), Subscription);
            assert_eq!(
                resolve_into_stream(SUB, None, &schema, &Variables::new(), &())
                    .then(|s| extract_next(s))
                    .await,
                Ok((graphql_value!({"array": ["Ai", "Ambarendya!"]}), vec![])),
            );
        }

        #[tokio::test]
        async fn resolves_object() {
            const QRY: &str = "{ object }";
            const SUB: &str = "subscription { object }";

            let schema = RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());
            assert_eq!(
                execute(QRY, None, &schema, &Variables::new(), &()).await,
                Ok((
                    graphql_value!({
                        "object": {"message": ["Ai", "Ambarendya!"]},
                    }),
                    vec![],
                )),
            );
            assert_eq!(
                execute_sync(QRY, None, &schema, &Variables::new(), &()),
                Ok((
                    graphql_value!({
                        "object": {"message": ["Ai", "Ambarendya!"]},
                    }),
                    vec![],
                )),
            );

            let schema = RootNode::new(Query, EmptyMutation::new(), Subscription);
            assert_eq!(
                resolve_into_stream(SUB, None, &schema, &Variables::new(), &())
                    .then(|s| extract_next(s))
                    .await,
                Ok((
                    graphql_value!({
                        "object": {"message": ["Ai", "Ambarendya!"]},
                    }),
                    vec![],
                )),
            );
        }

        #[tokio::test]
        async fn resolves_nullable() {
            const QRY: &str = "{ nullable }";
            const SUB: &str = "subscription { nullable }";

            let schema = RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());
            assert_eq!(
                execute(QRY, None, &schema, &Variables::new(), &()).await,
                Ok((
                    graphql_value!({
                        "nullable": {"message": ["Ai", "Ambarendya!"]},
                    }),
                    vec![],
                )),
            );
            assert_eq!(
                execute_sync(QRY, None, &schema, &Variables::new(), &()),
                Ok((
                    graphql_value!({
                        "nullable": {"message": ["Ai", "Ambarendya!"]},
                    }),
                    vec![],
                )),
            );

            let schema = RootNode::new(Query, EmptyMutation::new(), Subscription);
            assert_eq!(
                resolve_into_stream(SUB, None, &schema, &Variables::new(), &())
                    .then(|s| extract_next(s))
                    .await,
                Ok((
                    graphql_value!({
                        "nullable": {"message": ["Ai", "Ambarendya!"]},
                    }),
                    vec![],
                )),
            );
        }

        #[tokio::test]
        async fn resolves_fallible() {
            const QRY: &str = "{ fallible }";
            const SUB: &str = "subscription { fallible }";

            let schema = RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());
            assert_eq!(
                execute(QRY, None, &schema, &Variables::new(), &()).await,
                Ok((
                    graphql_value!({
                        "fallible": {"message": ["Ai", "Ambarendya!"]},
                    }),
                    vec![],
                )),
            );
            assert_eq!(
                execute_sync(QRY, None, &schema, &Variables::new(), &()),
                Ok((
                    graphql_value!({
                        "fallible": {"message": ["Ai", "Ambarendya!"]},
                    }),
                    vec![],
                )),
            );

            let schema = RootNode::new(Query, EmptyMutation::new(), Subscription);
            assert_eq!(
                resolve_into_stream(SUB, None, &schema, &Variables::new(), &())
                    .then(|s| extract_next(s))
                    .await,
                Ok((
                    graphql_value!({
                        "fallible": {"message": ["Ai", "Ambarendya!"]},
                    }),
                    vec![],
                )),
            );
        }

        #[tokio::test]
        async fn resolves_fields() {
            const QRY: &str = r#"{
                object { message }
                nullable { message }
                fallible { message }
            }"#;
            const SUB1: &str = r#"subscription {
                object { message }
            }"#;
            const SUB2: &str = r#"subscription {
                nullable { message }
            }"#;
            const SUB3: &str = r#"subscription {
                fallible { message }
            }"#;

            let schema = RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());
            assert_eq!(
                execute(QRY, None, &schema, &Variables::new(), &()).await,
                Ok((
                    graphql_value!({
                        "object": {"message": ["Ai", "Ambarendya!"]},
                        "nullable": {"message": ["Ai", "Ambarendya!"]},
                        "fallible": {"message": ["Ai", "Ambarendya!"]},
                    }),
                    vec![],
                )),
            );
            assert_eq!(
                execute_sync(QRY, None, &schema, &Variables::new(), &()),
                Ok((
                    graphql_value!({
                        "object": {"message": ["Ai", "Ambarendya!"]},
                        "nullable": {"message": ["Ai", "Ambarendya!"]},
                        "fallible": {"message": ["Ai", "Ambarendya!"]},
                    }),
                    vec![],
                )),
            );

            let schema = RootNode::new(Query, EmptyMutation::new(), Subscription);
            assert_eq!(
                resolve_into_stream(SUB1, None, &schema, &Variables::new(), &())
                    .then(|s| extract_next(s))
                    .await,
                Ok((
                    graphql_value!({
                        "object": {"message": ["Ai", "Ambarendya!"]},
                    }),
                    vec![],
                )),
            );
            assert_eq!(
                resolve_into_stream(SUB2, None, &schema, &Variables::new(), &())
                    .then(|s| extract_next(s))
                    .await,
                Ok((
                    graphql_value!({
                        "nullable": {"message": ["Ai", "Ambarendya!"]},
                    }),
                    vec![],
                )),
            );
            assert_eq!(
                resolve_into_stream(SUB3, None, &schema, &Variables::new(), &())
                    .then(|s| extract_next(s))
                    .await,
                Ok((
                    graphql_value!({
                        "fallible": {"message": ["Ai", "Ambarendya!"]},
                    }),
                    vec![],
                )),
            );
        }

        #[tokio::test]
        async fn resolves_unknown_fields_as_null() {
            const QRY: &str = r#"{
                object { message, friend }
                nullable { message, mellon }
                fallible { message, freund }
            }"#;
            const SUB1: &str = r#"subscription {
                object { message, friend }
            }"#;
            const SUB2: &str = r#"subscription {
                nullable { message, mellon }
            }"#;
            const SUB3: &str = r#"subscription {
                fallible { message, freund }
            }"#;

            let schema = RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());
            assert_eq!(
                execute(QRY, None, &schema, &Variables::new(), &()).await,
                Ok((
                    graphql_value!({
                        "object": {
                            "message": ["Ai", "Ambarendya!"],
                            "friend": None,
                        },
                        "nullable": {
                            "message": ["Ai", "Ambarendya!"],
                            "mellon": None,
                        },
                        "fallible": {
                            "message": ["Ai", "Ambarendya!"],
                            "freund": None,
                        },
                    }),
                    vec![],
                )),
            );
            assert_eq!(
                execute_sync(QRY, None, &schema, &Variables::new(), &()),
                Ok((
                    graphql_value!({
                        "object": {
                            "message": ["Ai", "Ambarendya!"],
                            "friend": None,
                        },
                        "nullable": {
                            "message": ["Ai", "Ambarendya!"],
                            "mellon": None,
                        },
                        "fallible": {
                            "message": ["Ai", "Ambarendya!"],
                            "freund": None,
                        },
                    }),
                    vec![],
                )),
            );

            let schema = RootNode::new(Query, EmptyMutation::new(), Subscription);
            assert_eq!(
                resolve_into_stream(SUB1, None, &schema, &Variables::new(), &())
                    .then(|s| extract_next(s))
                    .await,
                Ok((
                    graphql_value!({
                        "object": {
                            "message": ["Ai", "Ambarendya!"],
                            "friend": None,
                        },
                    }),
                    vec![],
                )),
            );
            assert_eq!(
                resolve_into_stream(SUB2, None, &schema, &Variables::new(), &())
                    .then(|s| extract_next(s))
                    .await,
                Ok((
                    graphql_value!({
                        "nullable": {
                            "message": ["Ai", "Ambarendya!"],
                            "mellon": None,
                        },
                    }),
                    vec![],
                )),
            );
            assert_eq!(
                resolve_into_stream(SUB3, None, &schema, &Variables::new(), &())
                    .then(|s| extract_next(s))
                    .await,
                Ok((
                    graphql_value!({
                        "fallible": {
                            "message": ["Ai", "Ambarendya!"],
                            "freund": None,
                        },
                    }),
                    vec![],
                )),
            );
        }

        #[tokio::test]
        async fn resolves_nested_object_fields() {
            const QRY: &str = "{ nested { message { body } } }";
            const SUB: &str = "subscription { nested { message { body } } }";

            let schema = RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());
            assert_eq!(
                execute(QRY, None, &schema, &Variables::new(), &()).await,
                Ok((
                    graphql_value!({
                        "nested": {"message": {"body": "Ambarendya!"}},
                    }),
                    vec![],
                )),
            );
            assert_eq!(
                execute_sync(QRY, None, &schema, &Variables::new(), &()),
                Ok((
                    graphql_value!({
                        "nested": {"message": {"body": "Ambarendya!"}},
                    }),
                    vec![],
                )),
            );

            let schema = RootNode::new(Query, EmptyMutation::new(), Subscription);
            assert_eq!(
                resolve_into_stream(SUB, None, &schema, &Variables::new(), &())
                    .then(|s| extract_next(s))
                    .await,
                Ok((
                    graphql_value!({
                        "nested": {"message": {"body": "Ambarendya!"}},
                    }),
                    vec![],
                )),
            );
        }

        #[tokio::test]
        async fn resolves_nested_unknown_object_fields() {
            const QRY: &str = "{ nested { message { body, foo } } }";
            const SUB: &str = "subscription { nested { message { body, foo } } }";

            let schema = RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());
            assert_eq!(
                execute(QRY, None, &schema, &Variables::new(), &()).await,
                Ok((
                    graphql_value!({
                        "nested": {"message": {
                            "body": "Ambarendya!",
                            "foo": None,
                        }},
                    }),
                    vec![],
                )),
            );
            assert_eq!(
                execute_sync(QRY, None, &schema, &Variables::new(), &()),
                Ok((
                    graphql_value!({
                        "nested": {"message": {
                            "body": "Ambarendya!",
                            "foo": None,
                        }},
                    }),
                    vec![],
                )),
            );

            let schema = RootNode::new(Query, EmptyMutation::new(), Subscription);
            assert_eq!(
                resolve_into_stream(SUB, None, &schema, &Variables::new(), &())
                    .then(|s| extract_next(s))
                    .await,
                Ok((
                    graphql_value!({
                        "nested": {"message": {
                            "body": "Ambarendya!",
                            "foo": None,
                        }},
                    }),
                    vec![],
                )),
            );
        }

        #[tokio::test]
        async fn resolves_nested_aliased_object_fields() {
            const QRY: &str = "{ nested { m: message { b: body } } }";
            const SUB: &str = "subscription { nested { m: message { b: body } } }";

            let schema = RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());
            assert_eq!(
                execute(QRY, None, &schema, &Variables::new(), &()).await,
                Ok((
                    graphql_value!({
                        "nested": {"m": {"b": "Ambarendya!"}},
                    }),
                    vec![],
                )),
            );
            assert_eq!(
                execute_sync(QRY, None, &schema, &Variables::new(), &()),
                Ok((
                    graphql_value!({
                        "nested": {"m": {"b": "Ambarendya!"}},
                    }),
                    vec![],
                )),
            );

            let schema = RootNode::new(Query, EmptyMutation::new(), Subscription);
            assert_eq!(
                resolve_into_stream(SUB, None, &schema, &Variables::new(), &())
                    .then(|s| extract_next(s))
                    .await,
                Ok((
                    graphql_value!({
                        "nested": {"m": {"b": "Ambarendya!"}},
                    }),
                    vec![],
                )),
            );
        }
    }

    mod as_input {
        use serde_json::Value;

        use crate::{
            execute, graphql_object, EmptyMutation, EmptySubscription, RootNode, Variables,
        };

        struct Query;

        #[graphql_object]
        impl Query {
            fn input(arg: Value) -> Value {
                arg
            }
        }

        #[tokio::test]
        async fn accepts_null() {
            const DOC: &str = r#"{
                null: input(arg: null)
            }"#;

            let schema = RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());
            assert_eq!(
                execute(DOC, None, &schema, &Variables::new(), &()).await,
                Ok((graphql_value!({ "null": None }), vec![])),
            );
        }

        #[tokio::test]
        async fn accepts_bool() {
            const DOC: &str = r#"{
                bool: input(arg: true)
            }"#;

            let schema = RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());
            assert_eq!(
                execute(DOC, None, &schema, &Variables::new(), &()).await,
                Ok((graphql_value!({"bool": true}), vec![])),
            );
        }

        #[tokio::test]
        async fn accepts_int() {
            const DOC: &str = r#"{
                int: input(arg: 42)
            }"#;

            let schema = RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());
            assert_eq!(
                execute(DOC, None, &schema, &Variables::new(), &()).await,
                Ok((graphql_value!({"int": 42}), vec![])),
            );
        }

        #[tokio::test]
        async fn accepts_float() {
            const DOC: &str = r#"{
                float: input(arg: 3.14)
            }"#;

            let schema = RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());
            assert_eq!(
                execute(DOC, None, &schema, &Variables::new(), &()).await,
                Ok((graphql_value!({"float": 3.14}), vec![])),
            );
        }

        #[tokio::test]
        async fn accepts_string() {
            const DOC: &str = r#"{
                string: input(arg: "Galadriel")
            }"#;

            let schema = RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());
            assert_eq!(
                execute(DOC, None, &schema, &Variables::new(), &()).await,
                Ok((graphql_value!({"string": "Galadriel"}), vec![])),
            );
        }

        #[tokio::test]
        async fn accepts_array() {
            const DOC: &str = r#"{
                array: input(arg: ["Ai", "Ambarendya!"])
            }"#;

            let schema = RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());
            assert_eq!(
                execute(DOC, None, &schema, &Variables::new(), &()).await,
                Ok((graphql_value!({"array": ["Ai", "Ambarendya!"]}), vec![])),
            );
        }

        #[tokio::test]
        async fn accepts_object() {
            const DOC: &str = r#"{
                object: input(arg: {message: ["Ai", "Ambarendya!"]})
            }"#;

            let schema = RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());
            assert_eq!(
                execute(DOC, None, &schema, &Variables::new(), &()).await,
                Ok((
                    graphql_value!({
                        "object": {"message": ["Ai", "Ambarendya!"]},
                    }),
                    vec![],
                )),
            );
        }
    }
}

#[cfg(test)]
mod json_test {
    mod as_output {
        use futures::FutureExt as _;
        use serde::{Deserialize, Serialize};
        use serde_json::Value;

        use crate::{
            execute, execute_sync, graphql_object, graphql_subscription, resolve_into_stream,
            tests::util::{extract_next, stream, Stream},
            EmptyMutation, EmptySubscription, FieldResult, RootNode, Variables,
        };

        use super::super::Json;

        #[derive(Debug, Deserialize, Serialize)]
        struct Message {
            message: Vec<String>,
        }

        #[derive(Debug, Deserialize, Serialize)]
        struct Envelope {
            envelope: Message,
        }

        struct Query;

        #[graphql_object]
        impl Query {
            fn null() -> Json {
                Value::Null.into()
            }

            fn bool() -> Json<bool> {
                Json(true)
            }

            fn int() -> Json<i32> {
                Json(42)
            }

            fn float() -> Json<f64> {
                Json(3.14)
            }

            fn string() -> Json<&'static str> {
                Json("Galadriel")
            }

            fn array() -> Json<Vec<&'static str>> {
                Json(vec!["Ai", "Ambarendya!"])
            }

            fn object() -> Json<Message> {
                Json(Message {
                    message: vec!["Ai".into(), "Ambarendya!".into()],
                })
            }

            fn nullable() -> Option<Json<Message>> {
                Some(Json(Message {
                    message: vec!["Ai".into(), "Ambarendya!".into()],
                }))
            }

            fn fallible() -> FieldResult<Json<Message>> {
                Ok(Json(Message {
                    message: vec!["Ai".into(), "Ambarendya!".into()],
                }))
            }

            fn nested() -> Json<Envelope> {
                Json(Envelope {
                    envelope: Message {
                        message: vec!["Ai".into(), "Ambarendya!".into()],
                    },
                })
            }
        }

        struct Subscription;

        #[graphql_subscription]
        impl Subscription {
            async fn null() -> Stream<Json> {
                stream(Value::Null.into())
            }

            async fn bool() -> Stream<Json<bool>> {
                stream(Json(true))
            }

            async fn int() -> Stream<Json<i32>> {
                stream(Json(42))
            }

            async fn float() -> Stream<Json<f64>> {
                stream(Json(3.14))
            }

            async fn string() -> Stream<Json<String>> {
                stream(Json("Galadriel".into()))
            }

            async fn array() -> Stream<Json<Vec<&'static str>>> {
                stream(Json(vec!["Ai", "Ambarendya!"]))
            }

            async fn object() -> Stream<Json<Message>> {
                stream(Json(Message {
                    message: vec!["Ai".into(), "Ambarendya!".into()],
                }))
            }

            async fn nullable() -> Stream<Option<Json<Message>>> {
                stream(Some(Json(Message {
                    message: vec!["Ai".into(), "Ambarendya!".into()],
                })))
            }

            async fn fallible() -> FieldResult<Stream<FieldResult<Json<Message>>>> {
                Ok(stream(Ok(Json(Message {
                    message: vec!["Ai".into(), "Ambarendya!".into()],
                }))))
            }

            async fn nested() -> Stream<Json<Envelope>> {
                stream(Json(Envelope {
                    envelope: Message {
                        message: vec!["Ai".into(), "Ambarendya!".into()],
                    },
                }))
            }
        }

        #[tokio::test]
        async fn resolves_null() {
            const QRY: &str = "{ null }";
            const SUB: &str = "subscription { null }";

            let schema = RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());
            assert_eq!(
                execute(QRY, None, &schema, &Variables::new(), &()).await,
                Ok((graphql_value!({ "null": None }), vec![])),
            );
            assert_eq!(
                execute_sync(QRY, None, &schema, &Variables::new(), &()),
                Ok((graphql_value!({ "null": None }), vec![])),
            );

            let schema = RootNode::new(Query, EmptyMutation::new(), Subscription);
            assert_eq!(
                resolve_into_stream(SUB, None, &schema, &Variables::new(), &())
                    .then(|s| extract_next(s))
                    .await,
                Ok((graphql_value!({ "null": None }), vec![])),
            );
        }

        #[tokio::test]
        async fn resolves_bool() {
            const QRY: &str = "{ bool }";
            const SUB: &str = "subscription { bool }";

            let schema = RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());
            assert_eq!(
                execute(QRY, None, &schema, &Variables::new(), &()).await,
                Ok((graphql_value!({"bool": true}), vec![])),
            );
            assert_eq!(
                execute_sync(QRY, None, &schema, &Variables::new(), &()),
                Ok((graphql_value!({"bool": true}), vec![])),
            );

            let schema = RootNode::new(Query, EmptyMutation::new(), Subscription);
            assert_eq!(
                resolve_into_stream(SUB, None, &schema, &Variables::new(), &())
                    .then(|s| extract_next(s))
                    .await,
                Ok((graphql_value!({"bool": true}), vec![])),
            );
        }

        #[tokio::test]
        async fn resolves_int() {
            const QRY: &str = "{ int }";
            const SUB: &str = "subscription { int }";

            let schema = RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());
            assert_eq!(
                execute(QRY, None, &schema, &Variables::new(), &()).await,
                Ok((graphql_value!({"int": 42}), vec![])),
            );
            assert_eq!(
                execute_sync(QRY, None, &schema, &Variables::new(), &()),
                Ok((graphql_value!({"int": 42}), vec![])),
            );

            let schema = RootNode::new(Query, EmptyMutation::new(), Subscription);
            assert_eq!(
                resolve_into_stream(SUB, None, &schema, &Variables::new(), &())
                    .then(|s| extract_next(s))
                    .await,
                Ok((graphql_value!({"int": 42}), vec![])),
            );
        }

        #[tokio::test]
        async fn resolves_float() {
            const QRY: &str = "{ float }";
            const SUB: &str = "subscription { float }";

            let schema = RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());
            assert_eq!(
                execute(QRY, None, &schema, &Variables::new(), &()).await,
                Ok((graphql_value!({"float": 3.14}), vec![])),
            );
            assert_eq!(
                execute_sync(QRY, None, &schema, &Variables::new(), &()),
                Ok((graphql_value!({"float": 3.14}), vec![])),
            );

            let schema = RootNode::new(Query, EmptyMutation::new(), Subscription);
            assert_eq!(
                resolve_into_stream(SUB, None, &schema, &Variables::new(), &())
                    .then(|s| extract_next(s))
                    .await,
                Ok((graphql_value!({"float": 3.14}), vec![])),
            );
        }

        #[tokio::test]
        async fn resolves_string() {
            const QRY: &str = "{ string }";
            const SUB: &str = "subscription { string }";

            let schema = RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());
            assert_eq!(
                execute(QRY, None, &schema, &Variables::new(), &()).await,
                Ok((graphql_value!({"string": "Galadriel"}), vec![])),
            );
            assert_eq!(
                execute_sync(QRY, None, &schema, &Variables::new(), &()),
                Ok((graphql_value!({"string": "Galadriel"}), vec![])),
            );

            let schema = RootNode::new(Query, EmptyMutation::new(), Subscription);
            assert_eq!(
                resolve_into_stream(SUB, None, &schema, &Variables::new(), &())
                    .then(|s| extract_next(s))
                    .await,
                Ok((graphql_value!({"string": "Galadriel"}), vec![])),
            );
        }

        #[tokio::test]
        async fn resolves_array() {
            const QRY: &str = "{ array }";
            const SUB: &str = "subscription { array }";

            let schema = RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());
            assert_eq!(
                execute(QRY, None, &schema, &Variables::new(), &()).await,
                Ok((graphql_value!({"array": ["Ai", "Ambarendya!"]}), vec![])),
            );
            assert_eq!(
                execute_sync(QRY, None, &schema, &Variables::new(), &()),
                Ok((graphql_value!({"array": ["Ai", "Ambarendya!"]}), vec![])),
            );

            let schema = RootNode::new(Query, EmptyMutation::new(), Subscription);
            assert_eq!(
                resolve_into_stream(SUB, None, &schema, &Variables::new(), &())
                    .then(|s| extract_next(s))
                    .await,
                Ok((graphql_value!({"array": ["Ai", "Ambarendya!"]}), vec![])),
            );
        }

        #[tokio::test]
        async fn resolves_object() {
            const QRY: &str = "{ object }";
            const SUB: &str = "subscription { object }";

            let schema = RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());
            assert_eq!(
                execute(QRY, None, &schema, &Variables::new(), &()).await,
                Ok((
                    graphql_value!({
                        "object": {"message": ["Ai", "Ambarendya!"]},
                    }),
                    vec![],
                )),
            );
            assert_eq!(
                execute_sync(QRY, None, &schema, &Variables::new(), &()),
                Ok((
                    graphql_value!({
                        "object": {"message": ["Ai", "Ambarendya!"]},
                    }),
                    vec![],
                )),
            );

            let schema = RootNode::new(Query, EmptyMutation::new(), Subscription);
            assert_eq!(
                resolve_into_stream(SUB, None, &schema, &Variables::new(), &())
                    .then(|s| extract_next(s))
                    .await,
                Ok((
                    graphql_value!({
                        "object": {"message": ["Ai", "Ambarendya!"]},
                    }),
                    vec![],
                )),
            );
        }

        #[tokio::test]
        async fn resolves_nullable() {
            const QRY: &str = "{ nullable }";
            const SUB: &str = "subscription { nullable }";

            let schema = RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());
            assert_eq!(
                execute(QRY, None, &schema, &Variables::new(), &()).await,
                Ok((
                    graphql_value!({
                        "nullable": {"message": ["Ai", "Ambarendya!"]},
                    }),
                    vec![],
                )),
            );
            assert_eq!(
                execute_sync(QRY, None, &schema, &Variables::new(), &()),
                Ok((
                    graphql_value!({
                        "nullable": {"message": ["Ai", "Ambarendya!"]},
                    }),
                    vec![],
                )),
            );

            let schema = RootNode::new(Query, EmptyMutation::new(), Subscription);
            assert_eq!(
                resolve_into_stream(SUB, None, &schema, &Variables::new(), &())
                    .then(|s| extract_next(s))
                    .await,
                Ok((
                    graphql_value!({
                        "nullable": {"message": ["Ai", "Ambarendya!"]},
                    }),
                    vec![],
                )),
            );
        }

        #[tokio::test]
        async fn resolves_fallible() {
            const QRY: &str = "{ fallible }";
            const SUB: &str = "subscription { fallible }";

            let schema = RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());
            assert_eq!(
                execute(QRY, None, &schema, &Variables::new(), &()).await,
                Ok((
                    graphql_value!({
                        "fallible": {"message": ["Ai", "Ambarendya!"]},
                    }),
                    vec![],
                )),
            );
            assert_eq!(
                execute_sync(QRY, None, &schema, &Variables::new(), &()),
                Ok((
                    graphql_value!({
                        "fallible": {"message": ["Ai", "Ambarendya!"]},
                    }),
                    vec![],
                )),
            );

            let schema = RootNode::new(Query, EmptyMutation::new(), Subscription);
            assert_eq!(
                resolve_into_stream(SUB, None, &schema, &Variables::new(), &())
                    .then(|s| extract_next(s))
                    .await,
                Ok((
                    graphql_value!({
                        "fallible": {"message": ["Ai", "Ambarendya!"]},
                    }),
                    vec![],
                )),
            );
        }

        #[tokio::test]
        async fn resolves_fields() {
            const QRY: &str = r#"{
                object { message }
                nullable { message }
                fallible { message }
            }"#;
            const SUB1: &str = r#"subscription {
                object { message }
            }"#;
            const SUB2: &str = r#"subscription {
                nullable { message }
            }"#;
            const SUB3: &str = r#"subscription {
                fallible { message }
            }"#;

            let schema = RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());
            assert_eq!(
                execute(QRY, None, &schema, &Variables::new(), &()).await,
                Ok((
                    graphql_value!({
                        "object": {"message": ["Ai", "Ambarendya!"]},
                        "nullable": {"message": ["Ai", "Ambarendya!"]},
                        "fallible": {"message": ["Ai", "Ambarendya!"]},
                    }),
                    vec![],
                )),
            );
            assert_eq!(
                execute_sync(QRY, None, &schema, &Variables::new(), &()),
                Ok((
                    graphql_value!({
                        "object": {"message": ["Ai", "Ambarendya!"]},
                        "nullable": {"message": ["Ai", "Ambarendya!"]},
                        "fallible": {"message": ["Ai", "Ambarendya!"]},
                    }),
                    vec![],
                )),
            );

            let schema = RootNode::new(Query, EmptyMutation::new(), Subscription);
            assert_eq!(
                resolve_into_stream(SUB1, None, &schema, &Variables::new(), &())
                    .then(|s| extract_next(s))
                    .await,
                Ok((
                    graphql_value!({
                        "object": {"message": ["Ai", "Ambarendya!"]},
                    }),
                    vec![],
                )),
            );
            assert_eq!(
                resolve_into_stream(SUB2, None, &schema, &Variables::new(), &())
                    .then(|s| extract_next(s))
                    .await,
                Ok((
                    graphql_value!({
                        "nullable": {"message": ["Ai", "Ambarendya!"]},
                    }),
                    vec![],
                )),
            );
            assert_eq!(
                resolve_into_stream(SUB3, None, &schema, &Variables::new(), &())
                    .then(|s| extract_next(s))
                    .await,
                Ok((
                    graphql_value!({
                        "fallible": {"message": ["Ai", "Ambarendya!"]},
                    }),
                    vec![],
                )),
            );
        }

        #[tokio::test]
        async fn resolves_unknown_fields_as_null() {
            const QRY: &str = r#"{
                object { message, friend }
                nullable { message, mellon }
                fallible { message, freund }
            }"#;
            const SUB1: &str = r#"subscription {
                object { message, friend }
            }"#;
            const SUB2: &str = r#"subscription {
                nullable { message, mellon }
            }"#;
            const SUB3: &str = r#"subscription {
                fallible { message, freund }
            }"#;

            let schema = RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());
            assert_eq!(
                execute(QRY, None, &schema, &Variables::new(), &()).await,
                Ok((
                    graphql_value!({
                        "object": {
                            "message": ["Ai", "Ambarendya!"],
                            "friend": None,
                        },
                        "nullable": {
                            "message": ["Ai", "Ambarendya!"],
                            "mellon": None,
                        },
                        "fallible": {
                            "message": ["Ai", "Ambarendya!"],
                            "freund": None,
                        },
                    }),
                    vec![],
                )),
            );
            assert_eq!(
                execute_sync(QRY, None, &schema, &Variables::new(), &()),
                Ok((
                    graphql_value!({
                        "object": {
                            "message": ["Ai", "Ambarendya!"],
                            "friend": None,
                        },
                        "nullable": {
                            "message": ["Ai", "Ambarendya!"],
                            "mellon": None,
                        },
                        "fallible": {
                            "message": ["Ai", "Ambarendya!"],
                            "freund": None,
                        },
                    }),
                    vec![],
                )),
            );

            let schema = RootNode::new(Query, EmptyMutation::new(), Subscription);
            assert_eq!(
                resolve_into_stream(SUB1, None, &schema, &Variables::new(), &())
                    .then(|s| extract_next(s))
                    .await,
                Ok((
                    graphql_value!({
                        "object": {
                            "message": ["Ai", "Ambarendya!"],
                            "friend": None,
                        },
                    }),
                    vec![],
                )),
            );
            assert_eq!(
                resolve_into_stream(SUB2, None, &schema, &Variables::new(), &())
                    .then(|s| extract_next(s))
                    .await,
                Ok((
                    graphql_value!({
                        "nullable": {
                            "message": ["Ai", "Ambarendya!"],
                            "mellon": None,
                        },
                    }),
                    vec![],
                )),
            );
            assert_eq!(
                resolve_into_stream(SUB3, None, &schema, &Variables::new(), &())
                    .then(|s| extract_next(s))
                    .await,
                Ok((
                    graphql_value!({
                        "fallible": {
                            "message": ["Ai", "Ambarendya!"],
                            "freund": None,
                        },
                    }),
                    vec![],
                )),
            );
        }

        #[tokio::test]
        async fn resolves_nested_object_fields() {
            const QRY: &str = "{ nested { envelope { message } } }";
            const SUB: &str = "subscription { nested { envelope { message } } }";

            let schema = RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());
            assert_eq!(
                execute(QRY, None, &schema, &Variables::new(), &()).await,
                Ok((
                    graphql_value!({
                        "nested": {"envelope": {"message": ["Ai", "Ambarendya!"]}},
                    }),
                    vec![],
                )),
            );
            assert_eq!(
                execute_sync(QRY, None, &schema, &Variables::new(), &()),
                Ok((
                    graphql_value!({
                        "nested": {"envelope": {"message": ["Ai", "Ambarendya!"]}},
                    }),
                    vec![],
                )),
            );

            let schema = RootNode::new(Query, EmptyMutation::new(), Subscription);
            assert_eq!(
                resolve_into_stream(SUB, None, &schema, &Variables::new(), &())
                    .then(|s| extract_next(s))
                    .await,
                Ok((
                    graphql_value!({
                        "nested": {"envelope": {"message": ["Ai", "Ambarendya!"]}},
                    }),
                    vec![],
                )),
            );
        }

        #[tokio::test]
        async fn resolves_nested_unknown_object_fields() {
            const QRY: &str = "{ nested { envelope { message, foo } } }";
            const SUB: &str = "subscription { nested { envelope { message, foo } } }";

            let schema = RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());
            assert_eq!(
                execute(QRY, None, &schema, &Variables::new(), &()).await,
                Ok((
                    graphql_value!({
                        "nested": {"envelope": {
                            "message": ["Ai", "Ambarendya!"],
                            "foo": None,
                        }},
                    }),
                    vec![],
                )),
            );
            assert_eq!(
                execute_sync(QRY, None, &schema, &Variables::new(), &()),
                Ok((
                    graphql_value!({
                        "nested": {"envelope": {
                            "message": ["Ai", "Ambarendya!"],
                            "foo": None,
                        }},
                    }),
                    vec![],
                )),
            );

            let schema = RootNode::new(Query, EmptyMutation::new(), Subscription);
            assert_eq!(
                resolve_into_stream(SUB, None, &schema, &Variables::new(), &())
                    .then(|s| extract_next(s))
                    .await,
                Ok((
                    graphql_value!({
                        "nested": {"envelope": {
                            "message": ["Ai", "Ambarendya!"],
                            "foo": None,
                        }},
                    }),
                    vec![],
                )),
            );
        }

        #[tokio::test]
        async fn resolves_nested_aliased_object_fields() {
            const QRY: &str = "{ nested { e: envelope { m: message } } }";
            const SUB: &str = "subscription { nested { e: envelope { m: message } } }";

            let schema = RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());
            assert_eq!(
                execute(QRY, None, &schema, &Variables::new(), &()).await,
                Ok((
                    graphql_value!({
                        "nested": {"e": {"m": ["Ai", "Ambarendya!"]}},
                    }),
                    vec![],
                )),
            );
            assert_eq!(
                execute_sync(QRY, None, &schema, &Variables::new(), &()),
                Ok((
                    graphql_value!({
                        "nested": {"e": {"m": ["Ai", "Ambarendya!"]}},
                    }),
                    vec![],
                )),
            );

            let schema = RootNode::new(Query, EmptyMutation::new(), Subscription);
            assert_eq!(
                resolve_into_stream(SUB, None, &schema, &Variables::new(), &())
                    .then(|s| extract_next(s))
                    .await,
                Ok((
                    graphql_value!({
                        "nested": {"e": {"m": ["Ai", "Ambarendya!"]}},
                    }),
                    vec![],
                )),
            );
        }
    }

    mod as_input {
        use serde::{Deserialize, Serialize};

        use crate::{
            execute, graphql_object, EmptyMutation, EmptySubscription, RootNode, Variables,
        };

        use super::super::Json;

        #[derive(Debug, Deserialize, Serialize)]
        struct Message {
            message: Vec<String>,
        }

        #[derive(Debug, Deserialize, Serialize)]
        struct Envelope {
            envelope: Message,
        }

        struct Query;

        #[graphql_object]
        impl Query {
            fn any(arg: Json) -> Json {
                arg
            }

            fn bool(arg: Json<bool>) -> Json<bool> {
                arg
            }

            fn int(arg: Json<i32>) -> Json<i32> {
                arg
            }

            fn float(arg: Json<f64>) -> Json<f64> {
                arg
            }

            fn string(arg: Json<String>) -> Json<String> {
                arg
            }

            fn array(arg: Json<Vec<String>>) -> Json<Vec<String>> {
                arg
            }

            fn object(arg: Json<Envelope>) -> Json<Envelope> {
                arg
            }
        }

        #[tokio::test]
        async fn accepts_null() {
            const DOC: &str = r#"{
                null: any(arg: null)
            }"#;

            let schema = RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());
            assert_eq!(
                execute(DOC, None, &schema, &Variables::new(), &()).await,
                Ok((graphql_value!({ "null": None }), vec![])),
            );
        }

        #[tokio::test]
        async fn accepts_bool() {
            const DOC: &str = r#"{
                bool(arg: true)
            }"#;

            let schema = RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());
            assert_eq!(
                execute(DOC, None, &schema, &Variables::new(), &()).await,
                Ok((graphql_value!({"bool": true}), vec![])),
            );
        }

        #[tokio::test]
        async fn accepts_int() {
            const DOC: &str = r#"{
                int(arg: 42)
            }"#;

            let schema = RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());
            assert_eq!(
                execute(DOC, None, &schema, &Variables::new(), &()).await,
                Ok((graphql_value!({"int": 42}), vec![])),
            );
        }

        #[tokio::test]
        async fn accepts_float() {
            const DOC: &str = r#"{
                float(arg: 3.14)
            }"#;

            let schema = RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());
            assert_eq!(
                execute(DOC, None, &schema, &Variables::new(), &()).await,
                Ok((graphql_value!({"float": 3.14}), vec![])),
            );
        }

        #[tokio::test]
        async fn accepts_string() {
            const DOC: &str = r#"{
                string(arg: "Galadriel")
            }"#;

            let schema = RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());
            assert_eq!(
                execute(DOC, None, &schema, &Variables::new(), &()).await,
                Ok((graphql_value!({"string": "Galadriel"}), vec![])),
            );
        }

        #[tokio::test]
        async fn accepts_array() {
            const DOC: &str = r#"{
                array(arg: ["Ai", "Ambarendya!"])
            }"#;

            let schema = RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());
            assert_eq!(
                execute(DOC, None, &schema, &Variables::new(), &()).await,
                Ok((graphql_value!({"array": ["Ai", "Ambarendya!"]}), vec![])),
            );
        }

        #[tokio::test]
        async fn accepts_object() {
            const DOC: &str = r#"{
                object(arg: {envelope: {message: ["Ai", "Ambarendya!"]}})
            }"#;

            let schema = RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());
            assert_eq!(
                execute(DOC, None, &schema, &Variables::new(), &()).await,
                Ok((
                    graphql_value!({
                        "object": {"envelope": {"message": ["Ai", "Ambarendya!"]}},
                    }),
                    vec![],
                )),
            );
        }

        // TODO: This should not panic!
        /*
        #[tokio::test]
        async fn errors_on_invalid_object() {
            const DOC: &str = r#"{
                object(arg: {envelope: ["Ai", "Ambarendya!"]})
            }"#;

            let schema = RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());
            assert_eq!(
                execute(DOC, None, &schema, &Variables::new(), &()).await,
                Ok((
                    graphql_value!({
                        "object": {"envelope": {"message": ["Ai", "Ambarendya!"]}},
                    }),
                    vec![],
                )),
            );
        }
        */
    }
}

//------------------------------------------------------------------------------

/*
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

impl<S: ScalarValue> IsOutputType<S> for Json {}

impl<S: ScalarValue> IsInputType<S> for Json {}

impl<S: ScalarValue> FromInputValue<S> for Json {
    fn from_input_value(v: &InputValue<S>) -> Option<Self> {
        match v {
            InputValue::Null => Some(Json::Null),
            InputValue::Scalar(x) => Some(if let Some(i) = x.as_int() {
                Json::Number(serde_json::Number::from(i))
            } else if let Some(f) = x.as_float() {
                Json::Number(serde_json::Number::from_f64(f).expect("f64 to convert"))
            } else if let Some(b) = x.as_boolean() {
                Json::Bool(b)
            } else if let Some(s) = x.as_str() {
                Json::String(s.to_string())
            } else {
                unreachable!("`ScalarValue` must represent at least one of the GraphQL spec types")
            }),
            InputValue::Enum(x) => Some(Json::String(x.clone())),
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
            InputValue::Variable(_) => None,
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
        Box::pin(
            async move { <Json as GraphQLValue<S>>::resolve(self, info, selection_set, executor) },
        )
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

/// Trait used to provide the type information for a [`serde_json::Value`].
pub trait TypedJsonInfo: Send + Sync {
    /// the GraphQL type name
    fn type_name() -> &'static str;

    /// schema returns the GrpahQL Schema Definition language that contains the type_name
    fn schema() -> &'static str;
}

/// Wrapper generic type for [`serde_json::Value`] that associates type
/// information.
#[derive(Debug, Clone, PartialEq)]
pub struct TypedJson<T: TypedJsonInfo> {
    /// the wrapped json value
    pub json: serde_json::Value,
    phantom: PhantomData<T>,
}

impl<T: TypedJsonInfo> TypedJson<T> {
    /// creates a new TypedJson from a serde_json::Value
    pub fn new(v: serde_json::Value) -> TypedJson<T> {
        TypedJson {
            json: v,
            phantom: PhantomData,
        }
    }
}

impl<T, S> IsOutputType<S> for TypedJson<T>
where
    S: ScalarValue,
    T: TypedJsonInfo,
{
}

impl<T, S> IsInputType<S> for TypedJson<T>
where
    S: ScalarValue,
    T: TypedJsonInfo,
{
}

impl<T, S> FromInputValue<S> for TypedJson<T>
where
    S: ScalarValue,
    T: TypedJsonInfo,
{
    fn from_input_value(v: &InputValue<S>) -> Option<Self> {
        <serde_json::Value as FromInputValue<S>>::from_input_value(v).map(|x| TypedJson::new(x))
    }
}

impl<T, S> GraphQLType<S> for TypedJson<T>
where
    S: ScalarValue,
    T: TypedJsonInfo,
{
    fn name(_info: &Self::TypeInfo) -> Option<&str> {
        Some(T::type_name())
    }
    fn meta<'r>(_info: &Self::TypeInfo, registry: &mut Registry<'r, S>) -> MetaType<'r, S>
    where
        S: 'r,
    {
        TypeInfo {
            name: T::type_name().to_string(),
            schema: Some(T::schema().to_string()),
        }
        .meta(registry)
    }
}

impl<T, S> GraphQLValue<S> for TypedJson<T>
where
    S: ScalarValue,
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
        executor.resolve(
            &TypeInfo {
                schema: None,
                name: T::type_name().to_string(),
            },
            &self.json,
        )
    }
}

impl<T, S> GraphQLValueAsync<S> for TypedJson<T>
where
    Self::TypeInfo: Sync,
    Self::Context: Sync,
    S: ScalarValue + Send + Sync,
    T: TypedJsonInfo,
{
    fn resolve_async<'a>(
        &'a self,
        _info: &'a Self::TypeInfo,
        selection_set: Option<&'a [Selection<S>]>,
        executor: &'a Executor<Self::Context, S>,
    ) -> BoxFuture<'a, ExecutionResult<S>> {
        Box::pin(async move {
            let info = TypeInfo {
                schema: None,
                name: T::type_name().to_string(),
            };
            <Json as GraphQLValue<S>>::resolve(&self.json, &info, selection_set, executor)
        })
    }

    fn resolve_field_async<'a>(
        &'a self,
        _info: &'a Self::TypeInfo,
        field_name: &'a str,
        arguments: &'a Arguments<S>,
        executor: &'a Executor<Self::Context, S>,
    ) -> BoxFuture<'a, ExecutionResult<S>> {
        Box::pin(async move {
            let info = TypeInfo {
                schema: None,
                name: T::type_name().to_string(),
            };
            <Json as GraphQLValue<S>>::resolve_field(
                &self.json, &info, field_name, arguments, executor,
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use juniper::{
        execute_sync, graphql_object, graphql_value,
        integrations::serde_json::{TypeInfo, TypedJson, TypedJsonInfo},
        EmptyMutation, EmptySubscription, FieldResult, RootNode, ToInputValue, Variables,
    };
    use serde_json::json;

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
        impl TypedJsonInfo for Foo {
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
        #[graphql_object]
        impl Query {
            fn foo() -> FieldResult<TypedJson<Foo>> {
                let data = json!({"message": ["Hello", "World"] });
                Ok(TypedJson::new(data))
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
        )
        .unwrap();

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
        impl TypedJsonInfo for Foo {
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
                Ok(value == TypedJson::new(json!({"message":["Hello", "World"]})))
            }
        }
        let schema = juniper::RootNode::new(Query, EmptyMutation::new(), EmptySubscription::new());

        let vars = vec![(
            "value".to_owned(),
            graphql_value!({
                "message":["Hello", "World"],
            })
            .to_input_value(),
        )]
        .into_iter()
        .collect();

        // Run the executor.
        let (res, _errors) = juniper::execute_sync(
            "query example($value:Foo!){ foo(value: $value) }",
            None,
            &schema,
            &vars,
            &(),
        )
        .unwrap();

        // Ensure the value matches.
        assert_eq!(
            res,
            graphql_value!({
                "foo": true,
            }),
        );
    }
}
*/
