use futures::StreamExt as _;
use tracing::{subscriber::DefaultGuard, Level};

use crate::{
    executor::Variables,
    schema::model::RootNode,
    tests::fixtures::tracing::{
        schema::{Database, Query, Subscriptions},
        SpanExt as _, TestSubscriber,
    },
    types::scalars::EmptyMutation,
};

type TestSchema<'a> = RootNode<'a, Query, EmptyMutation<Database>, Subscriptions>;

fn init_schema<'a>() -> TestSchema<'a> {
    TestSchema::new(Query, EmptyMutation::<Database>::new(), Subscriptions)
}

fn init_tracer() -> (TestSubscriber, DefaultGuard) {
    let subscriber = TestSubscriber::new();

    (
        subscriber.clone(),
        tracing::subscriber::set_default(subscriber),
    )
}

#[test]
fn test_execute_sync_clean() {
    let doc = r#"
        {
            foo {
                id
                nonTraced
                skipArgument(name: "name?", meaningOfLife: 42)
            }
        }"#;
    let schema = init_schema();
    let database = Database::new();
    let (handle, _guard) = init_tracer();

    juniper::execute_sync(doc, None, &schema, &Variables::new(), &database).ok();

    handle
        .assert()
        .enter_new_span("execute_sync")
        .simple_span("rule_validation")
        .simple_span("validate_input_values")
        .enter_new_span("execute_validated_query")
        .enter_new_span("Query.foo")
        .simple_span("Foo.id")
        .simple_span(
            &"Foo.skipArgument"
                .with_field("meaningOfLife", "42")
                .with_strict_fields(true),
        )
        .close_exited("Query.foo")
        .close_exited("execute_validated_query")
        .close_exited("execute_sync");
}

#[test]
fn test_execute_sync_with_error() {
    let doc = r#"
        {
            super_hero {
                name
            }
        }"#;
    let schema = init_schema();
    let database = Database::new();
    let (handle, _guard) = init_tracer();

    juniper::execute_sync(doc, None, &schema, &Variables::new(), &database).err();

    handle
        .assert()
        .enter_new_span("execute_sync")
        .enter_new_span("rule_validation")
        // Test that it writes event to traces when failed to validate rules
        .event(Level::TRACE, Some("juniper"), vec![])
        .close_exited("rule_validation")
        .close_exited("execute_sync");
}

#[tokio::test]
async fn records_sub_spans() {
    let doc = r#"
        {
            bar(id: 15) {
                tracedData
                nonTracedData
            }
        }
        "#;
    let schema = init_schema();
    let database = Database::new();
    let (handle, _guard) = init_tracer();

    let res = juniper::execute(doc, None, &schema, &Variables::new(), &database).await;
    assert!(res.is_ok(), "Should be ok");

    handle
        .assert()
        .enter_new_span("execute")
        .simple_span("rule_validation")
        .simple_span("validate_input_values")
        .enter_new_span("execute_validated_query")
        .enter_new_span("Query.bar")
        // To check whether context hasn't leaked.
        .enter_new_span(&"Bar.tracedData".with_strict_fields(true))
        // `traced_query` is fn marked with `#[instrument]` from `tracing` crate
        .simple_span(&"traced_query".with_field("id", "15"))
        .close_exited("Bar.tracedData")
        // Should has no child spans because `non_traced_query` doesn't marked with `#[instrument]`
        .simple_span(&"Bar.nonTracedData".with_strict_fields(true))
        .close_exited("Query.bar")
        .close_exited("execute_validated_query")
        .close_exited("execute");
}

