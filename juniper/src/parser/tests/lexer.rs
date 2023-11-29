use crate::parser::{Lexer, LexerError, ScalarToken, SourcePosition, Spanning, Token};

fn tokenize_to_vec(s: &str) -> Vec<Spanning<Token<'_>>> {
    let mut tokens = Vec::new();
    let mut lexer = Lexer::new(s);

    loop {
        match lexer.next() {
            Some(Ok(t)) => {
                let at_eof = t.item == Token::EndOfFile;
                tokens.push(t);
                if at_eof {
                    break;
                }
            }
            Some(Err(e)) => panic!("Error in input stream: {e:#?} for {s:#?}"),
            None => panic!("EOF before EndOfFile token in {s:#?}"),
        }
    }

    tokens
}

fn tokenize_single(s: &str) -> Spanning<Token<'_>> {
    let mut tokens = tokenize_to_vec(s);

    assert_eq!(tokens.len(), 2);
    assert_eq!(tokens[1].item, Token::EndOfFile);

    tokens.remove(0)
}

fn tokenize_error(s: &str) -> Spanning<LexerError> {
    let mut lexer = Lexer::new(s);

    loop {
        match lexer.next() {
            Some(Ok(t)) => {
                if t.item == Token::EndOfFile {
                    panic!("Tokenizer did not return error for {s:#?}");
                }
            }
            Some(Err(e)) => {
                return e;
            }
            None => panic!("Tokenizer did not return error for {s:#?}"),
        }
    }
}

#[test]
fn empty_source() {
    assert_eq!(
        tokenize_to_vec(""),
        vec![Spanning::zero_width(
            &SourcePosition::new_origin(),
            Token::EndOfFile,
        )]
    );
}

#[test]
fn disallow_control_codes() {
    assert_eq!(
        Lexer::new("\u{0007}").next(),
        Some(Err(Spanning::zero_width(
            &SourcePosition::new_origin(),
            LexerError::UnknownCharacter('\u{0007}')
        )))
    );
}

#[test]
fn skip_whitespace() {
    assert_eq!(
        tokenize_to_vec(
            r#"

            foo

            "#
        ),
        vec![
            Spanning::start_end(
                &SourcePosition::new(14, 2, 12),
                &SourcePosition::new(17, 2, 15),
                Token::Name("foo"),
            ),
            Spanning::zero_width(&SourcePosition::new(31, 4, 12), Token::EndOfFile),
        ]
    );
}

#[test]
fn skip_comments() {
    assert_eq!(
        tokenize_to_vec(
            r#"
            #comment
            foo#comment
            "#
        ),
        vec![
            Spanning::start_end(
                &SourcePosition::new(34, 2, 12),
                &SourcePosition::new(37, 2, 15),
                Token::Name("foo"),
            ),
            Spanning::zero_width(&SourcePosition::new(58, 3, 12), Token::EndOfFile),
        ]
    );
}

