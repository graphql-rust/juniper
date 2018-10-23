mod interface {
    use schema::model::RootNode;
    use types::scalars::EmptyMutation;
    use value::Value;

    trait Pet {
        fn name(&self) -> &str;

        fn as_dog(&self) -> Option<&Dog> {
            None
        }
        fn as_cat(&self) -> Option<&Cat> {
            None
        }
    }

    graphql_interface!(<'a> &'a Pet: () as "Pet" |&self| {
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

    graphql_object!(Dog: () |&self| {
        field name() -> &str { &self.name }
        field woofs() -> bool { self.woofs }

        interfaces: [&Pet]
    });

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

    graphql_object!(Cat: () |&self| {
        field name() -> &str { &self.name }
        field meows() -> bool { self.meows }

        interfaces: [&Pet]
    });

    struct Schema {
        pets: Vec<Box<Pet>>,
    }

    graphql_object!(Schema: () |&self| {
        field pets() -> Vec<&Pet> {
            self.pets.iter().map(|p| p.as_ref()).collect()
        }
    });

    #[test]
    fn test() {
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

        let (result, errs) = ::execute(doc, None, &schema, &vars, &()).expect("Execution failed");

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
                            ].into_iter()
                            .collect(),
                        ),
                        Value::object(
                            vec![
                                ("name", Value::scalar("Garfield")),
                                ("meows", Value::scalar(false)),
                            ].into_iter()
                            .collect(),
                        ),
                    ]),
                )].into_iter()
                .collect()
            )
        );
    }
}

mod union {
    use schema::model::RootNode;
    use types::scalars::EmptyMutation;
    use value::Value;

    trait Pet {
        fn as_dog(&self) -> Option<&Dog> {
            None
        }
        fn as_cat(&self) -> Option<&Cat> {
            None
        }
    }

    graphql_union!(<'a> &'a Pet: () as "Pet" |&self| {
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
        fn as_dog(&self) -> Option<&Dog> {
            Some(self)
        }
    }

    graphql_object!(Dog: () |&self| {
        field name() -> &str { &self.name }
        field woofs() -> bool { self.woofs }
    });

    struct Cat {
        name: String,
        meows: bool,
    }

    impl Pet for Cat {
        fn as_cat(&self) -> Option<&Cat> {
            Some(self)
        }
    }

    graphql_object!(Cat: () |&self| {
        field name() -> &str { &self.name }
        field meows() -> bool { self.meows }
    });

    struct Schema {
        pets: Vec<Box<Pet>>,
    }

    graphql_object!(Schema: () |&self| {
        field pets() -> Vec<&Pet> {
            self.pets.iter().map(|p| p.as_ref()).collect()
        }
    });

    #[test]
    fn test() {
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

        let (result, errs) = ::execute(doc, None, &schema, &vars, &()).expect("Execution failed");

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
                            ].into_iter()
                            .collect(),
                        ),
                        Value::object(
                            vec![
                                ("__typename", Value::scalar("Cat")),
                                ("name", Value::scalar("Garfield")),
                                ("meows", Value::scalar(false)),
                            ].into_iter()
                            .collect(),
                        ),
                    ]),
                )].into_iter()
                .collect()
            )
        );
    }
}
