#![no_main]
use libfuzzer_sys::{
    arbitrary::{self, Unstructured},
    fuzz_target,
};

use juniper::{
    DefaultScalarValue, EmptyMutation, EmptySubscription, GraphQLObject, IntrospectionFormat,
};

#[derive(Default, GraphQLObject, arbitrary::Arbitrary)]
struct Query {
    x: i32,
    y: String,
    z: Vec<bool>,
}

fuzz_target!(|input: &[u8]| {
    let mut u = Unstructured::new(input);
    let arbitrary_schema: arbitrary::Result<
        juniper::RootNode<Query, EmptyMutation<()>, EmptySubscription<()>, DefaultScalarValue>,
    > = u.arbitrary();

    if let Ok(schema) = arbitrary_schema {
        let _ = juniper::introspect(&schema, &(), IntrospectionFormat::default());
    }
});
