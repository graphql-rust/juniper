mod field_execution {
    use value::Value;
    use ast::InputValue;
    use schema::model::RootNode;
    use types::scalars::EmptyMutation;

    struct DataType;
    struct DeepDataType;

    graphql_object!(DataType: () |&self| {
        field a() -> &str { "Apple" }
        field b() -> &str { "Banana" }
        field c() -> &str { "Cookie" }
        field d() -> &str { "Donut" }
        field e() -> &str { "Egg" }
        field f() -> &str { "Fish" }

        field pic(size: Option<i64>) -> String {
            format!("Pic of size: {}", size.unwrap_or(50))
        }

        field deep() -> DeepDataType {
            DeepDataType
        }
    });

    graphql_object!(DeepDataType: () |&self| {
        field a() -> &str { "Already Been Done" }
        field b() -> &str { "Boring" }
        field c() -> Vec<Option<&str>> { vec![Some("Contrived"), None, Some("Confusing")] }

        field deeper() -> Vec<Option<DataType>> { vec![Some(DataType), None, Some(DataType) ] }
    });

    #[test]
    fn test() {
        let schema = RootNode::new(DataType, EmptyMutation::<()>::new());
        let doc = r"
          query Example($size: Int) {
            a,
            b,
            x: c
            ...c
            f
            ...on DataType {
              pic(size: $size)
            }
            deep {
              a
              b
              c
              deeper {
                a
                b
              }
            }
          }

          fragment c on DataType {
            d
            e
          }";

        let vars = vec![
            ("size".to_owned(), InputValue::int(100))
        ].into_iter().collect();

        let (result, errs) = ::execute(doc, None, &schema, &vars, &())
            .expect("Execution failed");

        assert_eq!(errs, []);

        println!("Result: {:?}", result);

        assert_eq!(
            result,
            Value::object(vec![
                ("a", Value::string("Apple")),
                ("b", Value::string("Banana")),
                ("x", Value::string("Cookie")),
                ("d", Value::string("Donut")),
                ("e", Value::string("Egg")),
                ("f", Value::string("Fish")),
                ("pic", Value::string("Pic of size: 100")),
                ("deep", Value::object(vec![
                    ("a", Value::string("Already Been Done")),
                    ("b", Value::string("Boring")),
                    ("c", Value::list(vec![
                        Value::string("Contrived"),
                        Value::null(),
                        Value::string("Confusing"),
                    ])),
                    ("deeper", Value::list(vec![
                        Value::object(vec![
                            ("a", Value::string("Apple")),
                            ("b", Value::string("Banana")),
                        ].into_iter().collect()),
                        Value::null(),
                        Value::object(vec![
                            ("a", Value::string("Apple")),
                            ("b", Value::string("Banana")),
                        ].into_iter().collect()),
                    ])),
                ].into_iter().collect())),
            ].into_iter().collect()));
    }
}


mod merge_parallel_fragments {
    use value::Value;
    use schema::model::RootNode;
    use types::scalars::EmptyMutation;

    struct Type;

    graphql_object!(Type: () |&self| {
        field a() -> &str { "Apple" }
        field b() -> &str { "Banana" }
        field c() -> &str { "Cherry" }
        field deep() -> Type { Type }
    });

    #[test]
    fn test() {
        let schema = RootNode::new(Type, EmptyMutation::<()>::new());
        let doc = r"
          { a, ...FragOne, ...FragTwo }
          fragment FragOne on Type {
            b
            deep { b, deeper: deep { b } }
          }
          fragment FragTwo on Type {
            c
            deep { c, deeper: deep { c } }
          }";

        let vars = vec![].into_iter().collect();

        let (result, errs) = ::execute(doc, None, &schema, &vars, &())
            .expect("Execution failed");

        assert_eq!(errs, []);

        println!("Result: {:?}", result);

        assert_eq!(
            result,
            Value::object(vec![
                ("a", Value::string("Apple")),
                ("b", Value::string("Banana")),
                ("c", Value::string("Cherry")),
                ("deep", Value::object(vec![
                    ("b", Value::string("Banana")),
                    ("c", Value::string("Cherry")),
                    ("deeper", Value::object(vec![
                        ("b", Value::string("Banana")),
                        ("c", Value::string("Cherry")),
                    ].into_iter().collect())),
                ].into_iter().collect())),
            ].into_iter().collect()));
    }
}

mod threads_context_correctly {
    use value::Value;
    use types::scalars::EmptyMutation;
    use schema::model::RootNode;

    struct Schema;

