use bencher::{Bencher, benchmark_group, benchmark_main};
use juniper::{
    DefaultScalarValue, EmptyMutation, EmptySubscription, RootNode, execute_sync, graphql_vars,
    tests::fixtures::starwars::schema::{Database, Query},
};

fn query_type_name(b: &mut Bencher) {
    let database = Database::new();
    let schema: RootNode<
        Query,
        EmptyMutation<Database>,
        EmptySubscription<Database>,
        DefaultScalarValue,
    > = RootNode::new(
        Query,
        EmptyMutation::<Database>::new(),
        EmptySubscription::<Database>::new(),
    );

    // language=GraphQL
    let query = "query IntrospectionQueryTypeQuery {
      __schema {
         queryType {
           name
         }
      }
    }";

    b.iter(|| execute_sync(query, None, &schema, &graphql_vars! {}, &database));
}

fn introspection_query(b: &mut Bencher) {
    let database = Database::new();
    let schema: RootNode<
        Query,
        EmptyMutation<Database>,
        EmptySubscription<Database>,
        DefaultScalarValue,
    > = RootNode::new(
        Query,
        EmptyMutation::<Database>::new(),
        EmptySubscription::<Database>::new(),
    );

    let query = include_str!("../src/introspection/query.graphql");

    b.iter(|| execute_sync(query, None, &schema, &graphql_vars! {}, &database));
}

benchmark_group!(queries, query_type_name, introspection_query);
benchmark_main!(queries);
