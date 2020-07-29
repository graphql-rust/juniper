use tracing_test::*;

use crate::{
    executor::Variables,
    schema::model::RootNode,
    tests::fixtures::starwars::{model::Database, schema::Query},
    types::scalars::{EmptyMutation, EmptySubscription},
};

// TODO: waiting for https://github.com/tokio-rs/tracing/pull/793
// TODO: async tests waiting for https://github.com/tokio-rs/tracing/pull/808
// TODO: tracing feature needs to be enable when testing
// cargo test --features tracing
#[test]
fn test_execute_sync_clean() {
    let doc = r#"
        {
            hero {
                name
            }
        }"#;
    let database = Database::new();
    let schema = RootNode::new(
        Query,
        EmptyMutation::<Database>::new(),
        EmptySubscription::<Database>::new(),
    );

    let span_rule_validation = span("rule_validation");
    let span_validate_input_values = span("validate_input_values");
    let span_execute_sync = span("execute_sync");

    let (subscriber, handle) = subscriber::expect()
        .new_span(span_rule_validation.clone())
        .enter(span_rule_validation.clone())
        .exit(span_rule_validation.clone())
        .drop_span(span_rule_validation.clone())
        .new_span(span_validate_input_values.clone())
        .enter(span_validate_input_values.clone())
        .exit(span_validate_input_values.clone())
        .drop_span(span_validate_input_values.clone())
        .new_span(span_execute_sync.clone())
        .enter(span_execute_sync.clone())
        .exit(span_execute_sync.clone())
        .drop_span(span_execute_sync.clone())
        .done()
        .run_with_handle();

    tracing::subscriber::with_default(subscriber, || {
        juniper::execute_sync(doc, None, &schema, &Variables::new(), &database).ok();
    });

    handle.assert_finished();
}

#[test]
fn test_execute_sync_with_error() {
    let doc = r#"
        {
            super_hero {
                name
            }
        }"#;
    let database = Database::new();
    let schema = RootNode::new(
        Query,
        EmptyMutation::<Database>::new(),
        EmptySubscription::<Database>::new(),
    );

    let span_rule_validation = span("rule_validation");

    let (subscriber, handle) = subscriber::expect()
        .new_span(span_rule_validation.clone())
        .enter(span_rule_validation.clone())
        .event(event().with_target("juniper"))
        .exit(span_rule_validation.clone())
        .drop_span(span_rule_validation.clone())
        .done()
        .run_with_handle();

    tracing::subscriber::with_default(subscriber, || {
        juniper::execute_sync(doc, None, &schema, &Variables::new(), &database).err();
    });

    handle.assert_finished();
}