    graphql_object!(Schema: String |&self| {
        field a(&executor) -> String { executor.context().clone() }
    });

    #[test]
    fn test() {
        let schema = RootNode::new(Schema, EmptyMutation::<String>::new());
        let doc = r"{ a }";

        let vars = vec![].into_iter().collect();

        let (result, errs) = ::execute(doc, None, &schema, &vars, &"Context value".to_owned())
            .expect("Execution failed");

        assert_eq!(errs, []);

        println!("Result: {:?}", result);

        assert_eq!(
            result,
            Value::object(vec![
                ("a", Value::string("Context value")),
            ].into_iter().collect()));
    }
}

mod nulls_out_errors {
    use value::Value;
    use schema::model::RootNode;
    use executor::{ExecutionError, FieldResult};
    use parser::SourcePosition;
    use types::scalars::EmptyMutation;

    struct Schema;

    graphql_object!(Schema: () |&self| {
        field sync() -> FieldResult<&str> { Ok("sync") }
        field sync_error() -> FieldResult<&str> { Err("Error for syncError".to_owned()) }
    });

    #[test]
    fn test() {
        let schema = RootNode::new(Schema, EmptyMutation::<()>::new());
        let doc = r"{ sync, syncError }";

        let vars = vec![].into_iter().collect();

        let (result, errs) = ::execute(doc, None, &schema, &vars, &())
            .expect("Execution failed");

        println!("Result: {:?}", result);

        assert_eq!(
            result,
            Value::object(vec![
                ("sync", Value::string("sync")),
                ("syncError", Value::null()),
            ].into_iter().collect()));

        assert_eq!(
            errs,
            vec![
                ExecutionError::new(
                    SourcePosition::new(8, 0, 8),
                    &["syncError"],
                    "Error for syncError",
                ),
            ]);
    }
}

mod named_operations {
    use value::Value;
    use schema::model::RootNode;
    use types::scalars::EmptyMutation;
    use ::GraphQLError;

    struct Schema;

    graphql_object!(Schema: () |&self| {
        field a() -> &str { "b" }
    });

    #[test]
    fn uses_inline_operation_if_no_name_provided() {
        let schema = RootNode::new(Schema, EmptyMutation::<()>::new());
        let doc = r"{ a }";

        let vars = vec![].into_iter().collect();

        let (result, errs) = ::execute(doc, None, &schema, &vars, &())
            .expect("Execution failed");

        assert_eq!(errs, []);

        assert_eq!(
            result,
            Value::object(vec![
                ("a", Value::string("b")),
            ].into_iter().collect()));
    }

    #[test]
    fn uses_only_named_operation() {
        let schema = RootNode::new(Schema, EmptyMutation::<()>::new());
        let doc = r"query Example { a }";

        let vars = vec![].into_iter().collect();

        let (result, errs) = ::execute(doc, None, &schema, &vars, &())
            .expect("Execution failed");

        assert_eq!(errs, []);

        assert_eq!(
            result,
            Value::object(vec![
                ("a", Value::string("b")),
            ].into_iter().collect()));
    }

    #[test]
    fn uses_named_operation_if_name_provided() {
        let schema = RootNode::new(Schema, EmptyMutation::<()>::new());
        let doc = r"query Example { first: a } query OtherExample { second: a }";

        let vars = vec![].into_iter().collect();

        let (result, errs) = ::execute(doc, Some("OtherExample"), &schema, &vars, &())
            .expect("Execution failed");

        assert_eq!(errs, []);

        assert_eq!(
            result,
            Value::object(vec![
                ("second", Value::string("b")),
            ].into_iter().collect()));
    }

    #[test]
    fn error_if_multiple_operations_provided_but_no_name() {
        let schema = RootNode::new(Schema, EmptyMutation::<()>::new());
        let doc = r"query Example { first: a } query OtherExample { second: a }";

        let vars = vec![].into_iter().collect();

        let err = ::execute(doc, None, &schema, &vars, &())
            .unwrap_err();

        assert_eq!(err, GraphQLError::MultipleOperationsProvided);
    }

    #[test]
    fn error_if_unknown_operation_name_provided() {
        let schema = RootNode::new(Schema, EmptyMutation::<()>::new());
        let doc = r"query Example { first: a } query OtherExample { second: a }";

        let vars = vec![].into_iter().collect();

        let err = ::execute(doc, Some("UnknownExample"), &schema, &vars, &())
            .unwrap_err();

        assert_eq!(err, GraphQLError::UnknownOperationName);
    }
}