#[test]
fn skip_commas() {
    assert_eq!(
        tokenize_to_vec(r#",,,foo,,,"#),
        vec![
            Spanning::start_end(
                &SourcePosition::new(3, 0, 3),
                &SourcePosition::new(6, 0, 6),
                Token::Name("foo"),
            ),
            Spanning::zero_width(&SourcePosition::new(9, 0, 9), Token::EndOfFile),
        ]
    );
}

#[test]
fn error_positions() {
    assert_eq!(
        Lexer::new(
            r#"

            ?

            "#
        )
        .next(),
        Some(Err(Spanning::zero_width(
            &SourcePosition::new(14, 2, 12),
            LexerError::UnknownCharacter('?')
        )))
    );
}

#[test]
fn strings() {
    assert_eq!(
        tokenize_single(r#""simple""#),
        Spanning::start_end(
            &SourcePosition::new(0, 0, 0),
            &SourcePosition::new(8, 0, 8),
            Token::Scalar(ScalarToken::String("simple"))
        )
    );

    assert_eq!(
        tokenize_single(r#"" white space ""#),
        Spanning::start_end(
            &SourcePosition::new(0, 0, 0),
            &SourcePosition::new(15, 0, 15),
            Token::Scalar(ScalarToken::String(" white space "))
        )
    );

    assert_eq!(
        tokenize_single(r#""quote \"""#),
        Spanning::start_end(
            &SourcePosition::new(0, 0, 0),
            &SourcePosition::new(10, 0, 10),
            Token::Scalar(ScalarToken::String(r#"quote \""#))
        )
    );

    assert_eq!(
        tokenize_single(r#""escaped \n\r\b\t\f""#),
        Spanning::start_end(
            &SourcePosition::new(0, 0, 0),
            &SourcePosition::new(20, 0, 20),
            Token::Scalar(ScalarToken::String(r"escaped \n\r\b\t\f"))
        )
    );

    assert_eq!(
        tokenize_single(r#""slashes \\ \/""#),
        Spanning::start_end(
            &SourcePosition::new(0, 0, 0),
            &SourcePosition::new(15, 0, 15),
            Token::Scalar(ScalarToken::String(r"slashes \\ \/"))
        )
    );

    assert_eq!(
        tokenize_single(r#""unicode \u1234\u5678\u90AB\uCDEF""#),
        Spanning::start_end(
            &SourcePosition::new(0, 0, 0),
            &SourcePosition::new(34, 0, 34),
            Token::Scalar(ScalarToken::String(r"unicode \u1234\u5678\u90AB\uCDEF")),
        )
    );
}

#[test]
fn string_errors() {
    assert_eq!(
        tokenize_error("\""),
        Spanning::zero_width(
            &SourcePosition::new(1, 0, 1),
            LexerError::UnterminatedString,
        )
    );

    assert_eq!(
        tokenize_error("\"no end quote"),
        Spanning::zero_width(
            &SourcePosition::new(13, 0, 13),
            LexerError::UnterminatedString,
        )
    );

    assert_eq!(
        tokenize_error("\"contains unescaped \u{0007} control char\""),
        Spanning::zero_width(
            &SourcePosition::new(20, 0, 20),
            LexerError::UnknownCharacterInString('\u{0007}'),
        )
    );

    assert_eq!(
        tokenize_error("\"null-byte is not \u{0000} end of file\""),
        Spanning::zero_width(
            &SourcePosition::new(18, 0, 18),
            LexerError::UnknownCharacterInString('\u{0000}'),
        )
    );

    assert_eq!(
        tokenize_error("\"multi\nline\""),
        Spanning::zero_width(
            &SourcePosition::new(6, 0, 6),
            LexerError::UnterminatedString,
        )
    );

    assert_eq!(
        tokenize_error("\"multi\rline\""),
        Spanning::zero_width(
            &SourcePosition::new(6, 0, 6),
            LexerError::UnterminatedString,
        )
    );

    assert_eq!(
        tokenize_error(r#""bad \z esc""#),
        Spanning::zero_width(
            &SourcePosition::new(6, 0, 6),
            LexerError::UnknownEscapeSequence("\\z".into()),
        )
    );

    assert_eq!(
        tokenize_error(r#""bad \x esc""#),
        Spanning::zero_width(
            &SourcePosition::new(6, 0, 6),
            LexerError::UnknownEscapeSequence("\\x".into()),
        )
    );

    assert_eq!(
        tokenize_error(r#""bad \u1 esc""#),
        Spanning::zero_width(
            &SourcePosition::new(6, 0, 6),
            LexerError::UnknownEscapeSequence("\\u1".into()),
        )
    );

    assert_eq!(
        tokenize_error(r#""bad \u0XX1 esc""#),
        Spanning::zero_width(
            &SourcePosition::new(6, 0, 6),
            LexerError::UnknownEscapeSequence("\\u0XX1".into()),
        )
    );

    assert_eq!(
        tokenize_error(r#""bad \uXXXX esc""#),
        Spanning::zero_width(
            &SourcePosition::new(6, 0, 6),
            LexerError::UnknownEscapeSequence("\\uXXXX".into()),
        )
    );

    assert_eq!(
        tokenize_error(r#""bad \uFXXX esc""#),
        Spanning::zero_width(
            &SourcePosition::new(6, 0, 6),
            LexerError::UnknownEscapeSequence("\\uFXXX".into()),
        )
    );

    assert_eq!(
        tokenize_error(r#""bad \uXXXF esc""#),
        Spanning::zero_width(
            &SourcePosition::new(6, 0, 6),
            LexerError::UnknownEscapeSequence("\\uXXXF".into()),
        )
    );

    assert_eq!(
        tokenize_error(r#""unterminated in string \""#),
        Spanning::zero_width(
            &SourcePosition::new(26, 0, 26),
            LexerError::UnterminatedString
        )
    );

    assert_eq!(
        tokenize_error(r#""unterminated \"#),
        Spanning::zero_width(
            &SourcePosition::new(15, 0, 15),
            LexerError::UnterminatedString
        )
    );

    // Found by fuzzing.
    assert_eq!(
        tokenize_error(r#""\uɠ^A"#),
        Spanning::zero_width(
            &SourcePosition::new(5, 0, 5),
            LexerError::UnterminatedString
        )
    );
}

#[test]
fn numbers() {
    fn assert_float_token_eq(
        source: &str,
        start: SourcePosition,
        end: SourcePosition,
        expected: &str,
    ) {
        let parsed = tokenize_single(source);
        assert_eq!(parsed.span.start, start);
        assert_eq!(parsed.span.end, end);

        match parsed.item {
            Token::Scalar(ScalarToken::Float(actual)) => {
                assert!(
                    expected == actual,
                    "[expected] {expected} != {actual} [actual]",
                );
            }
            _ => assert!(false),
        }
    }

    assert_eq!(
        tokenize_single("4"),
        Spanning::start_end(
            &SourcePosition::new(0, 0, 0),
            &SourcePosition::new(1, 0, 1),
            Token::Scalar(ScalarToken::Int("4"))
        )
    );

    assert_float_token_eq(
        "4.123",
        SourcePosition::new(0, 0, 0),
        SourcePosition::new(5, 0, 5),
        "4.123",
    );

    assert_float_token_eq(
        "4.0",
        SourcePosition::new(0, 0, 0),
        SourcePosition::new(3, 0, 3),
        "4.0",
    );

    assert_eq!(
        tokenize_single("-4"),
        Spanning::start_end(
            &SourcePosition::new(0, 0, 0),
            &SourcePosition::new(2, 0, 2),
            Token::Scalar(ScalarToken::Int("-4"))
        )
    );

    assert_eq!(
        tokenize_single("9"),
        Spanning::start_end(
            &SourcePosition::new(0, 0, 0),
            &SourcePosition::new(1, 0, 1),
            Token::Scalar(ScalarToken::Int("9"))
        )
    );

    assert_eq!(
        tokenize_single("0"),
        Spanning::start_end(
            &SourcePosition::new(0, 0, 0),
            &SourcePosition::new(1, 0, 1),
            Token::Scalar(ScalarToken::Int("0"))
        )
    );

    assert_float_token_eq(
        "-4.123",
        SourcePosition::new(0, 0, 0),
        SourcePosition::new(6, 0, 6),
        "-4.123",
    );

    assert_float_token_eq(
        "0.123",
        SourcePosition::new(0, 0, 0),
        SourcePosition::new(5, 0, 5),
        "0.123",
    );

    assert_float_token_eq(
        "123e4",
        SourcePosition::new(0, 0, 0),
        SourcePosition::new(5, 0, 5),
        "123e4",
    );

    assert_float_token_eq(
        "123E4",
        SourcePosition::new(0, 0, 0),
        SourcePosition::new(5, 0, 5),
        "123E4",
    );

    assert_float_token_eq(
        "123e-4",
        SourcePosition::new(0, 0, 0),
        SourcePosition::new(6, 0, 6),
        "123e-4",
    );

    assert_float_token_eq(
        "123e+4",
        SourcePosition::new(0, 0, 0),
        SourcePosition::new(6, 0, 6),
        "123e+4",
    );

    assert_float_token_eq(
        "-1.123e4",
        SourcePosition::new(0, 0, 0),
        SourcePosition::new(8, 0, 8),
        "-1.123e4",
    );

    assert_float_token_eq(
        "-1.123E4",
        SourcePosition::new(0, 0, 0),
        SourcePosition::new(8, 0, 8),
        "-1.123E4",
    );

    assert_float_token_eq(
        "-1.123e-4",
        SourcePosition::new(0, 0, 0),
        SourcePosition::new(9, 0, 9),
        "-1.123e-4",
    );

    assert_float_token_eq(
        "-1.123e+4",
        SourcePosition::new(0, 0, 0),
        SourcePosition::new(9, 0, 9),
        "-1.123e+4",
    );

    assert_float_token_eq(
        "-1.123e45",
        SourcePosition::new(0, 0, 0),
        SourcePosition::new(9, 0, 9),
        "-1.123e45",
    );
}

#[test]
fn numbers_errors() {
    assert_eq!(
        tokenize_error("00"),
        Spanning::zero_width(
            &SourcePosition::new(1, 0, 1),
            LexerError::UnexpectedCharacter('0')
        )
    );

    assert_eq!(
        tokenize_error("+1"),
        Spanning::zero_width(
            &SourcePosition::new(0, 0, 0),
            LexerError::UnknownCharacter('+')
        )
    );

    assert_eq!(
        tokenize_error("1."),
        Spanning::zero_width(
            &SourcePosition::new(2, 0, 2),
            LexerError::UnexpectedEndOfFile
        )
    );

    assert_eq!(
        tokenize_error(".123"),
        Spanning::zero_width(
            &SourcePosition::new(0, 0, 0),
            LexerError::UnexpectedCharacter('.')
        )
    );

    assert_eq!(
        tokenize_error("1.A"),
        Spanning::zero_width(
            &SourcePosition::new(2, 0, 2),
            LexerError::UnexpectedCharacter('A')
        )
    );

    assert_eq!(
        tokenize_error("-A"),
        Spanning::zero_width(
            &SourcePosition::new(1, 0, 1),
            LexerError::UnexpectedCharacter('A')
        )
    );

    assert_eq!(
        tokenize_error("1.0e"),
        Spanning::zero_width(
            &SourcePosition::new(4, 0, 4),
            LexerError::UnexpectedEndOfFile
        )
    );

    assert_eq!(
        tokenize_error("1.0eA"),
        Spanning::zero_width(
            &SourcePosition::new(4, 0, 4),
            LexerError::UnexpectedCharacter('A')
        )
    );
}

#[test]
fn punctuation() {
    assert_eq!(
        tokenize_single("!"),
        Spanning::single_width(&SourcePosition::new(0, 0, 0), Token::ExclamationMark)
    );

    assert_eq!(
        tokenize_single("$"),
        Spanning::single_width(&SourcePosition::new(0, 0, 0), Token::Dollar)
    );

    assert_eq!(
        tokenize_single("("),
        Spanning::single_width(&SourcePosition::new(0, 0, 0), Token::ParenOpen)
    );

    assert_eq!(
        tokenize_single(")"),
        Spanning::single_width(&SourcePosition::new(0, 0, 0), Token::ParenClose)
    );

    assert_eq!(
        tokenize_single("..."),
        Spanning::start_end(
            &SourcePosition::new(0, 0, 0),
            &SourcePosition::new(3, 0, 3),
            Token::Ellipsis
        )
    );

    assert_eq!(
        tokenize_single(":"),
        Spanning::single_width(&SourcePosition::new(0, 0, 0), Token::Colon)
    );

    assert_eq!(
        tokenize_single("="),
        Spanning::single_width(&SourcePosition::new(0, 0, 0), Token::Equals)
    );

    assert_eq!(
        tokenize_single("@"),
        Spanning::single_width(&SourcePosition::new(0, 0, 0), Token::At)
    );

    assert_eq!(
        tokenize_single("["),
        Spanning::single_width(&SourcePosition::new(0, 0, 0), Token::BracketOpen)
    );

    assert_eq!(
        tokenize_single("]"),
        Spanning::single_width(&SourcePosition::new(0, 0, 0), Token::BracketClose)
    );

    assert_eq!(
        tokenize_single("{"),
        Spanning::single_width(&SourcePosition::new(0, 0, 0), Token::CurlyOpen)
    );

    assert_eq!(
        tokenize_single("}"),
        Spanning::single_width(&SourcePosition::new(0, 0, 0), Token::CurlyClose)
    );

    assert_eq!(
        tokenize_single("|"),
        Spanning::single_width(&SourcePosition::new(0, 0, 0), Token::Pipe)
    );
}

#[test]
fn punctuation_error() {
    assert_eq!(
        tokenize_error(".."),
        Spanning::zero_width(
            &SourcePosition::new(2, 0, 2),
            LexerError::UnexpectedEndOfFile
        )
    );

    assert_eq!(
        tokenize_error("?"),
        Spanning::zero_width(
            &SourcePosition::new(0, 0, 0),
            LexerError::UnknownCharacter('?')
        )
    );

    assert_eq!(
        tokenize_error("\u{203b}"),
        Spanning::zero_width(
            &SourcePosition::new(0, 0, 0),
            LexerError::UnknownCharacter('\u{203b}')
        )
    );

    assert_eq!(
        tokenize_error("\u{200b}"),
        Spanning::zero_width(
            &SourcePosition::new(0, 0, 0),
            LexerError::UnknownCharacter('\u{200b}')
        )
    );
}

#[test]
fn display() {
    for (input, expected) in [
        (Token::Name("identifier"), "identifier"),
        (Token::Scalar(ScalarToken::Int("123")), "123"),
        (Token::Scalar(ScalarToken::Float("4.5")), "4.5"),
        (
            Token::Scalar(ScalarToken::String("some string")),
            "\"some string\"",
        ),
        (
            Token::Scalar(ScalarToken::String("string with \\ escape and \" quote")),
            "\"string with \\\\ escape and \\\" quote\"",
        ),
        (Token::ExclamationMark, "!"),
        (Token::Dollar, "$"),
        (Token::ParenOpen, "("),
        (Token::ParenClose, ")"),
        (Token::BracketOpen, "["),
        (Token::BracketClose, "]"),
        (Token::CurlyOpen, "{"),
        (Token::CurlyClose, "}"),
        (Token::Ellipsis, "..."),
        (Token::Colon, ":"),
        (Token::Equals, "="),
        (Token::At, "@"),
        (Token::Pipe, "|"),
    ] {
        assert_eq!(input.to_string(), expected);
    }
}
