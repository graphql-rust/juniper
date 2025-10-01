use std::borrow::Cow;

use crate::{
    ast, graphql,
    parser::{ParseError, SourcePosition, Spanning, Token, document::parse_document_source},
    schema::model::SchemaType,
    types::scalars::{EmptyMutation, EmptySubscription},
    validation::test_harness::{MutationRoot, QueryRoot, SubscriptionRoot},
    value::{DefaultScalarValue, ScalarValue},
};

fn parse_document<S>(s: &str) -> ast::OwnedDocument<'_, S>
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
        vec![ast::Definition::Operation(Spanning::start_end(
            &SourcePosition::new(0, 0, 0),
            &SourcePosition::new(111, 5, 13),
            ast::Operation {
                operation_type: ast::OperationType::Query,
                name: None,
                description: None,
                variable_definitions: None,
                directives: None,
                selection_set: vec![ast::Selection::Field(Spanning::start_end(
                    &SourcePosition::new(18, 1, 16),
                    &SourcePosition::new(97, 4, 17),
                    ast::Field {
                        alias: None,
                        name: Spanning::start_end(
                            &SourcePosition::new(18, 1, 16),
                            &SourcePosition::new(22, 1, 20),
                            "node",
                        ),
                        arguments: Some(Spanning::start_end(
                            &SourcePosition::new(22, 1, 20),
                            &SourcePosition::new(29, 1, 27),
                            ast::Arguments {
                                items: vec![(
                                    Spanning::start_end(
                                        &SourcePosition::new(23, 1, 21),
                                        &SourcePosition::new(25, 1, 23),
                                        "id",
                                    ),
                                    Spanning::start_end(
                                        &SourcePosition::new(27, 1, 25),
                                        &SourcePosition::new(28, 1, 26),
                                        graphql::input_value!(4),
                                    ),
                                )],
                            },
                        )),
                        directives: None,
                        selection_set: Some(vec![
                            ast::Selection::Field(Spanning::start_end(
                                &SourcePosition::new(52, 2, 20),
                                &SourcePosition::new(54, 2, 22),
                                ast::Field {
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
                            ast::Selection::Field(Spanning::start_end(
                                &SourcePosition::new(75, 3, 20),
                                &SourcePosition::new(79, 3, 24),
                                ast::Field {
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
                  "ID you should provide and \u{90AB} symbol"
                  $id: String
                  """
                  Switch for experiment ....
                  Multiline
                  """
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
            ast::Definition::Operation(Spanning::start_end(
                &SourcePosition::new(71, 2, 16),
                &SourcePosition::new(567, 17, 17),
                ast::Operation {
                    operation_type: ast::OperationType::Query,
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
                        &SourcePosition::new(364, 10, 17),
                        ast::VariableDefinitions {
                            items: vec![
                                (
                                    Spanning::start_end(
                                        &SourcePosition::new(172, 4, 18),
                                        &SourcePosition::new(175, 4, 21),
                                        "id",
                                    ),
                                    ast::VariableDefinition {
                                        description: Some(Spanning::start_end(
                                            &SourcePosition::new(110, 3, 18),
                                            &SourcePosition::new(153, 3, 61),
                                            Cow::Owned(
                                                "ID you should provide and \u{90AB} symbol".into(),
                                            ),
                                        )),
                                        var_type: Spanning::start_end(
                                            &SourcePosition::new(177, 4, 23),
                                            &SourcePosition::new(183, 4, 29),
                                            ast::Type::nullable("String"),
                                        ),
                                        default_value: None,
                                        directives: None,
                                    },
                                ),
                                (
                                    Spanning::start_end(
                                        &SourcePosition::new(319, 9, 18),
                                        &SourcePosition::new(329, 9, 28),
                                        "enableBaz",
                                    ),
                                    ast::VariableDefinition {
                                        description: Some(Spanning::start_end(
                                            &SourcePosition::new(202, 5, 18),
                                            &SourcePosition::new(300, 8, 21),
                                            Cow::Borrowed("Switch for experiment ....\nMultiline"),
                                        )),
                                        var_type: Spanning::start_end(
                                            &SourcePosition::new(331, 9, 30),
                                            &SourcePosition::new(338, 9, 37),
                                            ast::Type::nullable("Boolean"),
                                        ),
                                        default_value: Some(Spanning::start_end(
                                            &SourcePosition::new(341, 9, 40),
                                            &SourcePosition::new(346, 9, 45),
                                            graphql::input_value!(false),
                                        )),
                                        directives: None,
                                    },
                                )
                            ],
                        }
                    )),
                    directives: None,
                    selection_set: vec![ast::Selection::Field(Spanning::start_end(
                        &SourcePosition::new(385, 11, 18),
                        &SourcePosition::new(549, 16, 19),
                        ast::Field {
                            alias: None,
                            name: Spanning::start_end(
                                &SourcePosition::new(385, 11, 18),
                                &SourcePosition::new(388, 11, 21),
                                "foo",
                            ),
                            arguments: Some(Spanning::start_end(
                                &SourcePosition::new(388, 11, 21),
                                &SourcePosition::new(397, 11, 30),
                                ast::Arguments {
                                    items: vec![(
                                        Spanning::start_end(
                                            &SourcePosition::new(389, 11, 22),
                                            &SourcePosition::new(391, 11, 24),
                                            "id",
                                        ),
                                        Spanning::start_end(
                                            &SourcePosition::new(393, 11, 26),
                                            &SourcePosition::new(396, 11, 29),
                                            graphql::input_value!(@id),
                                        ),
                                    )],
                                },
                            )),
                            directives: None,
                            selection_set: Some(vec![
                                ast::Selection::Field(Spanning::start_end(
                                    &SourcePosition::new(420, 12, 20),
                                    &SourcePosition::new(423, 12, 23),
                                    ast::Field {
                                        alias: None,
                                        name: Spanning::start_end(
                                            &SourcePosition::new(420, 12, 20),
                                            &SourcePosition::new(423, 12, 23),
                                            "bar",
                                        ),
                                        arguments: None,
                                        directives: None,
                                        selection_set: None,
                                    },
                                )),
                                ast::Selection::Field(Spanning::start_end(
                                    &SourcePosition::new(444, 13, 20),
                                    &SourcePosition::new(529, 15, 21),
                                    ast::Field {
                                        alias: None,
                                        name: Spanning::start_end(
                                            &SourcePosition::new(444, 13, 20),
                                            &SourcePosition::new(447, 13, 23),
                                            "baz",
                                        ),
                                        arguments: None,
                                        directives: Some(vec![Spanning::start_end(
                                            &SourcePosition::new(448, 13, 24),
                                            &SourcePosition::new(472, 13, 48),
                                            ast::Directive {
                                                name: Spanning::start_end(
                                                    &SourcePosition::new(449, 13, 25),
                                                    &SourcePosition::new(456, 13, 32),
                                                    "include",
                                                ),
                                                arguments: Some(Spanning::start_end(
                                                    &SourcePosition::new(456, 13, 32),
                                                    &SourcePosition::new(472, 13, 48),
                                                    ast::Arguments {
                                                        items: vec![(
                                                            Spanning::start_end(
                                                                &SourcePosition::new(457, 13, 33),
                                                                &SourcePosition::new(459, 13, 35),
                                                                "if",
                                                            ),
                                                            Spanning::start_end(
                                                                &SourcePosition::new(461, 13, 37),
                                                                &SourcePosition::new(471, 13, 47),
                                                                graphql::input_value!(@enableBaz),
                                                            ),
                                                        )],
                                                    },
                                                )),
                                            },
                                        )]),
                                        selection_set: Some(vec![ast::Selection::FragmentSpread(
                                            Spanning::start_end(
                                                &SourcePosition::new(497, 14, 22),
                                                &SourcePosition::new(507, 14, 32),
                                                ast::FragmentSpread {
                                                    name: Spanning::start_end(
                                                        &SourcePosition::new(500, 14, 25),
                                                        &SourcePosition::new(507, 14, 32),
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
            ast::Definition::Fragment(Spanning::start_end(
                &SourcePosition::new(695, 23, 16),
                &SourcePosition::new(767, 25, 17),
                ast::Fragment {
                    name: Spanning::start_end(
                        &SourcePosition::new(704, 23, 25),
                        &SourcePosition::new(711, 23, 32),
                        "BazInfo",
                    ),
                    description: Some(Spanning::start_end(
                        &SourcePosition::new(585, 19, 16),
                        &SourcePosition::new(678, 22, 19),
                        Cow::Borrowed("Some block description here\nMultiline"),
                    )),
                    type_condition: Spanning::start_end(
                        &SourcePosition::new(715, 23, 36),
                        &SourcePosition::new(718, 23, 39),
                        "Baz",
                    ),
                    directives: None,
                    selection_set: vec![ast::Selection::Field(Spanning::start_end(
                        &SourcePosition::new(741, 24, 20),
                        &SourcePosition::new(749, 24, 28),
                        ast::Field {
                            alias: None,
                            name: Spanning::start_end(
                                &SourcePosition::new(741, 24, 20),
                                &SourcePosition::new(749, 24, 28),
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
