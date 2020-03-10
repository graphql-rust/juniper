mod interface {
    use crate::{schema::model::RootNode, types::scalars::EmptyMutation, value::Value};

    trait Pet {
        fn name(&self) -> &str;

        fn as_dog(&self) -> Option<&Dog> {
            None
        }
        fn as_cat(&self) -> Option<&Cat> {
            None
        }
    }

    graphql_interface!(<'a> &'a dyn Pet: () as "Pet" |&self| {
        field name() -> &str { self.name() }

        instance_resolvers: |&_| {
            &Dog => self.as_dog(),
            &Cat => self.as_cat(),
        }
    });

    struct Dog {
        name: String,
        woofs: bool,
    }

    impl Pet for Dog {
        fn name(&self) -> &str {
            &self.name
        }
        fn as_dog(&self) -> Option<&Dog> {
            Some(self)
        }
    }

    #[crate::graphql_object_internal(
        interfaces = [&dyn Pet]
    )]
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
        fn name(&self) -> &str {
            &self.name
        }
        fn as_cat(&self) -> Option<&Cat> {
            Some(self)
        }
    }

    #[crate::graphql_object_internal(
        interfaces = [&dyn Pet]
    )]
    impl Cat {
        fn name(&self) -> &str {
            &self.name
        }
        fn meows(&self) -> bool {
            self.meows
        }
    }

    struct Schema {
        pets: Vec<Box<dyn Pet>>,
    }

    #[crate::graphql_object_internal]
    impl Schema {
        fn pets(&self) -> Vec<&dyn Pet> {
            self.pets.iter().map(|p| p.as_ref()).collect()
        }
    }

    #[tokio::test]
    async fn test() {
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
    use crate::{schema::model::RootNode, types::scalars::EmptyMutation, value::Value};

    trait Pet {
        fn as_dog(&self) -> Option<&Dog> {
            None
        }
        fn as_cat(&self) -> Option<&Cat> {
            None
        }
    }

    #[crate::graphql_union_internal]
    impl<'a> GraphQLUnion for &'a dyn Pet {
        fn resolve(&self) {
            match self {
                Dog => self.as_dog(),
                Cat => self.as_cat(),
            }
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

    #[crate::graphql_object_internal]
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

    #[crate::graphql_object_internal]
    impl Cat {
        fn name(&self) -> &str {
            &self.name
        }
        fn meows(&self) -> bool {
            self.meows
        }
    }

    struct Schema {
        pets: Vec<Box<dyn Pet>>,
    }

    #[crate::graphql_object_internal]
    impl Schema {
        fn pets(&self) -> Vec<&dyn Pet> {
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
