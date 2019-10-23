use crate::Context;
use juniper_codegen::GraphQLObjectInternal;
use juniper_codegen::object_internal;

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

#[object_internal(
    context = MyContext
)]
impl MyQuery {}

#[cfg(test)]
mod sync_tests {
    use super::*;

    use std::iter::{self, FromIterator};

    use crate::{DefaultScalarValue, RootNode, Value, EmptyMutation};
    use juniper_codegen::subscription_internal;
    use crate::http::GraphQLRequest;
    use crate::value::Object;

    type Schema = RootNode<'static, MyQuery, EmptyMutation::<MyContext>, MySubscription, DefaultScalarValue>;

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

        fn human_with_context(ctxt: &MyContext) -> Human {
            let iter = Box::new(iter::once(Human {
                id: ctxt.0.to_string(),
                name: ctxt.0.to_string(),
                home_planet: ctxt.0.to_string(),
            }));
            Ok(iter)
        }

        fn human_with_args(id: String, name: String) -> Human {
            let iter = Box::new(iter::once(Human {
                id: id,
                name: name,
                home_planet: "default home planet".to_string(),
            }));
            Ok(iter)
        }
    }

    /// Helper method to create all variables, execute subscription
    /// and collect returned iterators
    fn create_and_execute(query: String)
        -> (Vec<String>, Vec<Vec<Value<DefaultScalarValue>>>)
    {
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
        let context = MyContext(2);

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

        (names, collected_values)
    }

    #[test]
    fn subscription_returns_iterator() {
        let query =
            r#"subscription {
            human(id: "1") {
    		    id
                name
        	}
        }"#.to_string();

        let (names, collected_values) = create_and_execute(query);

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
        assert_eq!(collected_values, expected_values);
    }

    #[test]
    fn subscription_can_access_context() {
        let query =
            r#"subscription {
            humanWithContext {
                id
              }
        }"#.to_string();

        let (names, collected_values) = create_and_execute(query);

        let mut iterator_count = 0;
        let expected_values = vec![
            vec![
                Value::Object(
                    Object::from_iter(
                        iter::from_fn(move || {
                            iterator_count += 1;
                            match iterator_count {
                                1 => Some(("id", Value::Scalar(DefaultScalarValue::String("2".to_string())))),
                                _ => None,
                            }
                        })
                    )
                )
            ]
        ];

        assert_eq!(names, vec!["humanWithContext"]);
        assert_eq!(collected_values, expected_values);
    }

    //todo: uncomment once fragments on type `Self` can be executed by default
    //#[test]
    fn subscription_with_inline_fragments_typed() {
        let query =
        r#"subscription {
             ... on MySubscription {
                human(id: "32") {
                  id
                }
             }
           }"#.to_string();

        let (names, collected_values) = create_and_execute(query);

        let mut iterator_count = 0;
        let expected_values = vec![
            vec![
                Value::Object(
                    Object::from_iter(
                        iter::from_fn(move || {
                            iterator_count += 1;
                            match iterator_count {
                                1 => Some(("id", Value::Scalar(DefaultScalarValue::String("subscription id".to_string())))),
                                _ => None,
                            }
                        })
                    )
                )
            ]
        ];

        assert_eq!(names, vec!["human"]);
        assert_eq!(collected_values, expected_values);
    }

    //todo: uncomment once fragments on type `Self` can be executed by default
    //#[test]
    fn subscription_with_inline_fragments_nontyped() {
        let query =
        r#"subscription {
             ... {
                human(id: "32") {
                  id
                }
             }
           }"#.to_string();

        let (names, collected_values) = create_and_execute(query);

        let mut iterator_count = 0;
        let expected_values = vec![
            vec![
                Value::Object(
                    Object::from_iter(
                        iter::from_fn(move || {
                            iterator_count += 1;
                            match iterator_count {
                                1 => Some(("id", Value::Scalar(DefaultScalarValue::String("subscription id".to_string())))),
                                _ => None,
                            }
                        })
                    )
                )
            ]
        ];

        assert_eq!(names, vec!["human"]);
        assert_eq!(collected_values, expected_values);
    }

    #[test]
    fn subscription_can_access_arguments() {
        let query =
            r#"subscription {
            humanWithArgs(id: "123", name: "args name") {
                id
                name
              }
        }"#.to_string();

        let (names, collected_values) = create_and_execute(query);

        let mut iterator_count = 0;
        let expected_values = vec![
            vec![
                Value::Object(
                    Object::from_iter(
                        iter::from_fn(move || {
                            iterator_count += 1;
                            match iterator_count {
                                1 => Some(("id", Value::Scalar(DefaultScalarValue::String("123".to_string())))),
                                2 => Some(("name", Value::Scalar(DefaultScalarValue::String("args name".to_string())))),
                                _ => None,
                            }
                        })
                    )
                )
            ]
        ];

        assert_eq!(names, vec!["humanWithArgs"]);
        assert_eq!(collected_values, expected_values);
    }
}

#[cfg(feature = "async")]
#[cfg(test)]
mod async_tests {
    use futures::{
        self,
        stream::StreamExt
    };
    use crate::{RootNode, EmptyMutation, Value, Object, DefaultScalarValue};
    use crate::http::GraphQLRequest;
    use std::iter::{
        self, FromIterator,
    };
    use juniper_codegen::subscription_internal;

    use super::*;

    type AsyncSchema = RootNode<'static, MyQuery, EmptyMutation::<MyContext>, MySubscriptionAsync, DefaultScalarValue>;

    //copied from `src/executor_tests/async_await/mod.rs`
    fn run<O>(f: impl std::future::Future<Output = O>) -> O {
        tokio::runtime::current_thread::Runtime::new()
            .unwrap()
            .block_on(f)
    }

    struct MySubscriptionAsync;

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

    /// Helper method to create all variables, execute subscription
    /// and collect returned iterators
    fn create_and_execute(query: String)
                          -> (Vec<String>, Vec<Vec<Value<DefaultScalarValue>>>)
    {
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

        (names, collected_values)
    }

    #[test]
    fn subscription_returns_stream() {
        let query =
            r#"subscription {
            asyncHuman(id: "1") {
    		    id
                name
        	}
        }"#.to_string();

        let (names, collected_values) = create_and_execute(query);

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

}

