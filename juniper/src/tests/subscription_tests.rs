use crate::{FieldResult, DefaultScalarValue, RootNode, Context, Value, EmptyMutation};
use juniper_codegen::GraphQLObjectInternal;
use juniper_codegen::{object_internal, subscription_internal};
use crate::http::GraphQLRequest;
use crate::value::Object;

use std::iter::FromIterator;

#[cfg(feature = "async")]
use futures::{
    self,
    stream::StreamExt
};

use std::iter;

#[derive(Debug, Clone)]
pub struct MyContext(i32);
impl Context for MyContext {}

type Schema = RootNode<'static, MyQuery, EmptyMutation::<MyContext>, MySubscription, DefaultScalarValue>;

#[cfg(feature = "async")]
type AsyncSchema = RootNode<'static, MyQuery, EmptyMutation::<MyContext>, MySubscriptionAsync, DefaultScalarValue>;

#[derive(GraphQLObjectInternal)]
#[graphql(description = "A humanoid creature in the Star Wars universe")]
#[derive(Clone)]
struct Human {
    id: String,
    name: String,
    home_planet: String,
}

struct MyQuery;

#[object_internal(
    context = MyContext
)]
impl MyQuery {
    fn human(id: String) -> FieldResult<Human> {
        let human = Human {
            id: "query".to_string(),
            name: "Query Human".to_string(),
            home_planet: "Query Human Planet".to_string(),
        };
        Ok(human)
    }
}

struct MySubscription;

#[subscription_internal(
    context = MyContext
)]
impl MySubscription {
    fn human(id: String) -> Human {
        let iter = Box::new(iter::once(Human {
            id: "subscription id".to_string(),
            name: "subscription name".to_string(),
            home_planet: "subscription planet".to_string(),
        }));
        Ok(iter)
    }
}

#[cfg(feature = "async")]
struct MySubscriptionAsync;

#[cfg(feature = "async")]
#[subscription_internal(
    context = MyContext
)]
impl MySubscriptionAsync {
    async fn async_human() -> Human {
        Ok(Box::pin(futures::stream::once(async {
            Human {
                id: "stream id".to_string(),
                name: "stream name".to_string(),
                home_planet: "stream home planet".to_string(),
            }
        })))
    }
}

#[test]
fn sync_subscription_returns_iterator() {
    let query =
    r#"subscription {
            human(id: "1") {
    		    id
                name
        	}
        }"#.to_string();

    let request = GraphQLRequest::new(
        query,
        None,
        None);

    let root_node =
        Schema::new(
            MyQuery,
            EmptyMutation::new(),
            MySubscription
        );

    let mut executor = crate::SubscriptionsExecutor::new();
    let mut context = MyContext(2);

    let response = request
        .subscribe(
            &root_node,
            &context,
            &mut executor
        )
        .into_inner();

    assert!(response.is_ok());

    let response = response.unwrap();

    // cannot compare with `assert_eq` because
    // iterator does not implement Debug
    let response_value_object = match response {
        Value::Object(o) => Some(o),
        _ => None,
    };

    assert!(response_value_object.is_some());

    let response_returned_object = response_value_object.unwrap();

    let fields_iterator = response_returned_object.into_key_value_list();

    let mut names = vec![];
    let mut collected_values = vec![];

    for (name, iter_val) in fields_iterator {
        names.push(name);

        // since macro returns Value::Scalar(iterator) every time,
        // other variants may be skipped
        match iter_val {
            Value::Scalar(iter) => {
                let collected = iter.collect::<Vec<_>>();
                collected_values.push(collected);
            },
            _ => unreachable!()
        }
    }

    let mut iterator_count = 0;
    let expected_values = vec![
        vec![
             Value::Object(
                 Object::from_iter(
                     iter::from_fn(move || {
                         iterator_count += 1;
                         match iterator_count {
                            1 => Some(("id", Value::Scalar(DefaultScalarValue::String("subscription id".to_string())))),
                            2 => Some(("name", Value::Scalar(DefaultScalarValue::String("subscription name".to_string())))),
                            _ => None,
                         }
                     })
                 )
             )
        ]
    ];

    assert_eq!(names, vec!["human"]);
    assert_eq!(collected_values, expected_values)
}

#[cfg(feature = "async")]
#[test]
fn async_subscription_returns_stream() {
    let query =
        r#"subscription {
            asyncHuman(id: "1") {
    		    id
                name
        	}
        }"#.to_string();

    let request = GraphQLRequest::new(
        query,
        None,
        None);

    let root_node =
        AsyncSchema::new(
            MyQuery,
            EmptyMutation::new(),
            MySubscriptionAsync
        );

    let mut executor = crate::SubscriptionsExecutor::new();
    let mut context = MyContext(2);

    let response = run(request
        .subscribe_async(
            &root_node,
            &context,
            &mut executor
        ))
        .into_inner();

    assert!(response.is_ok());

    let response = response.unwrap();

    // cannot compare with `assert_eq` because
    // iterator does not implement Debug
    let response_value_object = match response {
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
            },
            _ => unreachable!()
        }
    }

    let mut iterator_count = 0;
    let expected_values = vec![
        vec![
            Value::Object(
                Object::from_iter(
                    iter::from_fn(move || {
                        iterator_count += 1;
                        match iterator_count {
                            1 => Some((
                                "id",
                                Value::Scalar(DefaultScalarValue::String("stream id".to_string()))
                            )),
                            2 => Some((
                                "name",
                                Value::Scalar(DefaultScalarValue::String("stream name".to_string()))
                            )),
                            _ => None,
                        }
                    })
                )
            )
        ]
    ];

    assert_eq!(names, vec!["asyncHuman"]);
    assert_eq!(collected_values, expected_values);
}

//copied from `src/executor_tests/async_await/mod.rs`
fn run<O>(f: impl std::future::Future<Output = O>) -> O {
    tokio::runtime::current_thread::Runtime::new()
        .unwrap()
        .block_on(f)
}