use juniper_codegen::{object_internal, GraphQLObjectInternal};

use crate::{Context, FieldResult, FieldError, ExecutionError};

#[derive(Debug, Clone)]
pub struct MyContext(i32);
impl Context for MyContext {}

#[derive(GraphQLObjectInternal)]
#[graphql(description = "A humanoid creature in the Star Wars universe")]
#[derive(Clone)]
struct Human {
    id: String,
    name: String,
    home_planet: String,
}

struct MyQuery;

#[object_internal(context = MyContext)]
impl MyQuery {}

    use std::iter::{self, FromIterator};

    use futures::{self, stream::StreamExt as _};
    use juniper_codegen::subscription_internal;

    use crate::{http::GraphQLRequest, DefaultScalarValue, EmptyMutation, Object, RootNode, Value};

    use super::*;
use std::pin::Pin;

type AsyncSchema = RootNode<
    'static,
    MyQuery,
    EmptyMutation<MyContext>,
    MySubscription,
    DefaultScalarValue,
>;

// Copied from `src/executor_tests/async_await/mod.rs`.
fn run<O>(f: impl std::future::Future<Output = O>) -> O {
    tokio::runtime::current_thread::Runtime::new()
        .unwrap()
        .block_on(f)
}

type HumanStream = Pin<Box<dyn futures::Stream<Item = Human> + Send>>;

struct MySubscription;

#[subscription_internal(context = MyContext)]
impl MySubscription {
    async fn async_human() -> HumanStream {
        Box::pin(futures::stream::once(async {
            Human {
                id: "stream id".to_string(),
                name: "stream name".to_string(),
                home_planet: "stream home planet".to_string(),
            }
        }))
    }

    async fn error_human() -> Result<HumanStream, FieldError> {
        Err(FieldError::new(
            "handler error",
            Value::Scalar(DefaultScalarValue::String(
                "more details".to_string(),
            )),
        ))
    }

    async fn human_with_context(ctxt: &MyContext) -> HumanStream {
        let context_val = ctxt.0.clone();
        Box::pin(futures::stream::once(async move {
            Human {
                id: context_val.to_string(),
                name: context_val.to_string(),
                home_planet: context_val.to_string(),
            }
        }))
    }

    async fn human_with_args(id: String, name: String) -> HumanStream {
        Box::pin(futures::stream::once(async {
            Human {
                id: id,
                name: name,
                home_planet: "default home planet".to_string(),
            }
        }))
    }
}

/// Create all variables, execute subscription
/// and collect returned iterators.
/// Panics if query is invalid (GraphQLError is returned)
fn create_and_execute(query: String) -> Result<
    (Vec<String>, Vec<Vec<FieldResult<Value<DefaultScalarValue>>>>),
    Vec<ExecutionError<DefaultScalarValue>>
> {
    let request = GraphQLRequest::new(query, None, None);

    let root_node = AsyncSchema::new(MyQuery, EmptyMutation::new(), MySubscription);

    let mut context = MyContext(2);

    let response = run(request.subscribe(&root_node, &context)).into_inner();

    assert!(response.is_ok());

    let (values, errors) = response.unwrap();

    if errors.len() > 0 {
        return Err(errors);
    }

    // cannot compare with `assert_eq` because
    // stream does not implement Debug
    let response_value_object = match values {
        Value::Object(o) => Some(o),
        _ => None,
    };

    assert!(response_value_object.is_some());

    let response_returned_object = response_value_object.unwrap();

    let fields_iterator = response_returned_object.into_key_value_list();

    let mut names = vec![];
    let mut collected_values = vec![];

    for (name, stream_val) in fields_iterator {
        names.push(name);

        // since macro returns Value::Scalar(iterator) every time,
        // other variants may be skipped
        match stream_val {
            Value::Scalar(stream) => {
                let collected = run(stream.collect::<Vec<_>>());
                collected_values.push(collected);
            }
            _ => unreachable!(),
        }
    }

    Ok((names, collected_values))
}