#[tokio::test]
async fn test_no_trace_field() {
    let doc = r#"
        {
            foo {
                nonTraced
                asyncNonTraced
            }
            asyncFoo {
                nonTraced
            }
            fooBars(id: 98) {
                nonTraced
                asyncNonTraced
                ... on Bar {
                    nonTraced
                }
            }
        }"#;
    let schema = init_schema();
    let database = Database::new();
    let (handle, _guard) = init_tracer();

    let res = juniper::execute(doc, None, &schema, &Variables::new(), &database).await;
    assert!(res.is_ok(), "Should be ok");

    handle
        .assert()
        .enter_new_span("execute")
        .simple_span("rule_validation")
        .simple_span("validate_input_values")
        .enter_new_span("execute_validated_query")
        .enter_new_span("Query.foo")
        // In between this two steps should be no other, because `nonTraced` and `asyncNonTraced`
        // marked with `no_trace`
        .close_exited("Query.foo")
        .enter_new_span("Query.asyncFoo")
        // In between this two steps should be no other, because `nonTraced` marked with `no_trace`
        .close_exited("Query.asyncFoo")
        .enter_new_span("Query.fooBars")
        // Field with name present in interface but resolved on concrete object.
        .simple_span("Bar.nonTraced")
        .close_exited("Query.fooBars")
        .close_exited("execute_validated_query")
        .close_exited("execute");
}

#[tokio::test]
async fn test_impl_fn_args() {
    let doc = r#"
        {
            foo {
                skipArgument(name: "don't spy!", meaningOfLife: 42)
            }
            bar(id: 37) {
                defaultArg(another: -1)
            }
        }"#;
    let schema = init_schema();
    let database = Database::new();
    let (handle, _guard) = init_tracer();

    let res = juniper::execute(doc, None, &schema, &Variables::new(), &database).await;
    assert!(res.is_ok(), "Should be ok");

    handle
        .assert()
        .enter_new_span("execute")
        .simple_span("rule_validation")
        .simple_span("validate_input_values")
        .enter_new_span("execute_validated_query")
        .enter_new_span("Query.foo")
        // Skipped field
        .simple_span(
            &"Foo.skipArgument"
                .with_field("meaningOfLife", "42")
                .with_strict_fields(true),
        )
        .close_exited("Query.foo")
        // Required argument
        .new_span(&"Query.bar".with_field("id", "37").with_strict_fields(true))
        .enter("Query.bar")
        // Optional, overwritten optional and skipped optional arguments
        .simple_span(
            &"Bar.defaultArg"
                .with_field("this", "42")
                .with_field("another", "-1")
                .with_strict_fields(true),
        )
        .close_exited("Query.bar")
        .close_exited("execute_validated_query")
        .close_exited("execute");
}

#[tokio::test]
async fn test_custom_fields() {
    let doc = r#"
        {
            bar(id: 127) {
                id
            }
            asyncFoo {
                id
            }
        }
    "#;
    let schema = init_schema();
    let database = Database::new();
    let (handle, _guard) = init_tracer();

    let res = juniper::execute(doc, None, &schema, &Variables::new(), &database).await;
    assert!(res.is_ok(), "Should be ok");

    handle
        .assert()
        .enter_new_span("execute")
        .simple_span("rule_validation")
        .simple_span("validate_input_values")
        .enter_new_span("execute_validated_query")
        .enter_new_span(&"Query.bar".with_field("id", "127").with_strict_fields(true))
        // Check whether custom field "self.id" exists
        .simple_span(
            &"Bar.id"
                .with_field("self.id", "127")
                .with_strict_fields(true),
        )
        .close_exited("Query.bar")
        .enter_new_span("Query.asyncFoo")
        // Check multiple custom arguments and const arguments
        .simple_span(
            &"DerivedFoo.id"
                .with_field("self.id", "42")
                .with_field("custom_fields", "\"work\"")
                .with_strict_fields(true),
        )
        .close_exited("Query.asyncFoo")
        .close_exited("execute_validated_query")
        .close_exited("execute");
}

