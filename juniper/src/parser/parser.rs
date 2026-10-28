use std::{collections::VecDeque, fmt};

use compact_str::{CompactString, format_compact};
use derive_more::with_trait::{Display, Error};

use crate::parser::{Lexer, LexerError, Spanning, Token};

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

    /// The document nests deeper than the depth limit of the schema.
    #[display("Document nesting exceeds the limit of {_0} levels")]
    DepthLimitExceeded(#[error(not(source))] usize),

    /// The document contains more tokens than the token limit of the schema.
    #[display("Document exceeds the limit of {_0} tokens")]
    TokenLimitExceeded(#[error(not(source))] usize),

    /// An operation of the document expands into more fields than the field limit of the schema.
    #[display("Document exceeds the limit of {_0} fields")]
    FieldLimitExceeded(#[error(not(source))] usize),
}

/// Limits on the documents that a [`Parser`] accepts.
///
/// The limits protect a server from documents that cost too much to parse, validate, or execute.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Limits {
    /// Maximum nesting depth of selection sets, fragments, input values, and list types.
    pub(crate) max_depth: usize,

    /// Maximum number of tokens in a document.
    pub(crate) max_tokens: usize,

    /// Maximum number of fields in an operation, with the fields of a fragment counted once for
    /// each of its spreads.
    pub(crate) max_fields: usize,
}

impl Limits {
    /// Default value of [`Limits::max_depth`].
    ///
    /// Real documents rarely nest deeper than 20 levels. In a debug build, the parser overflows a
    /// 2 MiB thread stack at about 300 levels.
    pub(crate) const DEFAULT_MAX_DEPTH: usize = 64;

    /// Default value of [`Limits::max_tokens`].
    ///
    /// graphql-java uses the same default token limit.
    pub(crate) const DEFAULT_MAX_TOKENS: usize = 15_000;

    /// Default value of [`Limits::max_fields`].
    ///
    /// Each field takes at least one token. A document without fragments thus reaches the default
    /// token limit before this limit.
    pub(crate) const DEFAULT_MAX_FIELDS: usize = Self::DEFAULT_MAX_TOKENS;
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_depth: Self::DEFAULT_MAX_DEPTH,
            max_tokens: Self::DEFAULT_MAX_TOKENS,
            max_fields: Self::DEFAULT_MAX_FIELDS,
        }
    }
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
    tokens: VecDeque<Spanning<Token<'a>>>,
    limits: Limits,

    /// Nesting depth of the current part of the document.
    ///
    /// [`Parser::nested()`] keeps this depth within [`Limits::max_depth`].
    depth: usize,
}

impl<'a> Parser<'a> {
    #[doc(hidden)]
    pub fn new(lexer: &mut Lexer<'a>) -> Result<Self, Spanning<ParseError>> {
        Self::with_limits(lexer, Limits::default())
    }

    /// Collects the tokens of `lexer` for parsing within `limits`.
    ///
    /// Fails with [`ParseError::TokenLimitExceeded`] as soon as `lexer` produces more tokens than
    /// [`Limits::max_tokens`].
    pub(crate) fn with_limits(
        lexer: &mut Lexer<'a>,
        limits: Limits,
    ) -> Result<Self, Spanning<ParseError>> {
        let mut tokens = VecDeque::new();
        for token in lexer {
            let token = token.map_err(|e| e.map(ParseError::LexerError))?;
            if token.item != Token::EndOfFile && tokens.len() >= limits.max_tokens {
                return Err(token.map(|_| ParseError::TokenLimitExceeded(limits.max_tokens)));
            }
            tokens.push_back(token);
        }
        Ok(Self {
            tokens,
            limits,
            depth: 0,
        })
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
            Ok(self
                .tokens
                .pop_front()
                .expect("`Token::EndOfFile` always remains in `self.tokens`"))
        }
    }

    /// Parses a nested part of the document with `parse`.
    ///
    /// Fails with [`ParseError::DepthLimitExceeded`] instead of calling `parse` if the nested part
    /// exceeds [`Limits::max_depth`]. The parser is recursive, so this limit keeps it from
    /// overflowing the stack.
    pub(crate) fn nested<T>(
        &mut self,
        parse: impl FnOnce(&mut Self) -> Result<T, Spanning<ParseError>>,
    ) -> Result<T, Spanning<ParseError>> {
        if self.depth >= self.limits.max_depth {
            return Err(Spanning::new(
                self.peek().span,
                ParseError::DepthLimitExceeded(self.limits.max_depth),
            ));
        }
        self.depth += 1;
        let parsed = parse(self);
        self.depth -= 1;
        parsed
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
