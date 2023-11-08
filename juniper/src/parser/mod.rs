//! Query parser and language utilities
#![allow(clippy::module_inception)]

mod document;
mod lexer;
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
