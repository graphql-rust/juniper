use std::{char, fmt, iter::Peekable, str::CharIndices};

use crate::parser::{SourcePosition, Spanning};

#[doc(hidden)]
#[derive(Debug)]
pub struct Lexer<'a> {
    iterator: Peekable<CharIndices<'a>>,
    source: &'a str,
    length: usize,
    position: SourcePosition,
    has_reached_eof: bool,
}

/// A single scalar value literal
///
/// This is only used for tagging how the lexer has interpreted a value literal
#[allow(missing_docs)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScalarToken<'a> {
    String(&'a str),
    Float(&'a str),
    Int(&'a str),
}

/// A single token in the input source
#[allow(missing_docs)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Token<'a> {
    Name(&'a str),
    Scalar(ScalarToken<'a>),
    ExclamationMark,
    Dollar,
    ParenOpen,
    ParenClose,
    BracketOpen,
    BracketClose,
    CurlyOpen,
    CurlyClose,
    Ellipsis,
    Colon,
    Equals,
    At,
    Pipe,
    EndOfFile,
}

/// Error when tokenizing the input source
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LexerError {
    /// An unknown character was found
    ///
    /// Unknown characters are characters that do not occur anywhere in the
    /// GraphQL language, such as `?` or `%`.
    UnknownCharacter(char),

    /// An unexpected character was found
    ///
    /// Unexpected characters are characters that _do_ exist in the GraphQL
    /// language, but is not expected at the current position in the document.
    UnexpectedCharacter(char),

    /// An unterminated string literal was found
    ///
    /// Apart from forgetting the ending `"`, terminating a string within a
    /// Unicode escape sequence or having a line break in the string also
    /// causes this error.
    UnterminatedString,

    /// An unknown character in a string literal was found
    ///
    /// This occurs when an invalid source character is found in a string
    /// literal, such as ASCII control characters.
    UnknownCharacterInString(char),

    /// An unknown escape sequence in a string literal was found
    ///
    /// Only a limited set of escape sequences are supported, this is emitted
    /// when e.g. `"\l"` is parsed.
    UnknownEscapeSequence(String),

    /// The input source was unexpectedly terminated
    ///
    /// Emitted when the current token requires a succeeding character, but
    /// the source has reached EOF. Emitted when scanning e.g. `"1."`.
    UnexpectedEndOfFile,

    /// An invalid number literal was found
    InvalidNumber,
}

pub type LexerResult<'a> = Result<Spanning<Token<'a>>, Spanning<LexerError>>;

impl<'a> Lexer<'a> {
    #[doc(hidden)]
    pub fn new(source: &'a str) -> Lexer<'a> {
        Lexer {
            iterator: source.char_indices().peekable(),
            source,
            length: source.len(),
            position: SourcePosition::new_origin(),
            has_reached_eof: false,
        }
    }

    fn peek_char(&mut self) -> Option<(usize, char)> {
        assert!(self.position.index() <= self.length);
        assert!(!self.has_reached_eof);

        self.iterator.peek().map(|&(idx, ch)| (idx, ch))
    }

    fn next_char(&mut self) -> Option<(usize, char)> {
        assert!(self.position.index() <= self.length);
        assert!(!self.has_reached_eof);

        let next = self.iterator.next();

        if let Some((_, ch)) = next {
            if ch == '\n' {
                self.position.advance_line();
            } else {
                self.position.advance_col();
            }
        }

        next
    }

    fn emit_single_char(&mut self, t: Token<'a>) -> Spanning<Token<'a>> {
        assert!(self.position.index() <= self.length);

        let start_pos = self.position;

        self.next_char()
            .expect("Internal error in GraphQL lexer: emit_single_char reached EOF");

        Spanning::single_width(&start_pos, t)
    }

    fn scan_over_whitespace(&mut self) {
        while let Some((_, ch)) = self.peek_char() {
            if ch == '\t' || ch == ' ' || ch == '\n' || ch == '\r' || ch == ',' {
                self.next_char();
            } else if ch == '#' {
                self.next_char();

                while let Some((_, ch)) = self.peek_char() {
                    if is_source_char(ch) && (ch == '\n' || ch == '\r') {
                        self.next_char();
                        break;
                    } else if is_source_char(ch) {
                        self.next_char();
                    } else {
                        break;
                    }
                }
            } else {
                break;
            }
        }
    }

    fn scan_ellipsis(&mut self) -> LexerResult<'a> {
        let start_pos = self.position;

        for _ in 0..3 {
            let (_, ch) = self.next_char().ok_or_else(|| {
                Spanning::zero_width(&self.position, LexerError::UnexpectedEndOfFile)
            })?;
            if ch != '.' {
                return Err(Spanning::zero_width(
                    &start_pos,
                    LexerError::UnexpectedCharacter('.'),
                ));
            }
        }

        Ok(Spanning::start_end(
            &start_pos,
            &self.position,
            Token::Ellipsis,
        ))
    }

    fn scan_name(&mut self) -> LexerResult<'a> {
        let start_pos = self.position;
        let (start_idx, start_ch) = self
            .next_char()
            .ok_or_else(|| Spanning::zero_width(&self.position, LexerError::UnexpectedEndOfFile))?;
        assert!(is_name_start(start_ch));

        let mut end_idx = start_idx;

        while let Some((idx, ch)) = self.peek_char() {
            if is_name_cont(ch) {
                self.next_char();
                end_idx = idx;
            } else {
                break;
            }
        }

        Ok(Spanning::start_end(
            &start_pos,
            &self.position,
            Token::Name(&self.source[start_idx..=end_idx]),
        ))
    }

    fn scan_string(&mut self) -> LexerResult<'a> {
        let start_pos = self.position;
        let (start_idx, start_ch) = self
            .next_char()
            .ok_or_else(|| Spanning::zero_width(&self.position, LexerError::UnexpectedEndOfFile))?;
        if start_ch != '"' {
            return Err(Spanning::zero_width(
                &self.position,
                LexerError::UnterminatedString,
            ));
        }

        let mut escaped = false;
        let mut old_pos = self.position;
        while let Some((idx, ch)) = self.next_char() {
            match ch {
                'b' | 'f' | 'n' | 'r' | 't' | '\\' | '/' | '"' if escaped => {
                    escaped = false;
                }
                'u' if escaped => {
                    self.scan_escaped_unicode(&old_pos)?;
                    escaped = false;
                }
                c if escaped => {
                    return Err(Spanning::zero_width(
                        &old_pos,
                        LexerError::UnknownEscapeSequence(format!("\\{c}")),
                    ));
                }
                '\\' => escaped = true,
                '"' if !escaped => {
                    return Ok(Spanning::start_end(
                        &start_pos,
                        &self.position,
                        Token::Scalar(ScalarToken::String(&self.source[start_idx + 1..idx])),
                    ));
                }
                '\n' | '\r' => {
                    return Err(Spanning::zero_width(
                        &old_pos,
                        LexerError::UnterminatedString,
                    ));
                }
                c if !is_source_char(c) => {
                    return Err(Spanning::zero_width(
                        &old_pos,
                        LexerError::UnknownCharacterInString(ch),
                    ));
                }
                _ => {}
            }
            old_pos = self.position;
        }

        Err(Spanning::zero_width(
            &self.position,
            LexerError::UnterminatedString,
        ))
    }

    fn scan_escaped_unicode(
        &mut self,
        start_pos: &SourcePosition,
    ) -> Result<(), Spanning<LexerError>> {
        let (start_idx, _) = self
            .peek_char()
            .ok_or_else(|| Spanning::zero_width(&self.position, LexerError::UnterminatedString))?;
        let mut end_idx = start_idx;
        let mut len = 0;

        for _ in 0..4 {
            let (idx, ch) = self.next_char().ok_or_else(|| {
                Spanning::zero_width(&self.position, LexerError::UnterminatedString)
            })?;

            if !ch.is_alphanumeric() {
                break;
            }

            end_idx = idx;
            len += 1;
        }

        // Make sure we are on a valid char boundary.
        let escape = self
            .source
            .get(start_idx..=end_idx)
            .ok_or_else(|| Spanning::zero_width(&self.position, LexerError::UnterminatedString))?;

        if len != 4 {
            return Err(Spanning::zero_width(
                start_pos,
                LexerError::UnknownEscapeSequence(format!("\\u{escape}")),
            ));
        }

        let code_point = u32::from_str_radix(escape, 16).map_err(|_| {
            Spanning::zero_width(
                start_pos,
                LexerError::UnknownEscapeSequence(format!("\\u{escape}")),
            )
        })?;

        char::from_u32(code_point)
            .ok_or_else(|| {
                Spanning::zero_width(
                    start_pos,
                    LexerError::UnknownEscapeSequence("\\u".to_owned() + escape),
                )
            })
            .map(|_| ())
    }

    fn scan_number(&mut self) -> LexerResult<'a> {
        let start_pos = self.position;
        let (start_idx, _) = self
            .peek_char()
            .ok_or_else(|| Spanning::zero_width(&self.position, LexerError::UnexpectedEndOfFile))?;

        let mut last_idx = start_idx;
        let mut last_char = '1';
        let mut is_float = false;

        let mut end_idx = loop {
            if let Some((idx, ch)) = self.peek_char() {
                if ch.is_ascii_digit() || (ch == '-' && last_idx == start_idx) {
                    if ch == '0' && last_char == '0' && last_idx == start_idx {
                        return Err(Spanning::zero_width(
                            &self.position,
                            LexerError::UnexpectedCharacter('0'),
                        ));
                    }
                    self.next_char();
                    last_char = ch;
                } else if last_char == '-' {
                    return Err(Spanning::zero_width(
                        &self.position,
                        LexerError::UnexpectedCharacter(ch),
                    ));
                } else {
                    break idx;
                }
                last_idx = idx;
            } else {
                break last_idx + 1;
            }
        };

        if let Some((start_idx, '.')) = self.peek_char() {
            is_float = true;
            let mut last_idx = start_idx;
            self.next_char();
            end_idx = loop {
                if let Some((idx, ch)) = self.peek_char() {
                    if ch.is_ascii_digit() {
                        self.next_char();
                    } else if last_idx == start_idx {
                        return Err(Spanning::zero_width(
                            &self.position,
                            LexerError::UnexpectedCharacter(ch),
                        ));
                    } else {
                        break idx;
                    }
                    last_idx = idx;
                } else if last_idx == start_idx {
                    return Err(Spanning::zero_width(
                        &self.position,
                        LexerError::UnexpectedEndOfFile,
                    ));
                } else {
                    break last_idx + 1;
                }
            };
        }
        if let Some((start_idx, ch)) = self.peek_char() {
            if ch == 'e' || ch == 'E' {
                is_float = true;
                self.next_char();
                let mut last_idx = start_idx;

                end_idx = loop {
                    if let Some((idx, ch)) = self.peek_char() {
                        if ch.is_ascii_digit()
                            || (last_idx == start_idx && (ch == '-' || ch == '+'))
                        {
                            self.next_char();
                        } else if last_idx == start_idx {
                            // 1e is not a valid floating point number
                            return Err(Spanning::zero_width(
                                &self.position,
                                LexerError::UnexpectedCharacter(ch),
                            ));
                        } else {
                            break idx;
                        }
                        last_idx = idx;
                    } else if last_idx == start_idx {
                        // 1e is not a valid floting point number
                        return Err(Spanning::zero_width(
                            &self.position,
                            LexerError::UnexpectedEndOfFile,
                        ));
                    } else {
                        break last_idx + 1;
                    }
                };
            }
        }
        let number = &self.source[start_idx..end_idx];
        let end_pos = &self.position;

        let token = if is_float {
            Token::Scalar(ScalarToken::Float(number))
        } else {
            Token::Scalar(ScalarToken::Int(number))
        };

        Ok(Spanning::start_end(&start_pos, end_pos, token))
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = LexerResult<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.has_reached_eof {
            return None;
        }

        self.scan_over_whitespace();

        let ch = self.iterator.peek().map(|&(_, ch)| ch);

        Some(match ch {
            Some('!') => Ok(self.emit_single_char(Token::ExclamationMark)),
            Some('$') => Ok(self.emit_single_char(Token::Dollar)),
            Some('(') => Ok(self.emit_single_char(Token::ParenOpen)),
            Some(')') => Ok(self.emit_single_char(Token::ParenClose)),
            Some('[') => Ok(self.emit_single_char(Token::BracketOpen)),
            Some(']') => Ok(self.emit_single_char(Token::BracketClose)),
            Some('{') => Ok(self.emit_single_char(Token::CurlyOpen)),
            Some('}') => Ok(self.emit_single_char(Token::CurlyClose)),
            Some(':') => Ok(self.emit_single_char(Token::Colon)),
            Some('=') => Ok(self.emit_single_char(Token::Equals)),
            Some('@') => Ok(self.emit_single_char(Token::At)),
            Some('|') => Ok(self.emit_single_char(Token::Pipe)),
            Some('.') => self.scan_ellipsis(),
            Some('"') => self.scan_string(),
            Some(ch) => {
                if is_number_start(ch) {
                    self.scan_number()
                } else if is_name_start(ch) {
                    self.scan_name()
                } else {
                    Err(Spanning::zero_width(
                        &self.position,
                        LexerError::UnknownCharacter(ch),
                    ))
                }
            }
            None => {
                self.has_reached_eof = true;
                Ok(Spanning::zero_width(&self.position, Token::EndOfFile))
            }
        })
    }
}

impl<'a> fmt::Display for Token<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Token::Name(name) => write!(f, "{name}"),
            Token::Scalar(ScalarToken::Int(s)) | Token::Scalar(ScalarToken::Float(s)) => {
                write!(f, "{s}")
            }
            Token::Scalar(ScalarToken::String(s)) => {
                write!(f, "\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
            }
            Token::ExclamationMark => write!(f, "!"),
            Token::Dollar => write!(f, "$"),
            Token::ParenOpen => write!(f, "("),
            Token::ParenClose => write!(f, ")"),
            Token::BracketOpen => write!(f, "["),
            Token::BracketClose => write!(f, "]"),
            Token::CurlyOpen => write!(f, "{{"),
            Token::CurlyClose => write!(f, "}}"),
            Token::Ellipsis => write!(f, "..."),
            Token::Colon => write!(f, ":"),
            Token::Equals => write!(f, "="),
            Token::At => write!(f, "@"),
            Token::Pipe => write!(f, "|"),
            Token::EndOfFile => write!(f, "End of file"),
        }
    }
}

fn is_source_char(c: char) -> bool {
    c == '\t' || c == '\n' || c == '\r' || c >= ' '
}

fn is_name_start(c: char) -> bool {
    c == '_' || c.is_ascii_alphabetic()
}

fn is_name_cont(c: char) -> bool {
    is_name_start(c) || c.is_ascii_digit()
}

fn is_number_start(c: char) -> bool {
    c == '-' || c.is_ascii_digit()
}

impl fmt::Display for LexerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            LexerError::UnknownCharacter(c) => write!(f, "Unknown character \"{c}\""),
            LexerError::UnterminatedString => write!(f, "Unterminated string literal"),
            LexerError::UnknownCharacterInString(c) => {
                write!(f, "Unknown character \"{c}\" in string literal")
            }
            LexerError::UnknownEscapeSequence(ref s) => {
                write!(f, "Unknown escape sequence \"{s}\" in string")
            }
            LexerError::UnexpectedCharacter(c) => write!(f, "Unexpected character \"{c}\""),
            LexerError::UnexpectedEndOfFile => write!(f, "Unexpected end of input"),
            LexerError::InvalidNumber => write!(f, "Invalid number literal"),
        }
    }
}

impl std::error::Error for LexerError {}
