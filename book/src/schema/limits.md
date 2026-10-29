Document limits
===============

**[Juniper] rejects [GraphQL documents][0] over its limits on nesting depth, tokens, and fields.**

Clients choose the documents that a server parses, validates, and executes. A short document can still cause a lot of work. Deep nesting can overflow the stack of the server. Repeated [fragment spreads][1] can expand a short document into an exponential number of fields. [Juniper] rejects such documents during parsing. Validation and execution do not start for a rejected document. Version 0.17.2 added the limits.

| Limit         | Default   | Method                       | Error                              |
|---------------|-----------|------------------------------|------------------------------------|
| Nesting depth | 64 levels | [`RootNode::limit_depth()`]  | [`ParseError::DepthLimitExceeded`] |
| Tokens        | 15,000    | [`RootNode::limit_tokens()`] | [`ParseError::TokenLimitExceeded`] |
| Fields        | 15,000    | [`RootNode::limit_fields()`] | [`ParseError::FieldLimitExceeded`] |

- The depth counts nested selection sets, fragments, input values, and list types. A [fragment spread][1] nests the selections of its fragment one level deeper.
- The token limit bounds the time to parse and validate a document.
- The field count of an operation includes the fields of a fragment once for each spread of the fragment.




## Changing the limits

The `limit_*()` methods of a [`RootNode`] replace the default limits. A document over a limit fails with a [`GraphQLError::ParseError`]:
```rust
# extern crate juniper;
# use juniper::{
#     EmptyMutation, EmptySubscription, GraphQLError, RootNode, graphql_object, graphql_vars,
#     parser::ParseError,
# };
#
struct Query;

#[graphql_object]
impl Query {
    fn ping() -> bool {
        true
    }
}

type Schema = RootNode<Query, EmptyMutation, EmptySubscription>;

fn main() {
    let schema = Schema::new(Query, EmptyMutation::new(), EmptySubscription::new())
        .limit_depth(32)
        .limit_tokens(5_000)
        .limit_fields(2);

    // The operation selects 3 fields.
    let query = "{ a: ping b: ping c: ping }";

    match juniper::execute_sync(query, None, &schema, &graphql_vars! {}, &()) {
        Err(GraphQLError::ParseError(err)) => {
            assert_eq!(err.item, ParseError::FieldLimitExceeded(2));
            assert_eq!(err.item.to_string(), "Document exceeds the limit of 2 fields");
        }
        res => panic!("expected `ParseError`, returned: {res:#?}"),
    }
}
```




## Overlapping fields

The validation of [field selection merging][2] compares the fields with the same response name. The number of comparisons can grow quadratically with the size of a document. [Juniper] stops the comparisons after 100,000 and rejects the document with the `Document has too many overlapping fields to validate` validation error. The validation reports at most 100 conflicts of a document. The two bounds are fixed.




## Request bodies

Web servers also limit the size of request bodies. [`juniper_hyper`] and [`juniper_warp`] read at most 2 MiB of a request body by default. A larger body gets a `413 Payload Too Large` response. The API docs of the two crates show how to change the limit. The other integrations use the body limits of their web frameworks.




[`GraphQLError::ParseError`]: https://docs.rs/juniper/0.17.2/juniper/enum.GraphQLError.html#variant.ParseError
[`juniper_hyper`]: https://docs.rs/juniper_hyper
[`juniper_warp`]: https://docs.rs/juniper_warp
[`ParseError::DepthLimitExceeded`]: https://docs.rs/juniper/0.17.2/juniper/parser/enum.ParseError.html#variant.DepthLimitExceeded
[`ParseError::FieldLimitExceeded`]: https://docs.rs/juniper/0.17.2/juniper/parser/enum.ParseError.html#variant.FieldLimitExceeded
[`ParseError::TokenLimitExceeded`]: https://docs.rs/juniper/0.17.2/juniper/parser/enum.ParseError.html#variant.TokenLimitExceeded
[`RootNode`]: https://docs.rs/juniper/0.17.2/juniper/struct.RootNode.html
[`RootNode::limit_depth()`]: https://docs.rs/juniper/0.17.2/juniper/struct.RootNode.html#method.limit_depth
[`RootNode::limit_fields()`]: https://docs.rs/juniper/0.17.2/juniper/struct.RootNode.html#method.limit_fields
[`RootNode::limit_tokens()`]: https://docs.rs/juniper/0.17.2/juniper/struct.RootNode.html#method.limit_tokens
[Juniper]: https://docs.rs/juniper

[0]: https://spec.graphql.org/October2021#sec-Document
[1]: https://spec.graphql.org/October2021#sec-Language.Fragments
[2]: https://spec.graphql.org/October2021#sec-Field-Selection-Merging
