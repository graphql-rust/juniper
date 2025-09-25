use std::{borrow::Cow, fmt, iter};

use compact_str::{CompactString, format_compact};
use derive_more::with_trait::{Display, Error, From};

use crate::parser::{
    Lexer, LexerError, ScalarToken, Spanning, StringLiteral, Token, UnicodeCodePoint,
};

/// Error while parsing a GraphQL query
#[derive(Clone, Debug, Display, Eq, Error, From, PartialEq)]
pub enum ParseError {
    /// An unexpected token occurred in the source
    // TODO: Previously was `Token<'a>`.
    //       Revisit on `graphql-parser` integration.
    #[display("Unexpected \"{_0}\"")]
    UnexpectedToken(#[error(not(source))] CompactString),

    /// The input source abruptly ended
    #[display("Unexpected end of input")]
    UnexpectedEndOfFile,

    /// An error during tokenization occurred
    #[from]
    LexerError(LexerError),

    /// A scalar of unexpected type occurred in the source
    ExpectedScalarError(#[error(not(source))] &'static str),
}

impl ParseError {
    /// Creates a [`ParseError::UnexpectedToken`] out of the provided [`Token`].
    #[must_use]
    pub fn unexpected_token(token: Token<'_>) -> Self {
        Self::UnexpectedToken(format_compact!("{token}"))
    }
}

#[doc(hidden)]
pub type ParseResult<T> = Result<Spanning<T>, Spanning<ParseError>>;

#[doc(hidden)]
pub type UnlocatedParseResult<T> = Result<T, Spanning<ParseError>>;

#[doc(hidden)]
pub type OptionParseResult<T> = Result<Option<Spanning<T>>, Spanning<ParseError>>;

#[doc(hidden)]
#[derive(Debug)]
pub struct Parser<'a> {
    tokens: Vec<Spanning<Token<'a>>>,
}

impl<'a> Parser<'a> {
    #[doc(hidden)]
    pub fn new(lexer: &mut Lexer<'a>) -> Result<Parser<'a>, Spanning<LexerError>> {
        let mut tokens = Vec::new();

        for res in lexer {
            match res {
                Ok(s) => tokens.push(s),
                Err(e) => return Err(e),
            }
        }

        Ok(Parser { tokens })
    }

    #[doc(hidden)]
    pub fn peek(&self) -> &Spanning<Token<'a>> {
        &self.tokens[0]
    }

    #[doc(hidden)]
    pub fn next_token(&mut self) -> ParseResult<Token<'a>> {
        if self.tokens.len() == 1 {
            Err(Spanning::new(
                self.peek().span,
                ParseError::UnexpectedEndOfFile,
            ))
        } else {
            Ok(self.tokens.remove(0))
        }
    }

