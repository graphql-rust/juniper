#[macro_use]
extern crate bencher;
extern crate juniper;

use bencher::Bencher;

use juniper::{execute_sync, tests::model::Database, EmptyMutation, RootNode, Variables};

fn query_type_name(b: &mut Bencher) {
    let database = Database::new();
    let schema = RootNode::new(&database, EmptyMutation::<Database>::new());

    let doc = r#"
        query IntrospectionQueryTypeQuery {
          __schema {
            queryType {
              name
            }
          }
        }"#;

    b.iter(|| execute_sync(doc, None, &schema, &Variables::new(), &database));
}

fn introspection_query(b: &mut Bencher) {
    let database = Database::new();
    let schema = RootNode::new(&database, EmptyMutation::<Database>::new());

    let doc = r#"
  query IntrospectionQuery {
    __schema {
      queryType { name }
      mutationType { name }
      subscriptionType { name }
      types {
        ...FullType
      }
      directives {
        name
        description
        locations
        args {
          ...InputValue
        }
      }
    }
  }

  fragment FullType on __Type {
    kind
    name
    description
    fields(includeDeprecated: true) {
      name
      description
      args {
        ...InputValue
      }
      type {
        ...TypeRef
      }
      isDeprecated
      deprecationReason
    }
    inputFields {
      ...InputValue
    }
    interfaces {
      ...TypeRef
    }
    enumValues(includeDeprecated: true) {
      name
      description
      isDeprecated
      deprecationReason
    }
    possibleTypes {
      ...TypeRef
    }
  }

  fragment InputValue on __InputValue {
    name
    description
    type { ...TypeRef }
    defaultValue
  }

  fragment TypeRef on __Type {
    kind
    name
    ofType {
      kind
      name
      ofType {
        kind
        name
        ofType {
          kind
          name
          ofType {
            kind
            name
            ofType {
              kind
              name
              ofType {
                kind
                name
                ofType {
                  kind
                  name
                }
              }
            }
          }
        }
      }
    }
  }
"#;

    b.iter(|| execute_sync(doc, None, &schema, &Variables::new(), &database));
}

benchmark_group!(queries, query_type_name, introspection_query);
benchmark_main!(queries);