#[tokio::test]
async fn overwrite_level_and_target() {
    let doc = r#"
        {
            foo {
                target
                level
            }
            asyncFoo {
                target
                level
            }
            fooBar {
                target
                level
            }
        }
        "#;
    let schema = init_schema();
    let database = Database::new();
    let (handle, _guard) = init_tracer();

    let res = juniper::execute(doc, None, &schema, &Variables::new(), &database).await;
    assert!(res.is_ok(), "Should be ok");

    handle
        .assert()
        .enter_new_span("execute")
        .simple_span("rule_validation")
        .simple_span("validate_input_values")
        .enter_new_span("execute_validated_query")
        .enter_new_span("Query.foo")
        .simple_span(
            &"Foo.target"
                // Check on overwritten target in #[graphql_object]
                .with_target("my_target")
                // Check on default level in #[graphql_object]
                .with_level(Level::INFO),
        )
        .simple_span(
            &"Foo.level"
                // Check on default target in #[graphql_object]
                .with_target("juniper::tests::fixtures::tracing::schema")
                // Check on overwritten level in #[graphql_object]
                .with_level(Level::WARN),
        )
        .close_exited("Query.foo")
        .enter_new_span("Query.asyncFoo")
        .simple_span(
            &"DerivedFoo.target"
                // Check on overwritten target in derived GraphQLObject
                .with_target("my_target")
                // Check on default level in derived GraphQLObject
                .with_level(Level::INFO),
        )
        .simple_span(
            &"DerivedFoo.level"
                // Check on default target in derived GraphQLObject
                .with_target("juniper::tests::fixtures::tracing::schema")
                // Check on overwritten level in derived GraphQLObject
                .with_level(Level::WARN),
        )
        .close_exited("Query.asyncFoo")
        .enter_new_span("Query.fooBar")
        .simple_span(
            &"FooBar.target"
                // Check on overwritten target in #[graphql_interface]
                .with_target("my_target")
                // Check on default level in #[graphql_interface]
                .with_level(Level::INFO),
        )
        .simple_span(
            &"FooBar.level"
                // Check on default target in #[graphql_interface]
                .with_target("juniper::tests::fixtures::tracing::schema")
                // Check on overwritten level in #[graphql_interface]
                .with_level(Level::WARN),
        )
        .close_exited("Query.fooBar")
        .close_exited("execute_validated_query")
        .close_exited("execute");
}

#[tokio::test]
async fn graphql_object_trace_arg() {
    let doc = r#"
        {
            traceAsync {
                asyncFn
                syncFn
            }
            derivedAsync {
                sync
            }
            traceSync {
                asyncFn
                syncFn
            }
            derivedSync {
                sync
            }
            skipAll {
                asyncFn
                syncFn
            }
            skipAllDerived {
                sync
            }
            complexSync {
                syncFn
                asyncFn
                simpleField
                noTraceComplex
            }
            complexDerived {
                complex
                anotherComplex
                sync
            }
        }
        "#;
    let schema = init_schema();
    let database = Database::new();
    let (handle, _guard) = init_tracer();

    let res = juniper::execute(doc, None, &schema, &Variables::new(), &database).await;
    assert!(res.is_ok(), "Should be ok");

    handle
        .assert()
        .enter_new_span("execute")
        .simple_span("rule_validation")
        .simple_span("validate_input_values")
        .enter_new_span("execute_validated_query")
        .enter_new_span("Query.traceAsync")
        .simple_span("TraceAsync.asyncFn")
        // There shouldn't be span for `syncFn` because it's not async
        .close_exited("Query.traceAsync")
        // There should be nothing because derived resolvers sync by nature
        .simple_span("Query.derivedAsync")
        // There shouldn't be span for `asyncFn` because it's async
        .enter_new_span("Query.traceSync")
        .simple_span("TraceSync.syncFn")
        .close_exited("Query.traceSync")
        .enter_new_span("Query.derivedSync")
        .simple_span("SyncDerived.sync")
        .close_exited("Query.derivedSync")
        // There shouldn't be any spans because `SkipAll` and `SkipAllDerived` marked with "skip-all"
        .simple_span("Query.skipAll")
        .simple_span("Query.skipAllDerived")
        .enter_new_span("Query.complexSync")
        .simple_span("Complex.syncFn")
        .simple_span("Complex.asyncFn")
        // There shouldn't be any spans for `simpleField` and `noTraceComplex`
        .close_exited("Query.complexSync")
        .enter_new_span("Query.complexDerived")
        .simple_span("DerivedComplex.complex")
        .simple_span(
            &"DerivedComplex.anotherComplex"
                .with_field("test", "\"magic\"")
                .with_strict_fields(true),
        )
        // There shouldn't be span for `sync` because it's not marked with `#[tracing(complex)]`
        .close_exited("Query.complexDerived")
        .close_exited("execute_validated_query")
        .close_exited("execute");
}

