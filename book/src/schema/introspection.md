Introspection
=============

> The [schema introspection][1] system is accessible from the meta-fields `__schema` and `__type` which are accessible from the type of the root of a query operation.
> ```graphql
> __schema: __Schema!
> __type(name: String!): __Type
> ```
> Like all meta-fields, these are implicit and do not appear in the fields list in the root type of the query operation.

[GraphQL] provides [introspection][0], allowing to see what [queries][2], [mutations][3] and [subscriptions][4] a [GraphQL] server supports at runtime.

Because [introspection][0] queries are just regular [GraphQL queries][2], [Juniper] supports them natively. For example, to get all the names of the types supported, we could [execute][5] the following [query][2] against [Juniper]:
```graphql
{
  __schema {
    types {
      name
    }
  }
}
```




## Disabling

> Disabling introspection in production is a widely debated topic, but we believe it’s one of the first things you can do to harden your GraphQL API in production.

[Some security requirements and considerations][10] may mandate to disable [GraphQL schema introspection][1] in production environments. In [Juniper] this can be achieved by using the [`RootNode::disable_introspection()`][9] method:
```rust
# extern crate juniper;
# use juniper::{
#     EmptyMutation, EmptySubscription, GraphQLError, RootNode,
#     graphql_object, graphql_vars,
# };
#
pub struct Query;

#[graphql_object]
impl Query {
    fn some() -> bool {
        true
    }
}

type Schema = RootNode<Query, EmptyMutation, EmptySubscription>;

fn main() {
    let schema = Schema::new(Query, EmptyMutation::new(), EmptySubscription::new())
        .disable_introspection();

    let query = "query { __schema { queryType { name } } }";

    match juniper::execute_sync(query, None, &schema, &graphql_vars! {}, &()) {
        Err(GraphQLError::ValidationError(errs)) => {
            assert_eq!(
                errs.first().unwrap().message(),
                "GraphQL introspection is not allowed, but the operation contained `__schema`",
            );
        }
        res => panic!("expected `ValidationError`, returned: {res:#?}"),
    }
}
```
> **NOTE**: Attempt to execute an [introspection query][1] results in [validation][11] error, rather than [execution][5] error.




[GraphQL]: https://graphql.org
[Juniper]: https://docs.rs/juniper

[0]: https://spec.graphql.org/October2021#sec-Introspection
[1]: https://spec.graphql.org/October2021#sec-Schema-Introspection
[2]: https://spec.graphql.org/October2021#sel-GAFRJBABABF_jB
[3]: https://spec.graphql.org/October2021#sel-GAFRJDABABI5C
[4]: https://spec.graphql.org/October2021#sel-GAFRJFABABMvpN
[5]: https://spec.graphql.org/October2021#sec-Execution
[9]: https://docs.rs/juniper/0.17.1/juniper/struct.RootNode.html#method.disable_introspection
[10]: https://www.apollographql.com/blog/why-you-should-disable-graphql-introspection-in-production
[11]: https://spec.graphql.org/October2021#sec-Validation
