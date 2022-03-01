#![no_main]
use apollo_smith::DocumentBuilder;
use libfuzzer_sys::{
    arbitrary::{self, Unstructured},
    fuzz_target,
};

use juniper::{DefaultScalarValue, EmptyMutation, EmptySubscription, GraphQLObject, RootNode};

#[derive(Default, GraphQLObject, arbitrary::Arbitrary)]
struct Query {
    x: i32,
    y: String,
    z: Vec<bool>,
}

fuzz_target!(|input: &[u8]| {
    let mut u = Unstructured::new(input);
    let mut u2 = Unstructured::new(input);
    if let Ok(gql_doc) = DocumentBuilder::new(&mut u) {
        let document = gql_doc.finish();
        let doc = String::from(document);

        let arbitrary_schema: arbitrary::Result<
            RootNode<Query, EmptyMutation<()>, EmptySubscription<()>, DefaultScalarValue>,
        > = u2.arbitrary();

        if let Ok(schema) = arbitrary_schema {
            let _ = juniper::parser::parse_document_source(doc.as_str(), &schema.schema);
        }
    }
});
