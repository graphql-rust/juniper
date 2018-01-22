use std::result::Result;
use std::fmt;

use parser::{Lexer, LexerError, Spanning, Token};

/// Error while parsing a GraphQL query
#[derive(Debug, PartialEq)]
pub enum ParseError<'a> {
    /// An unexpected token occurred in the source
    UnexpectedToken(Token<'a>),

    /// The input source abruptly ended
    UnexpectedEndOfFile,

    /// An error during tokenization occurred
    LexerError(LexerError),
}

#[doc(hidden)]
pub type ParseResult<'a, T> = Result<Spanning<T>, Spanning<ParseError<'a>>>;

#[doc(hidden)]
pub type UnlocatedParseResult<'a, T> = Result<T, Spanning<ParseError<'a>>>;

#[doc(hidden)]
pub type OptionParseResult<'a, T> = Result<Option<Spanning<T>>, Spanning<ParseError<'a>>>;

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

        Ok(Parser { tokens: tokens })
    }

    #[doc(hidden)]
    pub fn peek(&self) -> &Spanning<Token<'a>> {
        &self.tokens[0]
    }

    #[doc(hidden)]
    pub fn next(&mut self) -> ParseResult<'a, Token<'a>> {
        if self.tokens.len() == 1 {
            Err(Spanning::start_end(
                &self.peek().start.clone(),
                &self.peek().end.clone(),
                ParseError::UnexpectedEndOfFile,
            ))
        } else {
            Ok(self.tokens.remove(0))
        }
    }

    #[doc(hidden)]
    pub fn expect(&mut self, expected: &Token) -> ParseResult<'a, Token<'a>> {
        if &self.peek().item != expected {
            Err(self.next()?.map(ParseError::UnexpectedToken))
        } else {
            self.next()
        }
    }

    #[doc(hidden)]
    pub fn skip(
        &mut self,
        expected: &Token,
    ) -> Result<Option<Spanning<Token<'a>>>, Spanning<ParseError<'a>>> {
        if &self.peek().item == expected {
            Ok(Some(self.next()?))
        } else if self.peek().item == Token::EndOfFile {
            Err(Spanning::zero_width(
                &self.peek().start,
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
    ) -> ParseResult<'a, Vec<Spanning<T>>>
    where
        T: fmt::Debug,
        F: Fn(&mut Parser<'a>) -> ParseResult<'a, T>,
    {
        let Spanning {
            start: start_pos, ..
        } = self.expect(opening)?;
        let mut items = Vec::new();

        loop {
            if let Some(Spanning { end: end_pos, .. }) = self.skip(closing)? {
                return Ok(Spanning::start_end(&start_pos, &end_pos, items));
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
    ) -> ParseResult<'a, Vec<Spanning<T>>>
    where
        T: fmt::Debug,
        F: Fn(&mut Parser<'a>) -> ParseResult<'a, T>,
    {
        let Spanning {
            start: start_pos, ..
        } = self.expect(opening)?;
        let mut items = Vec::new();

        loop {
            items.push(parser(self)?);

            if let Some(Spanning { end: end_pos, .. }) = self.skip(closing)? {
                return Ok(Spanning::start_end(&start_pos, &end_pos, items));
            }
        }
    }

    #[doc(hidden)]
    pub fn unlocated_delimited_nonempty_list<T, F>(
        &mut self,
        opening: &Token,
        parser: F,
        closing: &Token,
    ) -> ParseResult<'a, Vec<T>>
    where
        T: fmt::Debug,
        F: Fn(&mut Parser<'a>) -> UnlocatedParseResult<'a, T>,
    {
        let Spanning {
            start: start_pos, ..
        } = self.expect(opening)?;
        let mut items = Vec::new();

        loop {
            items.push(parser(self)?);

            if let Some(Spanning { end: end_pos, .. }) = self.skip(closing)? {
                return Ok(Spanning::start_end(&start_pos, &end_pos, items));
            }
        }
    }

    #[doc(hidden)]
    pub fn expect_name(&mut self) -> ParseResult<'a, &'a str> {
        match *self.peek() {
            Spanning {
                item: Token::Name(_),
                ..
            } => Ok(self.next()?.map(|token| {
                if let Token::Name(name) = token {
                    name
                } else {
                    panic!("Internal parse error in `expect_name`");
                }
            })),
            Spanning {
                item: Token::EndOfFile,
                ..
            } => Err(Spanning::start_end(
                &self.peek().start.clone(),
                &self.peek().end.clone(),
                ParseError::UnexpectedEndOfFile,
            )),
            _ => Err(self.next()?.map(ParseError::UnexpectedToken)),
        }
    }
}

impl<'a> fmt::Display for ParseError<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ParseError::UnexpectedToken(ref token) => write!(f, "Unexpected \"{}\"", token),
            ParseError::UnexpectedEndOfFile => write!(f, "Unexpected end of input"),
            ParseError::LexerError(ref err) => err.fmt(f),
        }
    }
}
