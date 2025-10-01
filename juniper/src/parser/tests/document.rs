use std::borrow::Cow;

use crate::{
    ast::{
        Arguments, Definition, Directive, Field, Fragment, FragmentSpread, Operation,
        OperationType, OwnedDocument, Selection, Type, VariableDefinition, VariableDefinitions,
    },
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
    .unwrap_or_else(|e| panic!("parse error on input {s:#?}:\n{e}"))
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
            // language=GraphQL
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
                description: None,
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
fn description() {
    assert_eq!(
        parse_document::<DefaultScalarValue>(
            // language=GraphQL
            r#"
                "Some description with \u90AB symbol"
                query SomeOperation(
                  #"ID you should provide"
                  $id: String
                  #"Switch for experiment ...."
                  $enableBaz: Boolean = false
                ) {
                  foo(id: $id) {
                    bar
                    baz @include(if: $enableBaz) {
                      ...BazInfo
                    }
                  }
                }

                """
                Some block description here
                Multiline
                """
                fragment BazInfo on Baz {
                    whatever
                }
            "#,
        ),
        vec![
            Definition::Operation(Spanning::start_end(
                &SourcePosition::new(71, 2, 16),
                &SourcePosition::new(479, 14, 17),
                Operation {
                    operation_type: OperationType::Query,
                    name: Some(Spanning::start_end(
                        &SourcePosition::new(77, 2, 22),
                        &SourcePosition::new(90, 2, 35),
                        "SomeOperation",
                    )),
                    description: Some(Spanning::start_end(
                        &SourcePosition::new(17, 1, 16),
                        &SourcePosition::new(54, 1, 53),
                        Cow::Owned("Some description with \u{90AB} symbol".into()),
                    )),
                    variable_definitions: Some(Spanning::start_end(
                        &SourcePosition::new(90, 2, 35),
                        &SourcePosition::new(276, 7, 17),
                        VariableDefinitions {
                            items: vec![
                                (
                                    Spanning::start_end(
                                        &SourcePosition::new(153, 4, 18),
                                        &SourcePosition::new(156, 4, 21),
                                        "id",
                                    ),
                                    VariableDefinition {
                                        var_type: Spanning::start_end(
                                            &SourcePosition::new(158, 4, 23),
                                            &SourcePosition::new(164, 4, 29),
                                            Type::nullable("String"),
                                        ),
                                        default_value: None,
                                        directives: None,
                                    },
                                ),
                                (
                                    Spanning::start_end(
                                        &SourcePosition::new(231, 6, 18),
                                        &SourcePosition::new(241, 6, 28),
                                        "enableBaz",
                                    ),
                                    VariableDefinition {
                                        var_type: Spanning::start_end(
                                            &SourcePosition::new(243, 6, 30),
                                            &SourcePosition::new(250, 6, 37),
                                            Type::nullable("Boolean"),
                                        ),
                                        default_value: Some(Spanning::start_end(
                                            &SourcePosition::new(253, 6, 40),
                                            &SourcePosition::new(258, 6, 45),
                                            graphql_input_value!(false),
                                        )),
                                        directives: None,
                                    },
                                )
                            ],
                        }
                    )),
                    directives: None,
                    selection_set: vec![Selection::Field(Spanning::start_end(
                        &SourcePosition::new(297, 8, 18),
                        &SourcePosition::new(461, 13, 19),
                        Field {
                            alias: None,
                            name: Spanning::start_end(
                                &SourcePosition::new(297, 8, 18),
                                &SourcePosition::new(300, 8, 21),
                                "foo",
                            ),
                            arguments: Some(Spanning::start_end(
                                &SourcePosition::new(300, 8, 21),
                                &SourcePosition::new(309, 8, 30),
                                Arguments {
                                    items: vec![(
                                        Spanning::start_end(
                                            &SourcePosition::new(301, 8, 22),
                                            &SourcePosition::new(303, 8, 24),
                                            "id",
                                        ),
                                        Spanning::start_end(
                                            &SourcePosition::new(305, 8, 26),
                                            &SourcePosition::new(308, 8, 29),
                                            graphql_input_value!(@id),
                                        ),
                                    )],
                                },
                            )),
                            directives: None,
                            selection_set: Some(vec![
                                Selection::Field(Spanning::start_end(
                                    &SourcePosition::new(332, 9, 20),
                                    &SourcePosition::new(335, 9, 23),
                                    Field {
                                        alias: None,
                                        name: Spanning::start_end(
                                            &SourcePosition::new(332, 9, 20),
                                            &SourcePosition::new(335, 9, 23),
                                            "bar",
                                        ),
                                        arguments: None,
                                        directives: None,
                                        selection_set: None,
                                    },
                                )),
                                Selection::Field(Spanning::start_end(
                                    &SourcePosition::new(356, 10, 20),
                                    &SourcePosition::new(441, 12, 21),
                                    Field {
                                        alias: None,
                                        name: Spanning::start_end(
                                            &SourcePosition::new(356, 10, 20),
                                            &SourcePosition::new(359, 10, 23),
                                            "baz",
                                        ),
                                        arguments: None,
                                        directives: Some(vec![Spanning::start_end(
                                            &SourcePosition::new(360, 10, 24),
                                            &SourcePosition::new(384, 10, 48),
                                            Directive {
                                                name: Spanning::start_end(
                                                    &SourcePosition::new(361, 10, 25),
                                                    &SourcePosition::new(368, 10, 32),
                                                    "include",
                                                ),
                                                arguments: Some(Spanning::start_end(
                                                    &SourcePosition::new(368, 10, 32),
                                                    &SourcePosition::new(384, 10, 48),
                                                    Arguments {
                                                        items: vec![(
                                                            Spanning::start_end(
                                                                &SourcePosition::new(369, 10, 33),
                                                                &SourcePosition::new(371, 10, 35),
                                                                "if",
                                                            ),
                                                            Spanning::start_end(
                                                                &SourcePosition::new(373, 10, 37),
                                                                &SourcePosition::new(383, 10, 47),
                                                                graphql_input_value!(@enableBaz),
                                                            ),
                                                        )],
                                                    },
                                                )),
                                            },
                                        )]),
                                        selection_set: Some(vec![Selection::FragmentSpread(
                                            Spanning::start_end(
                                                &SourcePosition::new(409, 11, 22),
                                                &SourcePosition::new(419, 11, 32),
                                                FragmentSpread {
                                                    name: Spanning::start_end(
                                                        &SourcePosition::new(412, 11, 25),
                                                        &SourcePosition::new(419, 11, 32),
                                                        "BazInfo",
                                                    ),
                                                    directives: None,
                                                },
                                            )
                                        )]),
                                    },
                                )),
                            ]),
                        },
                    ))],
                },
            )),
            Definition::Fragment(Spanning::start_end(
                &SourcePosition::new(607, 20, 16),
                &SourcePosition::new(679, 22, 17),
                Fragment {
                    name: Spanning::start_end(
                        &SourcePosition::new(616, 20, 25),
                        &SourcePosition::new(623, 20, 32),
                        "BazInfo",
                    ),
                    description: Some(Spanning::start_end(
                        &SourcePosition::new(497, 16, 16),
                        &SourcePosition::new(590, 19, 19),
                        Cow::Borrowed("Some block description here\nMultiline"),
                    )),
                    type_condition: Spanning::start_end(
                        &SourcePosition::new(627, 20, 36),
                        &SourcePosition::new(630, 20, 39),
                        "Baz",
                    ),
                    directives: None,
                    selection_set: vec![Selection::Field(Spanning::start_end(
                        &SourcePosition::new(653, 21, 20),
                        &SourcePosition::new(661, 21, 28),
                        Field {
                            alias: None,
                            name: Spanning::start_end(
                                &SourcePosition::new(653, 21, 20),
                                &SourcePosition::new(661, 21, 28),
                                "whatever",
                            ),
                            arguments: None,
                            directives: None,
                            selection_set: None,
                        },
                    ))]
                }
            ))
        ]
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

    // Descriptions are not permitted on query shorthand.
    // See: https://spec.graphql.org/September2025#sel-GAFTRJABAByBz7P
    assert_eq!(
        parse_document_error::<DefaultScalarValue>(r#""description" { foo }"#),
        Spanning::start_end(
            &SourcePosition::new(14, 0, 14),
            &SourcePosition::new(15, 0, 15),
            ParseError::unexpected_token(Token::CurlyOpen)
        ),
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
