use indexmap::IndexMap;

use ast::InputValue;
use parser::value::parse_value_literal;
use parser::{Lexer, Parser, SourcePosition, Spanning};

fn parse_value(s: &str) -> Spanning<InputValue> {
    let mut lexer = Lexer::new(s);
    let mut parser = Parser::new(&mut lexer).expect(&format!("Lexer error on input {:#?}", s));

    parse_value_literal(&mut parser, false).expect(&format!("Parse error on input {:#?}", s))
}

#[test]
fn input_value_literals() {
    assert_eq!(
        parse_value("123"),
        Spanning::start_end(
            &SourcePosition::new(0, 0, 0),
            &SourcePosition::new(3, 0, 3),
            InputValue::int(123)
        )
    );
    assert_eq!(
        parse_value("123.45"),
        Spanning::start_end(
            &SourcePosition::new(0, 0, 0),
            &SourcePosition::new(6, 0, 6),
            InputValue::float(123.45)
        )
    );
    assert_eq!(
        parse_value("true"),
        Spanning::start_end(
            &SourcePosition::new(0, 0, 0),
            &SourcePosition::new(4, 0, 4),
            InputValue::boolean(true)
        )
    );
    assert_eq!(
        parse_value("false"),
        Spanning::start_end(
            &SourcePosition::new(0, 0, 0),
            &SourcePosition::new(5, 0, 5),
            InputValue::boolean(false)
        )
    );
    assert_eq!(
        parse_value(r#""test""#),
        Spanning::start_end(
            &SourcePosition::new(0, 0, 0),
            &SourcePosition::new(6, 0, 6),
            InputValue::string("test")
        )
    );
    assert_eq!(
        parse_value("enum_value"),
        Spanning::start_end(
            &SourcePosition::new(0, 0, 0),
            &SourcePosition::new(10, 0, 10),
            InputValue::enum_value("enum_value")
        )
    );
    assert_eq!(
        parse_value("$variable"),
        Spanning::start_end(
            &SourcePosition::new(0, 0, 0),
            &SourcePosition::new(9, 0, 9),
            InputValue::variable("variable")
        )
    );
    assert_eq!(
        parse_value("[]"),
        Spanning::start_end(
            &SourcePosition::new(0, 0, 0),
            &SourcePosition::new(2, 0, 2),
            InputValue::list(vec![])
        )
    );
    assert_eq!(
        parse_value("[1, [2, 3]]"),
        Spanning::start_end(
            &SourcePosition::new(0, 0, 0),
            &SourcePosition::new(11, 0, 11),
            InputValue::parsed_list(vec![
                Spanning::start_end(
                    &SourcePosition::new(1, 0, 1),
                    &SourcePosition::new(2, 0, 2),
                    InputValue::int(1),
                ),
                Spanning::start_end(
                    &SourcePosition::new(4, 0, 4),
                    &SourcePosition::new(10, 0, 10),
                    InputValue::parsed_list(vec![
                        Spanning::start_end(
                            &SourcePosition::new(5, 0, 5),
                            &SourcePosition::new(6, 0, 6),
                            InputValue::int(2),
                        ),
                        Spanning::start_end(
                            &SourcePosition::new(8, 0, 8),
                            &SourcePosition::new(9, 0, 9),
                            InputValue::int(3),
                        ),
                    ]),
                ),
            ])
        )
    );
    assert_eq!(
        parse_value("{}"),
        Spanning::start_end(
            &SourcePosition::new(0, 0, 0),
            &SourcePosition::new(2, 0, 2),
            InputValue::object(IndexMap::<String, InputValue>::new())
        )
    );
    assert_eq!(
        parse_value(r#"{key: 123, other: {foo: "bar"}}"#),
        Spanning::start_end(
            &SourcePosition::new(0, 0, 0),
            &SourcePosition::new(31, 0, 31),
            InputValue::parsed_object(vec![
                (
                    Spanning::start_end(
                        &SourcePosition::new(1, 0, 1),
                        &SourcePosition::new(4, 0, 4),
                        "key".to_owned(),
                    ),
                    Spanning::start_end(
                        &SourcePosition::new(6, 0, 6),
                        &SourcePosition::new(9, 0, 9),
                        InputValue::int(123),
                    ),
                ),
                (
                    Spanning::start_end(
                        &SourcePosition::new(11, 0, 11),
                        &SourcePosition::new(16, 0, 16),
                        "other".to_owned(),
                    ),
                    Spanning::start_end(
                        &SourcePosition::new(18, 0, 18),
                        &SourcePosition::new(30, 0, 30),
                        InputValue::parsed_object(vec![(
                            Spanning::start_end(
                                &SourcePosition::new(19, 0, 19),
                                &SourcePosition::new(22, 0, 22),
                                "foo".to_owned(),
                            ),
                            Spanning::start_end(
                                &SourcePosition::new(24, 0, 24),
                                &SourcePosition::new(29, 0, 29),
                                InputValue::string("bar"),
                            ),
                        )]),
                    ),
                ),
            ])
        )
    );
}