#[tokio::test]
async fn graphql_interface_trace_arg() {
    let doc = r#"
        {
            erasedSimple {
                syncFn
                asyncFn
            }
            erasedSync {
                syncFn
                asyncFn
            }
            erasedAsync {
                syncFn
                asyncFn
            }
            erasedSkipAll {
                syncFn
                asyncFn
            }
            erasedComplex {
                syncFn
                asyncFn
            }
        }
        "#;
    let schema = init_schema();
    let database = Database::new();
    let (handle, _guard) = init_tracer();

    let res = juniper::execute(doc, None, &schema, &Variables::new(), &database).await;
    assert!(res.is_ok(), "Should be ok");

    handle
        .assert()
        .enter_new_span("execute")
        .simple_span("rule_validation")
        .simple_span("validate_input_values")
        .enter_new_span("execute_validated_query")
        .enter_new_span("Query.erasedSimple")
        .simple_span("InterfacedSimple.syncFn")
        .simple_span("InterfacedSimple.asyncFn")
        .close_exited("Query.erasedSimple")
        .enter_new_span("Query.erasedSync")
        .simple_span("InterfacedSync.syncFn")
        .close_exited("Query.erasedSync")
        .enter_new_span("Query.erasedAsync")
        .simple_span("InterfacedAsync.asyncFn")
        .close_exited("Query.erasedAsync")
        .simple_span("Query.erasedSkipAll")
        .enter_new_span("Query.erasedComplex")
        .simple_span("InterfacedComplex.asyncFn")
        .close_exited("Query.erasedComplex")
        .close_exited("execute_validated_query")
        .close_exited("execute");
}

#[tokio::test]
async fn subscription_tracing() {
    let doc = r#"
        subscription {
            barSub(id: 10) {
                id
                defaultArg(another: -1)
            }
        }
        "#;
    let schema = init_schema();
    let database = Database::new();
    let (handle, _guard) = init_tracer();

    let request = crate::http::GraphQLRequest::new(doc.to_owned(), None, None);

    let response = crate::http::resolve_into_stream(&request, &schema, &database).await;

    assert!(response.is_ok());

    let (values, errors) = response.unwrap();

    assert_eq!(errors.len(), 0, "Should return no errors");

    // cannot compare with `assert_eq` because
    // stream does not implement Debug
    let response_value_object = match values {
        juniper::Value::Object(o) => Some(o),
        _ => None,
    };

    assert!(response_value_object.is_some());

    let response_returned_object = response_value_object.unwrap();

    let fields = response_returned_object.into_iter();

    let mut names = vec![];
    let mut collected_values = vec![];

    for (name, stream_val) in fields {
        names.push(name.clone());

        // since macro returns Value::Scalar(iterator) every time,
        // other variants may be skipped
        match stream_val {
            juniper::Value::Scalar(stream) => {
                let collected = stream.collect::<Vec<_>>().await;
                collected_values.push(collected);
            }
            _ => unreachable!(),
        }
    }

    // Required argument
    handle
        .assert()
        .enter_new_span("resolve_into_stream")
        .simple_span("rule_validation")
        .simple_span("validate_input_values")
        .enter_new_span("resolve_validated_subscription")
        .new_span(
            &"Subscriptions.barSub"
                .with_field("id", "10")
                .with_strict_fields(true),
        )
        .close_exited("resolve_validated_subscription")
        .close_exited("resolve_into_stream")
        .enter("Subscriptions.barSub")
        .simple_span(&"Bar.id".with_field("self.id", "11"))
        .simple_span(
            &"Bar.defaultArg"
                .with_field("this", "42")
                .with_field("another", "-1")
                .with_strict_fields(true),
        )
        .re_enter("Subscriptions.barSub")
        .simple_span(&"Bar.id".with_field("self.id", "12"))
        .simple_span(
            &"Bar.defaultArg"
                .with_field("this", "42")
                .with_field("another", "-1")
                .with_strict_fields(true),
        )
        .close_exited("Subscriptions.barSub");
}
