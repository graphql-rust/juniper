//! Query parser and language utilities

mod document;
mod lexer;
#[expect(clippy::module_inception, reason = "intended")]
mod parser;
mod utils;
mod value;

#[cfg(test)]
mod tests;

pub use self::document::parse_document_source;

pub use self::{
    lexer::{Lexer, LexerError, ScalarToken, Token},
    parser::{OptionParseResult, ParseError, ParseResult, Parser, UnlocatedParseResult},
    utils::{SourcePosition, Span, Spanning},
};
