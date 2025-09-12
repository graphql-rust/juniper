use std::{borrow::Cow, fmt};

use compact_str::{CompactString, format_compact};
use derive_more::with_trait::{Display, Error};

use crate::parser::{Lexer, LexerError, ScalarToken, Spanning, StringLiteral, Token};

/// Error while parsing a GraphQL query
#[derive(Clone, Debug, Display, Eq, Error, PartialEq)]
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
                    return Err(ParseError::LexerError(LexerError::UnterminatedString));
                }

                let unquoted = &lit[1..lit.len() - 1];
                if !unquoted.contains('\\') {
                    return Ok(unquoted.into());
                }

                let mut unescaped = String::with_capacity(unquoted.len());
                let mut char_iter = unquoted.chars();
                while let Some(ch) = char_iter.next() {
                    match ch {
                        '\\' => match char_iter.next() {
                            Some('"') => {
                                unescaped.push('"');
                            }
                            Some('/') => {
                                unescaped.push('/');
                            }
                            Some('n') => {
                                unescaped.push('\n');
                            }
                            Some('r') => {
                                unescaped.push('\r');
                            }
                            Some('t') => {
                                unescaped.push('\t');
                            }
                            Some('\\') => {
                                unescaped.push('\\');
                            }
                            Some('f') => {
                                unescaped.push('\u{000c}');
                            }
                            Some('b') => {
                                unescaped.push('\u{0008}');
                            }
                            Some('u') => {
                                unescaped.push(parse_unicode_codepoint(&mut char_iter)?);
                            }
                            Some(s) => {
                                return Err(ParseError::LexerError(
                                    LexerError::UnknownEscapeSequence(format!(r"\{s}")),
                                ));
                            }
                            None => {
                                return Err(ParseError::LexerError(LexerError::UnterminatedString));
                            }
                        },
                        ch => {
                            unescaped.push(ch);
                        }
                    }
                }
                Ok(unescaped.into())
            }
            Self::Block(_) => todo!(),
        }
    }
}

fn parse_unicode_codepoint<I>(char_iter: &mut I) -> Result<char, ParseError>
where
    I: Iterator<Item = char>,
{
    let escaped_code_point = char_iter
        .next()
        .ok_or_else(|| ParseError::LexerError(LexerError::UnknownEscapeSequence(r"\u".into())))
        .and_then(|c1| {
            char_iter
                .next()
                .map(|c2| format!("{c1}{c2}"))
                .ok_or_else(|| {
                    ParseError::LexerError(LexerError::UnknownEscapeSequence(format!(r"\u{c1}")))
                })
        })
        .and_then(|mut s| {
            char_iter
                .next()
                .ok_or_else(|| {
                    ParseError::LexerError(LexerError::UnknownEscapeSequence(format!(r"\u{s}")))
                })
                .map(|c2| {
                    s.push(c2);
                    s
                })
        })
        .and_then(|mut s| {
            char_iter
                .next()
                .ok_or_else(|| {
                    ParseError::LexerError(LexerError::UnknownEscapeSequence(format!(r"\u{s}")))
                })
                .map(|c2| {
                    s.push(c2);
                    s
                })
        })?;
    let code_point = u32::from_str_radix(&escaped_code_point, 16).map_err(|_| {
        ParseError::LexerError(LexerError::UnknownEscapeSequence(format!(
            r"\u{escaped_code_point}",
        )))
    })?;
    char::from_u32(code_point).ok_or_else(|| {
        ParseError::LexerError(LexerError::UnknownEscapeSequence(format!(
            r"\u{escaped_code_point}",
        )))
    })
}
