use crate::{
    ast::{Arguments, Definition, Field, Operation, OperationType, OwnedDocument, Selection},
    graphql_input_value,
    parser::{ParseError, SourcePosition, Spanning, Token, document::parse_document_source},
    schema::model::SchemaType,
    types::scalars::{EmptyMutation, EmptySubscription},
    validation::test_harness::{MutationRoot, QueryRoot, SubscriptionRoot},
    value::{DefaultScalarValue, ScalarValue},
};

fn parse_document<S>(s: &str) -> OwnedDocument<'_, S>
where
    S: ScalarValue,
{
    parse_document_source(
        s,
        &SchemaType::new::<QueryRoot, MutationRoot, SubscriptionRoot>(&(), &(), &()),
    )
    .unwrap_or_else(|_| panic!("Parse error on input {s:#?}"))
}

fn parse_document_error<S: ScalarValue>(s: &str) -> Spanning<ParseError> {
    match parse_document_source::<S>(
        s,
        &SchemaType::new::<QueryRoot, MutationRoot, SubscriptionRoot>(&(), &(), &()),
    ) {
        Ok(doc) => panic!("*No* parse error on input {s:#?} =>\n{doc:#?}"),
        Err(err) => err,
    }
}

#[test]
fn simple_ast() {
    assert_eq!(
        parse_document::<DefaultScalarValue>(
            r#"{
                node(id: 4) {
                    id
                    name
                }
            }"#,
        ),
        vec![Definition::Operation(Spanning::start_end(
            &SourcePosition::new(0, 0, 0),
            &SourcePosition::new(111, 5, 13),
            Operation {
                operation_type: OperationType::Query,
                name: None,
                variable_definitions: None,
                directives: None,
                selection_set: vec![Selection::Field(Spanning::start_end(
                    &SourcePosition::new(18, 1, 16),
                    &SourcePosition::new(97, 4, 17),
                    Field {
                        alias: None,
                        name: Spanning::start_end(
                            &SourcePosition::new(18, 1, 16),
                            &SourcePosition::new(22, 1, 20),
                            "node",
                        ),
                        arguments: Some(Spanning::start_end(
                            &SourcePosition::new(22, 1, 20),
                            &SourcePosition::new(29, 1, 27),
                            Arguments {
                                items: vec![(
                                    Spanning::start_end(
                                        &SourcePosition::new(23, 1, 21),
                                        &SourcePosition::new(25, 1, 23),
                                        "id",
                                    ),
                                    Spanning::start_end(
                                        &SourcePosition::new(27, 1, 25),
                                        &SourcePosition::new(28, 1, 26),
                                        graphql_input_value!(4),
                                    ),
                                )],
                            },
                        )),
                        directives: None,
                        selection_set: Some(vec![
                            Selection::Field(Spanning::start_end(
                                &SourcePosition::new(52, 2, 20),
                                &SourcePosition::new(54, 2, 22),
                                Field {
                                    alias: None,
                                    name: Spanning::start_end(
                                        &SourcePosition::new(52, 2, 20),
                                        &SourcePosition::new(54, 2, 22),
                                        "id",
                                    ),
                                    arguments: None,
                                    directives: None,
                                    selection_set: None,
                                },
                            )),
                            Selection::Field(Spanning::start_end(
                                &SourcePosition::new(75, 3, 20),
                                &SourcePosition::new(79, 3, 24),
                                Field {
                                    alias: None,
                                    name: Spanning::start_end(
                                        &SourcePosition::new(75, 3, 20),
                                        &SourcePosition::new(79, 3, 24),
                                        "name",
                                    ),
                                    arguments: None,
                                    directives: None,
                                    selection_set: None,
                                },
                            )),
                        ]),
                    },
                ))],
            },
        ))]
    )
}

#[test]
fn errors() {
    assert_eq!(
        parse_document_error::<DefaultScalarValue>("{"),
        Spanning::zero_width(
            &SourcePosition::new(1, 0, 1),
            ParseError::UnexpectedEndOfFile
        )
    );

    assert_eq!(
        parse_document_error::<DefaultScalarValue>("{ ...MissingOn }\nfragment MissingOn Type"),
        Spanning::start_end(
            &SourcePosition::new(36, 1, 19),
            &SourcePosition::new(40, 1, 23),
            ParseError::UnexpectedToken("Type".into())
        )
    );

    assert_eq!(
        parse_document_error::<DefaultScalarValue>("{ ...on }"),
        Spanning::start_end(
            &SourcePosition::new(8, 0, 8),
            &SourcePosition::new(9, 0, 9),
            ParseError::unexpected_token(Token::CurlyClose)
        )
    );
}

#[test]
fn issue_427_panic_is_not_expected() {
    struct QueryWithoutFloat;

    #[crate::graphql_object]
    impl QueryWithoutFloat {
        fn echo(value: String) -> String {
            value
        }
    }

    let schema = <SchemaType<DefaultScalarValue>>::new::<
        QueryWithoutFloat,
        EmptyMutation<()>,
        EmptySubscription<()>,
    >(&(), &(), &());
    let parse_result = parse_document_source(r##"{ echo(value: 123.0) }"##, &schema);

    assert_eq!(
        parse_result.unwrap_err().item,
        ParseError::ExpectedScalarError("There needs to be a Float type")
    );
}
