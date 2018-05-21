use ast::{Arguments, Definition, Document, Field, InputValue, Operation, OperationType, Selection};
use parser::document::parse_document_source;
use parser::{ParseError, SourcePosition, Spanning, Token};

fn parse_document(s: &str) -> Document {
    parse_document_source(s).expect(&format!("Parse error on input {:#?}", s))
}

fn parse_document_error<'a>(s: &'a str) -> Spanning<ParseError<'a>> {
    match parse_document_source(s) {
        Ok(doc) => panic!("*No* parse error on input {:#?} =>\n{:#?}", s, doc),
        Err(err) => err,
    }
}

#[test]
fn simple_ast() {
    assert_eq!(
        parse_document(
            r#"
            {
                node(id: 4) {
                    id
                    name
                }
            }
        "#
        ),
        vec![Definition::Operation(Spanning::start_end(
            &SourcePosition::new(13, 1, 12),
            &SourcePosition::new(124, 6, 13),
            Operation {
                operation_type: OperationType::Query,
                name: None,
                variable_definitions: None,
                directives: None,
                selection_set: vec![Selection::Field(Spanning::start_end(
                    &SourcePosition::new(31, 2, 16),
                    &SourcePosition::new(110, 5, 17),
                    Field {
                        alias: None,
                        name: Spanning::start_end(
                            &SourcePosition::new(31, 2, 16),
                            &SourcePosition::new(35, 2, 20),
                            "node",
                        ),
                        arguments: Some(Spanning::start_end(
                            &SourcePosition::new(35, 2, 20),
                            &SourcePosition::new(42, 2, 27),
                            Arguments {
                                items: vec![(
                                    Spanning::start_end(
                                        &SourcePosition::new(36, 2, 21),
                                        &SourcePosition::new(38, 2, 23),
                                        "id",
                                    ),
                                    Spanning::start_end(
                                        &SourcePosition::new(40, 2, 25),
                                        &SourcePosition::new(41, 2, 26),
                                        InputValue::int(4),
                                    ),
                                )],
                            },
                        )),
                        directives: None,
                        selection_set: Some(vec![
                            Selection::Field(Spanning::start_end(
                                &SourcePosition::new(65, 3, 20),
                                &SourcePosition::new(67, 3, 22),
                                Field {
                                    alias: None,
                                    name: Spanning::start_end(
                                        &SourcePosition::new(65, 3, 20),
                                        &SourcePosition::new(67, 3, 22),
                                        "id",
                                    ),
                                    arguments: None,
                                    directives: None,
                                    selection_set: None,
                                },
                            )),
                            Selection::Field(Spanning::start_end(
                                &SourcePosition::new(88, 4, 20),
                                &SourcePosition::new(92, 4, 24),
                                Field {
                                    alias: None,
                                    name: Spanning::start_end(
                                        &SourcePosition::new(88, 4, 20),
                                        &SourcePosition::new(92, 4, 24),
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
        parse_document_error("{"),
        Spanning::zero_width(
            &SourcePosition::new(1, 0, 1),
            ParseError::UnexpectedEndOfFile
        )
    );

    assert_eq!(
        parse_document_error("{ ...MissingOn }\nfragment MissingOn Type"),
        Spanning::start_end(
            &SourcePosition::new(36, 1, 19),
            &SourcePosition::new(40, 1, 23),
            ParseError::UnexpectedToken(Token::Name("Type"))
        )
    );

    assert_eq!(
        parse_document_error("{ ...on }"),
        Spanning::start_end(
            &SourcePosition::new(8, 0, 8),
            &SourcePosition::new(9, 0, 9),
            ParseError::UnexpectedToken(Token::CurlyClose)
        )
    );
}