    #[doc(hidden)]
    pub fn expect(&mut self, expected: &Token) -> ParseResult<Token<'a>> {
        if &self.peek().item != expected {
            Err(self.next_token()?.map(ParseError::unexpected_token))
        } else {
            self.next_token()
        }
    }

    #[doc(hidden)]
    pub fn skip(
        &mut self,
        expected: &Token,
    ) -> Result<Option<Spanning<Token<'a>>>, Spanning<ParseError>> {
        if &self.peek().item == expected {
            Ok(Some(self.next_token()?))
        } else if self.peek().item == Token::EndOfFile {
            Err(Spanning::zero_width(
                &self.peek().span.start,
                ParseError::UnexpectedEndOfFile,
            ))
        } else {
            Ok(None)
        }
    }

    #[doc(hidden)]
    pub fn delimited_list<T, F>(
        &mut self,
        opening: &Token,
        parser: F,
        closing: &Token,
    ) -> ParseResult<Vec<Spanning<T>>>
    where
        T: fmt::Debug,
        F: Fn(&mut Parser<'a>) -> ParseResult<T>,
    {
        let start_pos = &self.expect(opening)?.span.start;
        let mut items = Vec::new();

        loop {
            if let Some(Spanning { span, .. }) = self.skip(closing)? {
                return Ok(Spanning::start_end(start_pos, &span.end, items));
            }

            items.push(parser(self)?);
        }
    }

    #[doc(hidden)]
    pub fn delimited_nonempty_list<T, F>(
        &mut self,
        opening: &Token,
        parser: F,
        closing: &Token,
    ) -> ParseResult<Vec<Spanning<T>>>
    where
        T: fmt::Debug,
        F: Fn(&mut Parser<'a>) -> ParseResult<T>,
    {
        let start_pos = &self.expect(opening)?.span.start;
        let mut items = Vec::new();

        loop {
            items.push(parser(self)?);

            if let Some(end_spanning) = self.skip(closing)? {
                return Ok(Spanning::start_end(start_pos, &end_spanning.end(), items));
            }
        }
    }

    #[doc(hidden)]
    pub fn unlocated_delimited_nonempty_list<T, F>(
        &mut self,
        opening: &Token,
        parser: F,
        closing: &Token,
    ) -> ParseResult<Vec<T>>
    where
        T: fmt::Debug,
        F: Fn(&mut Parser<'a>) -> UnlocatedParseResult<T>,
    {
        let start_pos = &self.expect(opening)?.span.start;
        let mut items = Vec::new();

        loop {
            items.push(parser(self)?);

            if let Some(end_spanning) = self.skip(closing)? {
                return Ok(Spanning::start_end(start_pos, &end_spanning.end(), items));
            }
        }
    }

    #[doc(hidden)]
    pub fn expect_name(&mut self) -> ParseResult<&'a str> {
        match *self.peek() {
            Spanning {
                item: Token::Name(_),
                ..
            } => Ok(self.next_token()?.map(|token| {
                if let Token::Name(name) = token {
                    name
                } else {
                    panic!("Internal parse error in `expect_name`");
                }
            })),
            Spanning {
                item: Token::EndOfFile,
                ..
            } => Err(Spanning::new(
                self.peek().span,
                ParseError::UnexpectedEndOfFile,
            )),
            _ => Err(self.next_token()?.map(ParseError::unexpected_token)),
        }
    }
}

impl<'a> StringLiteral<'a> {
    /// Parses this [`StringLiteral`] returning an unescaped and unquoted string value.
    ///
    /// # Errors
    ///
    /// If this [`StringLiteral`] is invalid.
    pub fn parse(self) -> Result<Cow<'a, str>, ParseError> {
        match self {
            Self::Quoted(lit) => {
                if !lit.starts_with('"') {
                    return Err(ParseError::unexpected_token(Token::Scalar(
                        ScalarToken::String(self),
                    )));
                }
                if !lit.ends_with('"') {
                    return Err(LexerError::UnterminatedString.into());
                }

                let unquoted = &lit[1..lit.len() - 1];
                if !unquoted.contains('\\') {
                    return Ok(unquoted.into());
                }

                let mut unescaped = String::with_capacity(unquoted.len());
                let mut char_iter = unquoted.chars();
                while let Some(ch) = char_iter.next() {
                    match ch {
                        // StringCharacter ::
                        //     SourceCharacter but not " or \ or LineTerminator
                        //     \uEscapedUnicode
                        //     \EscapedCharacter
                        '\\' => match char_iter.next() {
                            // EscapedCharacter :: one of
                            //     " \ / b f n r t
                            Some('"') => unescaped.push('"'),
                            Some('\\') => unescaped.push('\\'),
                            Some('/') => unescaped.push('/'),
                            Some('b') => unescaped.push('\u{0008}'),
                            Some('f') => unescaped.push('\u{000C}'),
                            Some('n') => unescaped.push('\n'),
                            Some('r') => unescaped.push('\r'),
                            Some('t') => unescaped.push('\t'),
                            // EscapedUnicode ::
                            //     {HexDigit[list]}
                            //     HexDigit HexDigit HexDigit HexDigit
                            Some('u') => {
                                let mut code_point =
                                    UnicodeCodePoint::parse_escaped(&mut char_iter)?;
                                if code_point.is_high_surrogate() {
                                    let (Some('\\'), Some('u')) =
                                        (char_iter.next(), char_iter.next())
                                    else {
                                        return Err(LexerError::UnknownEscapeSequence(
                                            code_point.to_string(),
                                        )
                                        .into());
                                    };

                                    let trailing_code_point =
                                        UnicodeCodePoint::parse_escaped(&mut char_iter)?;
                                    if !trailing_code_point.is_low_surrogate() {
                                        return Err(LexerError::UnknownEscapeSequence(
                                            code_point.to_string(),
                                        )
                                        .into());
                                    }
                                    code_point = UnicodeCodePoint::from_surrogate_pair(
                                        code_point,
                                        trailing_code_point,
                                    );
                                }
                                unescaped.push(code_point.try_into_char()?);
                            }
                            Some(s) => {
                                return Err(
                                    LexerError::UnknownEscapeSequence(format!(r"\{s}")).into()
                                );
                            }
                            None => {
                                return Err(LexerError::UnterminatedString.into());
                            }
                        },
                        ch => {
                            unescaped.push(ch);
                        }
                    }
                }
                Ok(unescaped.into())
            }
            Self::Block(lit) => {
                if !lit.starts_with(r#"""""#) {
                    return Err(ParseError::unexpected_token(Token::Scalar(
                        ScalarToken::String(self),
                    )));
                }
                if !lit.ends_with(r#"""""#) {
                    return Err(LexerError::UnterminatedBlockString.into());
                }

                let unquoted = &lit[3..lit.len() - 3];

                let (mut indent, mut total_lines) = (usize::MAX, 0);
                let (mut first_text_line, mut last_text_line) = (None, 0);
                for (n, line) in unquoted.lines().enumerate() {
                    total_lines += 1;

                    let trimmed = line.trim_start();
                    if trimmed.is_empty() {
                        continue;
                    }

                    _ = first_text_line.get_or_insert(n);
                    last_text_line = n;

                    if n != 0 {
                        indent = indent.min(line.len() - trimmed.len());
                    }
                }

                let Some(first_text_line) = first_text_line else {
                    return Ok("".into()); // no text, only whitespaces
                };
                if (indent == 0 || total_lines == 1) && !unquoted.contains(r#"\""""#) {
                    return Ok(unquoted.into()); // nothing to dedent or unescape
                }

                let mut unescaped = String::with_capacity(unquoted.len());
                let mut lines = unquoted
                    .lines()
                    .enumerate()
                    .skip(first_text_line)
                    .take(last_text_line - first_text_line + 1)
                    .map(|(n, line)| {
                        if n != 0 && line.len() >= indent {
                            &line[indent..]
                        } else {
                            line
                        }
                    })
                    .map(|x| x.replace(r#"\""""#, r#"""""#));
                if let Some(line) = lines.next() {
                    unescaped.push_str(&line);
                    for line in lines {
                        unescaped.push('\n');
                        unescaped.push_str(&line);
                    }
                }
                Ok(unescaped.into())
            }
        }
    }
}

impl UnicodeCodePoint {
    /// Parses a [`UnicodeCodePoint`] from an [escaped] value in the provided [`Iterator`].
    ///
    /// [escaped]: https://spec.graphql.org/September2025#EscapedUnicode
    pub(crate) fn parse_escaped(
        char_iter: &mut impl Iterator<Item = char>,
    ) -> Result<Self, ParseError> {
        // EscapedUnicode ::
        //     {HexDigit[list]}
        //     HexDigit HexDigit HexDigit HexDigit

        let Some(mut curr_ch) = char_iter.next() else {
            return Err(LexerError::UnknownEscapeSequence(r"\u".into()).into());
        };
        let mut escaped_code_point = String::with_capacity(6); // `\u{10FFFF}` is max code point

        let is_variable_width = curr_ch == '{';
        if is_variable_width {
            loop {
                curr_ch = char_iter.next().ok_or_else(|| {
                    LexerError::UnknownEscapeSequence(format!(r"\u{{{escaped_code_point}"))
                })?;
                if curr_ch == '}' {
                    break;
                } else if !curr_ch.is_alphanumeric() {
                    return Err(LexerError::UnknownEscapeSequence(format!(
                        r"\u{{{escaped_code_point}"
                    ))
                    .into());
                }
                escaped_code_point.push(curr_ch);
            }
        } else {
            let mut char_iter = iter::once(curr_ch).chain(char_iter);
            for _ in 0..4 {
                curr_ch = char_iter.next().ok_or_else(|| {
                    LexerError::UnknownEscapeSequence(format!(r"\u{escaped_code_point}"))
                })?;
                if !curr_ch.is_alphanumeric() {
                    return Err(LexerError::UnknownEscapeSequence(format!(
                        r"\u{escaped_code_point}"
                    ))
                    .into());
                }
                escaped_code_point.push(curr_ch);
            }
        }

        let Ok(code) = u32::from_str_radix(&escaped_code_point, 16) else {
            return Err(LexerError::UnknownEscapeSequence(if is_variable_width {
                format!(r"\u{{{escaped_code_point}}}")
            } else {
                format!(r"\u{escaped_code_point}")
            })
            .into());
        };

        Ok(Self {
            code,
            is_variable_width,
        })
    }
}

#[cfg(test)]
mod string_literal_tests {
    use super::StringLiteral;

    #[test]
    fn quoted() {
        for (input, expected) in [
            (r#""""#, ""),
            (r#""simple""#, "simple"),
            (r#"" white space ""#, " white space "),
            (r#""quote \"""#, r#"quote ""#),
            (r#""escaped \n\r\b\t\f""#, "escaped \n\r\u{0008}\t\u{000c}"),
            (r#""slashes \\ \/""#, r"slashes \ /"),
            (
                r#""unicode \u1234\u5678\u90AB\uCDEF""#,
                "unicode \u{1234}\u{5678}\u{90ab}\u{cdef}",
            ),
            (
                r#""string with unicode escape outside BMP \u{1F600}""#,
                "string with unicode escape outside BMP \u{1F600}",
            ),
            (
                r#""string with minimal unicode escape \u{0}""#,
                "string with minimal unicode escape \u{0}",
            ),
            (
                r#""string with maximal unicode escape \u{10FFFF}""#,
                "string with maximal unicode escape \u{10FFFF}",
            ),
            (
                r#""string with maximal minimal unicode escape \u{000000}""#,
                "string with maximal minimal unicode escape \u{000000}",
            ),
            (
                r#""string with unicode surrogate pair escape \uD83D\uDE00""#,
                "string with unicode surrogate pair escape \u{1f600}",
            ),
            (
                r#""string with minimal surrogate pair escape \uD800\uDC00""#,
                "string with minimal surrogate pair escape \u{10000}",
            ),
            (
                r#""string with maximal surrogate pair escape \uDBFF\uDFFF""#,
                "string with maximal surrogate pair escape \u{10FFFF}",
            ),
        ] {
            let res = StringLiteral::Quoted(input).parse();
            assert!(
                res.is_ok(),
                "parsing error occurred on {input}: {}",
                res.unwrap_err(),
            );

            assert_eq!(res.unwrap(), expected);
        }
    }

    #[test]
    fn quoted_errors() {
        for (input, expected) in [
            (
                r#""bad surrogate \uDEAD""#,
                r#"Unknown escape sequence "\uDEAD" in string"#,
            ),
            (
                r#""bad low surrogate pair \uD800\uD800""#,
                r#"Unknown escape sequence "\uD800" in string"#,
            ),
        ] {
            let res = StringLiteral::Quoted(input).parse();
            assert!(res.is_err(), "parsing error doesn't occur on {input}");

            let err = res.unwrap_err();
            assert!(
                err.to_string().contains(expected),
                "returned error `{err}` doesn't contain `{expected}`",
            );
        }
    }

    #[test]
    fn block() {
        for (input, expected) in [
            (r#""""""""#, ""),
            (r#""""simple""""#, "simple"),
            (r#"""" white space """"#, " white space "),
            (r#""""contains " quote""""#, r#"contains " quote"#),
            (
                r#""""contains \""" triple quote""""#,
                r#"contains """ triple quote"#,
            ),
            (
                r#""""contains \"" double quote""""#,
                r#"contains \"" double quote"#,
            ),
            (
                r#""""contains \\""" triple quote""""#,
                r#"contains \""" triple quote"#,
            ),
            (r#""""\"""quote" """"#, r#""""quote" "#),
            (r#""""multi\nline""""#, r"multi\nline"),
            (
                r#""""multi\rline\r\nnormalized""""#,
                r"multi\rline\r\nnormalized",
            ),
            (
                r#""""unescaped \\n\\r\\b\\t\\f\\u1234""""#,
                r"unescaped \\n\\r\\b\\t\\f\\u1234",
            ),
            (
                r#""""unescaped unicode outside BMP \u{1f600}""""#,
                r"unescaped unicode outside BMP \u{1f600}",
            ),
            (r#""""slashes \\\\ \\/""""#, r"slashes \\\\ \\/"),
            (
                r#""""

        spans
          multiple
            lines

        """"#,
                "spans\n  multiple\n    lines",
            ),
            // removes uniform indentation
            (
                r#""""
    Hello,
      World!

    Yours,
      GraphQL.""""#,
                "Hello,\n  World!\n\nYours,\n  GraphQL.",
            ),
            // removes empty leading and trailing lines
            (
                r#""""

    Hello,
      World!

    Yours,
      GraphQL.

        """"#,
                "Hello,\n  World!\n\nYours,\n  GraphQL.",
            ),
            // retains indentation from first line
            (
                r#""""    Hello,
      World!

    Yours,
      GraphQL.""""#,
                "    Hello,\n  World!\n\nYours,\n  GraphQL.",
            ),
            // does not alter trailing spaces
            (
                r#""""
    Hello,
      World!

    Yours,
      GraphQL.   """"#,
                "Hello,\n  World!\n\nYours,\n  GraphQL.   ",
            ),
        ] {
            let res = StringLiteral::Block(input).parse();
            assert!(
                res.is_ok(),
                "parsing error occurred on {input}: {}",
                res.unwrap_err(),
            );

            assert_eq!(res.unwrap(), expected);
        }
    }
}
