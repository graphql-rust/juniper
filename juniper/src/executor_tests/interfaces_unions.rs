mod interface {
    use crate::{
        graphql_interface, graphql_object,
        schema::model::RootNode,
        types::scalars::{EmptyMutation, EmptySubscription},
        value::Value,
    };

    #[graphql_interface(for = [Cat, Dog])]
    trait Pet {
        fn name(&self) -> &str;

        #[graphql(downcast)]
        fn as_dog(&self) -> Option<&Dog> {
            None
        }
        #[graphql(downcast)]
        fn as_cat(&self) -> Option<&Cat> {
            None
        }
    }

    struct Dog {
        name: String,
        woofs: bool,
    }

    #[graphql_interface]
    impl Pet for Dog {
        fn name(&self) -> &str {
            &self.name
        }
        fn as_dog(&self) -> Option<&Dog> {
            Some(self)
        }
    }

    #[graphql_object(impl = PetValue)]
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

    #[graphql_interface]
    impl Pet for Cat {
        fn name(&self) -> &str {
            &self.name
        }
        fn as_cat(&self) -> Option<&Cat> {
            Some(self)
        }
    }

    #[graphql_object(impl = PetValue)]
    impl Cat {
        fn name(&self) -> &str {
            &self.name
        }
        fn meows(&self) -> bool {
            self.meows
        }
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
                        name: "Odie".to_owned(),
                        woofs: true,
                    }
                    .into(),
                    Cat {
                        name: "Garfield".to_owned(),
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

        println!("Result: {:#?}", result);

        assert_eq!(
            result,
            Value::object(
                vec![(
                    "pets",
                    Value::list(vec![
                        Value::object(
                            vec![
                                ("name", Value::scalar("Odie")),
                                ("woofs", Value::scalar(true)),
                            ]
                            .into_iter()
                            .collect(),
                        ),
                        Value::object(
                            vec![
                                ("name", Value::scalar("Garfield")),
                                ("meows", Value::scalar(false)),
                            ]
                            .into_iter()
                            .collect(),
                        ),
                    ]),
                )]
                .into_iter()
                .collect()
            )
        );
    }
}

mod union {
    use crate::{
        graphql_object, graphql_union,
        schema::model::RootNode,
        types::scalars::{EmptyMutation, EmptySubscription},
        value::Value,
    };

    #[graphql_union]
    trait Pet {
        fn as_dog(&self) -> Option<&Dog> {
            None
        }
        fn as_cat(&self) -> Option<&Cat> {
            None
        }
    }

    struct Dog {
        name: String,
        woofs: bool,
    }

    impl Pet for Dog {
        fn as_dog(&self) -> Option<&Dog> {
            Some(self)
        }
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

    impl Pet for Cat {
        fn as_cat(&self) -> Option<&Cat> {
            Some(self)
        }
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
        pets: Vec<Box<dyn Pet + Send + Sync>>,
    }

    #[graphql_object]
    impl Schema {
        fn pets(&self) -> Vec<&(dyn Pet + Send + Sync)> {
            self.pets.iter().map(|p| p.as_ref()).collect()
        }
    }

    #[tokio::test]
    async fn test_unions() {
        let schema = RootNode::new(
            Schema {
                pets: vec![
                    Box::new(Dog {
                        name: "Odie".to_owned(),
                        woofs: true,
                    }),
                    Box::new(Cat {
                        name: "Garfield".to_owned(),
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

        println!("Result: {:#?}", result);

        assert_eq!(
            result,
            Value::object(
                vec![(
                    "pets",
                    Value::list(vec![
                        Value::object(
                            vec![
                                ("__typename", Value::scalar("Dog")),
                                ("name", Value::scalar("Odie")),
                                ("woofs", Value::scalar(true)),
                            ]
                            .into_iter()
                            .collect(),
                        ),
                        Value::object(
                            vec![
                                ("__typename", Value::scalar("Cat")),
                                ("name", Value::scalar("Garfield")),
                                ("meows", Value::scalar(false)),
                            ]
                            .into_iter()
                            .collect(),
                        ),
                    ]),
                )]
                .into_iter()
                .collect()
            )
        );
    }
}