#[test]
fn returns_requested_object() {
    let query = r#"subscription {
        asyncHuman(id: "1") {
            id
            name
        }
    }"#
    .to_string();

    let (names, collected_values) = create_and_execute(query).expect("Got error from stream");

    let mut iterator_count = 0;
    let expected_values = vec![vec![Ok(Value::Object(Object::from_iter(iter::from_fn(
        move || {
            iterator_count += 1;
            match iterator_count {
                1 => Some((
                    "id",
                    Value::Scalar(DefaultScalarValue::String("stream id".to_string())),
                )),
                2 => Some((
                    "name",
                    Value::Scalar(DefaultScalarValue::String("stream name".to_string())),
                )),
                _ => None,
            }
        },
    ))))]];

    assert_eq!(names, vec!["asyncHuman"]);
    assert_eq!(collected_values, expected_values);
}

#[test]
fn returns_error() {
    let query = r#"subscription {
        errorHuman(id: "1") {
            id
            name
        }
    }"#
        .to_string();

    let response = create_and_execute(query);

    assert!(response.is_err());

    let returned_errors = response.err().unwrap();

    let expected_error = ExecutionError::new(
        crate::parser::SourcePosition::new(23, 1, 8),
        &vec!["errorHuman"],
        FieldError::new("handler error", Value::Scalar(DefaultScalarValue::String(
            "more details".to_string(),
        ))),
    );
}

#[test]
fn can_access_context() {
    let query = r#"subscription {
            humanWithContext {
                id
              }
        }"#
        .to_string();

    let (names, collected_values) = create_and_execute(query).expect("Got error from stream");

    let mut iterator_count = 0;
    let expected_values = vec![vec![Ok(Value::Object(Object::from_iter(iter::from_fn(
        move || {
            iterator_count += 1;
            match iterator_count {
                1 => Some((
                    "id",
                    Value::Scalar(DefaultScalarValue::String("2".to_string())),
                )),
                _ => None,
            }
        },
    ))))]];

    assert_eq!(names, vec!["humanWithContext"]);
    assert_eq!(collected_values, expected_values);
}

#[test]
fn resolves_typed_inline_fragments() {
    let query = r#"subscription {
             ... on MySubscription {
                asyncHuman(id: "32") {
                  id
                }
             }
           }"#
        .to_string();

    let (names, collected_values) = create_and_execute(query).expect("Got error from stream");

    let mut iterator_count = 0;
    let expected_values = vec![vec![Ok(Value::Object(Object::from_iter(iter::from_fn(
        move || {
            iterator_count += 1;
            match iterator_count {
                1 => Some((
                    "id",
                    Value::Scalar(DefaultScalarValue::String("stream id".to_string())),
                )),
                _ => None,
            }
        },
    ))))]];

    assert_eq!(names, vec!["asyncHuman"]);
    assert_eq!(collected_values, expected_values);
}

#[test]
fn resolves_nontyped_inline_fragments() {
    let query = r#"subscription {
             ... {
                asyncHuman(id: "32") {
                  id
                }
             }
           }"#
        .to_string();

    let (names, collected_values) = create_and_execute(query).expect("Got error from stream");

    let mut iterator_count = 0;
    let expected_values = vec![vec![Ok(Value::Object(Object::from_iter(iter::from_fn(
        move || {
            iterator_count += 1;
            match iterator_count {
                1 => Some((
                    "id",
                    Value::Scalar(DefaultScalarValue::String("stream id".to_string())),
                )),
                _ => None,
            }
        },
    ))))]];

    assert_eq!(names, vec!["asyncHuman"]);
    assert_eq!(collected_values, expected_values);
}

#[test]
fn can_access_arguments() {
    let query = r#"subscription {
            humanWithArgs(id: "123", name: "args name") {
                id
                name
              }
        }"#
        .to_string();

    let (names, collected_values) = create_and_execute(query).expect("Got error from stream");

    let mut iterator_count = 0;
    let expected_values = vec![vec![Ok(Value::Object(Object::from_iter(iter::from_fn(
        move || {
            iterator_count += 1;
            match iterator_count {
                1 => Some((
                    "id",
                    Value::Scalar(DefaultScalarValue::String("123".to_string())),
                )),
                2 => Some((
                    "name",
                    Value::Scalar(DefaultScalarValue::String("args name".to_string())),
                )),
                _ => None,
            }
        },
    ))))]];

    assert_eq!(names, vec!["humanWithArgs"]);
    assert_eq!(collected_values, expected_values);
}