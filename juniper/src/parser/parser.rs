use std::{error::Error, fmt};

use smartstring::alias::String;

use crate::parser::{Lexer, LexerError, Spanning, Token};

/// Error while parsing a GraphQL query
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ParseError {
    /// An unexpected token occurred in the source
    // TODO: Previously was `Token<'a>`.
    //       Revisit on `graphql-parser` integration.
    UnexpectedToken(String),

    /// The input source abruptly ended
    UnexpectedEndOfFile,

    /// An error during tokenization occurred
    LexerError(LexerError),

    /// A scalar of unexpected type occurred in the source
    ExpectedScalarError(&'static str),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedToken(token) => write!(f, "Unexpected \"{token}\""),
            Self::UnexpectedEndOfFile => write!(f, "Unexpected end of input"),
            Self::LexerError(e) => e.fmt(f),
            Self::ExpectedScalarError(e) => e.fmt(f),
        }
    }
}

impl Error for ParseError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::LexerError(e) => Some(e),
            Self::ExpectedScalarError(_) | Self::UnexpectedToken(_) | Self::UnexpectedEndOfFile => {
                None
            }
        }
    }
}

impl ParseError {
    /// Creates a [`ParseError::UnexpectedToken`] out of the provided [`Token`].
    #[must_use]
    pub fn unexpected_token(token: Token<'_>) -> Self {
        use std::fmt::Write as _;

        let mut s = String::new();
        // PANIC: Unwrapping is OK here, as it may panic only on allocation
        //        error.
        write!(s, "{token}").unwrap();

        Self::UnexpectedToken(s)
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
