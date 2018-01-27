use std::char;
use std::str::CharIndices;
use std::iter::{Iterator, Peekable};
use std::result::Result;
use std::fmt;

use parser::{SourcePosition, Spanning};

#[doc(hidden)]
#[derive(Debug)]
pub struct Lexer<'a> {
    iterator: Peekable<CharIndices<'a>>,
    source: &'a str,
    length: usize,
    position: SourcePosition,
    has_reached_eof: bool,
}

/// A single token in the input source
#[derive(Debug, PartialEq)]
#[allow(missing_docs)]
pub enum Token<'a> {
    Name(&'a str),
    Int(i32),
    Float(f64),
    String(String),
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
#[derive(Debug, PartialEq, Eq)]
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
            source: source,
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

        let start_pos = self.position.clone();

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
        let start_pos = self.position.clone();

        for _ in 0..3 {
            let (_, ch) = self.next_char().ok_or(Spanning::zero_width(
                &self.position,
                LexerError::UnexpectedEndOfFile,
            ))?;
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
        let start_pos = self.position.clone();
        let (start_idx, start_ch) = self.next_char().ok_or(Spanning::zero_width(
            &self.position,
            LexerError::UnexpectedEndOfFile,
        ))?;
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
            Token::Name(&self.source[start_idx..end_idx + 1]),
        ))
    }

    fn scan_string(&mut self) -> LexerResult<'a> {
        let start_pos = self.position.clone();
        let (_, start_ch) = self.next_char().ok_or(Spanning::zero_width(
            &self.position,
            LexerError::UnexpectedEndOfFile,
        ))?;
        assert!(start_ch == '"');

        let mut acc = String::new();

        while let Some((_, ch)) = self.peek_char() {
            if ch == '"' {
                self.next_char();
                return Ok(Spanning::start_end(
                    &start_pos,
                    &self.position,
                    Token::String(acc),
                ));
            } else if ch == '\\' {
                self.next_char();

                match self.peek_char() {
                    Some((_, '"')) => {
                        self.next_char();
                        acc.push('"');
                    }
                    Some((_, '\\')) => {
                        self.next_char();
                        acc.push('\\');
                    }
                    Some((_, '/')) => {
                        self.next_char();
                        acc.push('/');
                    }
                    Some((_, 'b')) => {
                        self.next_char();
                        acc.push('\u{0008}');
                    }
                    Some((_, 'f')) => {
                        self.next_char();
                        acc.push('\u{000c}');
                    }
                    Some((_, 'n')) => {
                        self.next_char();
                        acc.push('\n');
                    }
                    Some((_, 'r')) => {
                        self.next_char();
                        acc.push('\r');
                    }
                    Some((_, 't')) => {
                        self.next_char();
                        acc.push('\t');
                    }
                    Some((_, 'u')) => {
                        let start_pos = self.position.clone();
                        self.next_char();
                        acc.push(self.scan_escaped_unicode(&start_pos)?);
                    }
                    Some((_, ch)) => {
                        let mut s = String::from("\\");
                        s.push(ch);

                        return Err(Spanning::zero_width(
                            &self.position,
                            LexerError::UnknownEscapeSequence(s),
                        ));
                    }
                    None => {
                        return Err(Spanning::zero_width(
                            &self.position,
                            LexerError::UnterminatedString,
                        ));
                    }
                }
                if let Some((_, ch)) = self.peek_char() {
                    if ch == 'n' {}
                } else {
                    return Err(Spanning::zero_width(
                        &self.position,
                        LexerError::UnterminatedString,
                    ));
                }
            } else if ch == '\n' || ch == '\r' {
                return Err(Spanning::zero_width(
                    &self.position,
                    LexerError::UnterminatedString,
                ));
            } else if !is_source_char(ch) {
                return Err(Spanning::zero_width(
                    &self.position,
                    LexerError::UnknownCharacterInString(ch),
                ));
            } else {
                self.next_char();
                acc.push(ch);
            }
        }

        Err(Spanning::zero_width(
            &self.position,
            LexerError::UnterminatedString,
        ))
    }

    fn scan_escaped_unicode(
        &mut self,
        start_pos: &SourcePosition,
    ) -> Result<char, Spanning<LexerError>> {
        let (start_idx, _) = self.peek_char().ok_or(Spanning::zero_width(
            &self.position,
            LexerError::UnterminatedString,
        ))?;
        let mut end_idx = start_idx;
        let mut len = 0;

        for _ in 0..4 {
            let (idx, ch) = self.next_char().ok_or(Spanning::zero_width(
                &self.position,
                LexerError::UnterminatedString,
            ))?;

            if !ch.is_alphanumeric() {
                break;
            }

            end_idx = idx;
            len += 1;
        }

        let escape = &self.source[start_idx..end_idx + 1];

        if len != 4 {
            return Err(Spanning::zero_width(
                start_pos,
                LexerError::UnknownEscapeSequence("\\u".to_owned() + escape),
            ));
        }

        let code_point = u32::from_str_radix(escape, 16).map_err(|_| {
            Spanning::zero_width(
                start_pos,
                LexerError::UnknownEscapeSequence("\\u".to_owned() + escape),
            )
        })?;

        char::from_u32(code_point).ok_or_else(|| {
            Spanning::zero_width(
                start_pos,
                LexerError::UnknownEscapeSequence("\\u".to_owned() + escape),
            )
        })
    }

    fn scan_number(&mut self) -> LexerResult<'a> {
        let start_pos = self.position.clone();
        let int_part = self.scan_integer_part()?;
        let mut frac_part = None;
        let mut exp_part = None;

        if let Some((_, '.')) = self.peek_char() {
            self.next_char();

            frac_part = Some(self.scan_digits()?);
        }

        if let Some((_, ch)) = self.peek_char() {
            if ch == 'e' || ch == 'E' {
                self.next_char();

                let mut is_negative = false;

                if let Some((_, ch)) = self.peek_char() {
                    if ch == '-' {
                        self.next_char();
                        is_negative = true;
                    } else if ch == '+' {
                        self.next_char();
                    }
                }
                exp_part = Some(if is_negative { -1 } else { 1 } * self.scan_digits()?);
            }
        }

        let mantissa = frac_part
            .map(|f| f64::from(f))
            .map(|frac| {
                if frac > 0f64 {
                    frac / 10f64.powf(frac.log10().floor() + 1f64)
                } else {
                    0f64
                }
            })
            .map(|m| if int_part < 0 { -m } else { m });

        let exp = exp_part.map(|e| f64::from(e)).map(|e| 10f64.powf(e));

        Ok(Spanning::start_end(
            &start_pos,
            &self.position,
            match (mantissa, exp) {
                (None, None) => Token::Int(int_part),
                (None, Some(exp)) => Token::Float((f64::from(int_part)) * exp),
                (Some(mantissa), None) => Token::Float((f64::from(int_part)) + mantissa),
                (Some(mantissa), Some(exp)) => {
                    Token::Float(((f64::from(int_part)) + mantissa) * exp)
                }
            },
        ))
    }

    fn scan_integer_part(&mut self) -> Result<i32, Spanning<LexerError>> {
        let is_negative = {
            let (_, init_ch) = self.peek_char().ok_or(Spanning::zero_width(
                &self.position,
                LexerError::UnexpectedEndOfFile,
            ))?;

            if init_ch == '-' {
                self.next_char();
                true
            } else {
                false
            }
        };

        let (_, ch) = self.peek_char().ok_or(Spanning::zero_width(
            &self.position,
            LexerError::UnexpectedEndOfFile,
        ))?;

        if ch == '0' {
            self.next_char();

            match self.peek_char() {
                Some((_, '0')) => Err(Spanning::zero_width(
                    &self.position,
                    LexerError::UnexpectedCharacter(ch),
                )),
                _ => Ok(0),
            }
        } else {
            Ok(self.scan_digits()? * if is_negative { -1 } else { 1 })
        }
    }

    fn scan_digits(&mut self) -> Result<i32, Spanning<LexerError>> {
        let start_pos = self.position.clone();
        let (start_idx, ch) = self.peek_char().ok_or(Spanning::zero_width(
            &self.position,
            LexerError::UnexpectedEndOfFile,
        ))?;
        let mut end_idx = start_idx;

        if !ch.is_digit(10) {
            return Err(Spanning::zero_width(
                &self.position,
                LexerError::UnexpectedCharacter(ch),
            ));
        }

        while let Some((idx, ch)) = self.peek_char() {
            if !ch.is_digit(10) {
                break;
            } else {
                self.next_char();
                end_idx = idx;
            }
        }

        i32::from_str_radix(&self.source[start_idx..end_idx + 1], 10)
            .map_err(|_| Spanning::zero_width(&start_pos, LexerError::InvalidNumber))
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
            Some(ch) => if is_number_start(ch) {
                self.scan_number()
            } else if is_name_start(ch) {
                self.scan_name()
            } else {
                Err(Spanning::zero_width(
                    &self.position,
                    LexerError::UnknownCharacter(ch),
                ))
            },
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
            Token::Name(name) => write!(f, "{}", name),
            Token::Int(i) => write!(f, "{}", i),
            Token::Float(v) => write!(f, "{}", v),
            Token::String(ref s) => {
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
    c == '_' || (c >= 'A' && c <= 'Z') || (c >= 'a' && c <= 'z')
}

fn is_name_cont(c: char) -> bool {
    is_name_start(c) || (c >= '0' && c <= '9')
}

fn is_number_start(c: char) -> bool {
    c == '-' || (c >= '0' && c <= '9')
}

impl fmt::Display for LexerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            LexerError::UnknownCharacter(c) => write!(f, "Unknown character \"{}\"", c),
            LexerError::UnterminatedString => write!(f, "Unterminated string literal"),
            LexerError::UnknownCharacterInString(c) => {
                write!(f, "Unknown character \"{}\" in string literal", c)
            }
            LexerError::UnknownEscapeSequence(ref s) => {
                write!(f, "Unknown escape sequence \"{}\" in string", s)
            }
            LexerError::UnexpectedCharacter(c) => write!(f, "Unexpected character \"{}\"", c),
            LexerError::UnexpectedEndOfFile => write!(f, "Unexpected end of input"),
            LexerError::InvalidNumber => write!(f, "Invalid number literal"),
        }
    }
}
