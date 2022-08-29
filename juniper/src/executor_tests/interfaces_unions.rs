mod interface {
    use crate::{
        graphql_interface, graphql_object, graphql_value,
        schema::model::RootNode,
        types::scalars::{EmptyMutation, EmptySubscription},
        GraphQLObject,
    };

    #[graphql_interface(for = [Cat, Dog])]
    trait Pet {
        fn name(&self) -> &str;
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = PetValue)]
    struct Dog {
        name: String,
        woofs: bool,
    }

    #[derive(GraphQLObject)]
    #[graphql(impl = PetValue)]
    struct Cat {
        name: String,
        meows: bool,
    }

    struct Schema {
        pets: Vec<PetValue>,
    }

    #[graphql_object]
    impl Schema {
        fn pets(&self) -> &Vec<PetValue> {
            &self.pets
        }
    }

    #[tokio::test]
    async fn test() {
        let schema = RootNode::new(
            Schema {
                pets: vec![
                    Dog {
                        name: "Odie".into(),
                        woofs: true,
                    }
                    .into(),
                    Cat {
                        name: "Garfield".into(),
                        meows: false,
                    }
                    .into(),
                ],
            },
            EmptyMutation::<()>::new(),
            EmptySubscription::<()>::new(),
        );
        let doc = r"
          {
            pets {
              name
              ... on Dog {
                woofs
              }
              ... on Cat {
                meows
              }
            }
          }";

        let vars = vec![].into_iter().collect();

        let (result, errs) = crate::execute(doc, None, &schema, &vars, &())
            .await
            .expect("Execution failed");

        assert_eq!(errs, []);

        println!("Result: {result:#?}");

        assert_eq!(
            result,
            graphql_value!({
                "pets": [{
                    "name": "Odie",
                    "woofs": true,
                }, {
                    "name": "Garfield",
                    "meows": false,
                }],
            }),
        );
    }
}

mod union {
    use crate::{
        graphql_object, GraphQLUnion, graphql_value,
        schema::model::RootNode,
        types::scalars::{EmptyMutation, EmptySubscription},
    };

    #[derive(GraphQLUnion)]
    enum Pet {
        Dog(Dog),
        Cat(Cat),
    }

    struct Dog {
        name: String,
        woofs: bool,
    }

    #[graphql_object]
    impl Dog {
        fn name(&self) -> &str {
            &self.name
        }
        fn woofs(&self) -> bool {
            self.woofs
        }
    }

    struct Cat {
        name: String,
        meows: bool,
    }

    #[graphql_object]
    impl Cat {
        fn name(&self) -> &str {
            &self.name
        }
        fn meows(&self) -> bool {
            self.meows
        }
    }

    struct Schema {
        pets: Vec<Pet>,
    }

    #[graphql_object]
    impl Schema {
        fn pets(&self) -> &[Pet] {
            &self.pets
        }
    }

    #[tokio::test]
    async fn test_unions() {
        let schema = RootNode::new(
            Schema {
                pets: vec![
                    Pet::Dog(Dog {
                        name: "Odie".into(),
                        woofs: true,
                    }),
                    Pet::Cat(Cat {
                        name: "Garfield".into(),
                        meows: false,
                    }),
                ],
            },
            EmptyMutation::<()>::new(),
            EmptySubscription::<()>::new(),
        );
        let doc = r"
          {
            pets {
              __typename
              ... on Dog {
                name
                woofs
              }
              ... on Cat {
                name
                meows
              }
            }
          }";

        let vars = vec![].into_iter().collect();

        let (result, errs) = crate::execute(doc, None, &schema, &vars, &())
            .await
            .expect("Execution failed");

        assert_eq!(errs, []);

        println!("Result: {result:#?}");

        assert_eq!(
            result,
            graphql_value!({
                "pets": [{
                    "__typename": "Dog",
                    "name": "Odie",
                    "woofs": true,
                }, {
                    "__typename": "Cat",
                    "name": "Garfield",
                    "meows": false,
                }],
            }),
        );
    }
}
