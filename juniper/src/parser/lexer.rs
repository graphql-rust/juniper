use std::{char, ops::Deref, str::CharIndices};

use derive_more::with_trait::{Display, Error};

use crate::parser::{SourcePosition, Spanning};

#[doc(hidden)]
#[derive(Debug)]
pub struct Lexer<'a> {
    iterator: itertools::PeekNth<CharIndices<'a>>,
    source: &'a str,
    length: usize,
    position: SourcePosition,
    has_reached_eof: bool,
}

/// Representation of a raw unparsed scalar value literal.
///
/// This is only used for tagging how the lexer has interpreted a value literal
#[expect(missing_docs, reason = "self-explanatory")]
#[derive(Clone, Copy, Debug, Display, Eq, PartialEq)]
pub enum ScalarToken<'a> {
    String(StringLiteral<'a>),
    Float(&'a str),
    Int(&'a str),
}

/// Representation of a raw unparsed [String Value] literal (with quotes included).
///
/// [String Value]: https://spec.graphql.org/October2021#sec-String-Value
#[derive(Clone, Copy, Debug, Display, Eq, PartialEq)]
pub enum StringLiteral<'a> {
    /// [Quoted][0] literal (denoted by single quotes `"`).
    ///
    /// [0]: https://spec.graphql.org/October2021#StringCharacter
    Quoted(&'a str),

    /// [Block][0] literal (denoted by triple quotes `"""`).
    ///
    /// [0]: https://spec.graphql.org/October2021#BlockStringCharacter
    Block(&'a str),
}

impl Deref for StringLiteral<'_> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Quoted(s) => s,
            Self::Block(s) => s,
        }
    }
}

/// A single token in the input source
#[expect(missing_docs, reason = "self-explanatory")]
#[derive(Clone, Copy, Debug, Display, Eq, PartialEq)]
pub enum Token<'a> {
    Name(&'a str),
    Scalar(ScalarToken<'a>),
    #[display("!")]
    ExclamationMark,
    #[display("$")]
    Dollar,
    #[display("(")]
    ParenOpen,
    #[display(")")]
    ParenClose,
    #[display("[")]
    BracketOpen,
    #[display("]")]
    BracketClose,
    #[display("{{")]
    CurlyOpen,
    #[display("}}")]
    CurlyClose,
    #[display("...")]
    Ellipsis,
    #[display(":")]
    Colon,
    #[display("=")]
    Equals,
    #[display("@")]
    At,
    #[display("|")]
    Pipe,
    #[display("End of file")]
    EndOfFile,
}

/// Error when tokenizing the input source
#[derive(Clone, Debug, Display, Eq, Error, PartialEq)]
pub enum LexerError {
    /// An unknown character was found
    ///
    /// Unknown characters are characters that do not occur anywhere in the
    /// GraphQL language, such as `?` or `%`.
    #[display("Unknown character \"{_0}\"")]
    UnknownCharacter(#[error(not(source))] char),

    /// An unexpected character was found
    ///
    /// Unexpected characters are characters that _do_ exist in the GraphQL
    /// language, but is not expected at the current position in the document.
    #[display("Unexpected character \"{_0}\"")]
    UnexpectedCharacter(#[error(not(source))] char),

    /// An unterminated string literal was found
    ///
    /// Apart from forgetting the ending `"`, terminating a string within a
    /// Unicode escape sequence or having a line break in the string also
    /// causes this error.
    #[display("Unterminated string literal")]
    UnterminatedString,

    /// An unterminated block string literal was found.
    #[display("Unterminated block string literal")]
    UnterminatedBlockString,

    /// An unknown escape sequence in a string literal was found
    ///
    /// Only a limited set of escape sequences are supported, this is emitted
    /// when e.g. `"\l"` is parsed.
    #[display("Unknown escape sequence \"{_0}\" in string")]
    UnknownEscapeSequence(#[error(not(source))] String),

    /// The input source was unexpectedly terminated
    ///
    /// Emitted when the current token requires a succeeding character, but
    /// the source has reached EOF. Emitted when scanning e.g. `"1."`.
    #[display("Unexpected end of input")]
    UnexpectedEndOfFile,

    /// An invalid number literal was found
    #[display("Invalid number literal")]
    InvalidNumber,
}

pub type LexerResult<'a> = Result<Spanning<Token<'a>>, Spanning<LexerError>>;

impl<'a> Lexer<'a> {
    #[doc(hidden)]
    pub fn new(source: &'a str) -> Lexer<'a> {
        Lexer {
            iterator: itertools::peek_nth(source.char_indices()),
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

    /// Advances this [`Lexer`] over any [ignored] character until a non-[ignored] is met.
    ///
    /// [ignored]: https://spec.graphql.org/September2025#Ignored
    fn scan_over_whitespace(&mut self) {
        while let Some((_, ch)) = self.peek_char() {
            // Ignored ::
            //     UnicodeBOM
            //     WhiteSpace
            //     LineTerminator
            //     Comment
            //     Comma
            match ch {
                // UnicodeBOM ::
                //     Byte Order Mark (U+FEFF)
                // Whitespace ::
                //     Horizontal Tab (U+0009)
                //     Space (U+0020)
                // LineTerminator ::
                //     New Line (U+000A)
                //     Carriage Return (U+000D) [lookahead != New Line (U+000A)]
                //     Carriage Return (U+000D) New Line (U+000A)
                // Comma ::
                //     ,
                '\u{FEFF}' | '\t' | ' ' | '\n' | '\r' | ',' => _ = self.next_char(),
                // Comment ::
                //     #CommentChar[list][opt] [lookahead != CommentChar]
                // CommentChar ::
                //     SourceCharacter but not LineTerminator
                '#' => {
                    _ = self.next_char();
                    while let Some((_, ch)) = self.peek_char() {
                        _ = self.next_char();
                        match ch {
                            '\r' if matches!(self.peek_char(), Some((_, '\n'))) => {
                                _ = self.next_char();
                                break;
                            }
                            '\n' | '\r' => break,
                            // Continue scanning `Comment`.
                            _ => {}
                        }
                    }
                }
                // Any other character is not `Ignored`.
                _ => break,
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

    /// Scans a [string] by this [`Lexer`], but not a [block string].
    ///
    /// [string]: https://spec.graphql.org/September2025#StringValue
    /// [block string]: https://spec.graphql.org/September2025#BlockString
    fn scan_string(&mut self) -> LexerResult<'a> {
        // StringValue ::
        //     "" [lookahead != "]
        //     "StringCharacter[list]"
        //     BlockString

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
            // StringCharacter ::
            //     SourceCharacter but not " or \ or LineTerminator
            //     \uEscapedUnicode
            //     \EscapedCharacter
            match ch {
                // EscapedCharacter :: one of
                //     " \ / b f n r t
                '"' | '\\' | '/' | 'b' | 'f' | 'n' | 'r' | 't' if escaped => {
                    escaped = false;
                }
                // EscapedUnicode ::
                //     {HexDigit[list]}
                //     HexDigit HexDigit HexDigit HexDigit
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
                        Token::Scalar(ScalarToken::String(StringLiteral::Quoted(
                            &self.source[start_idx..=idx],
                        ))),
                    ));
                }
                '\n' | '\r' => {
                    return Err(Spanning::zero_width(
                        &old_pos,
                        LexerError::UnterminatedString,
                    ));
                }
                // Any other valid Unicode scalar value is a `SourceCharacter`:
                // https://spec.graphql.org/September2025#SourceCharacter
                _ => {}
            }
            old_pos = self.position;
        }

        Err(Spanning::zero_width(
            &self.position,
            LexerError::UnterminatedString,
        ))
    }

    /// Scans a [block string] by this [`Lexer`].
    ///
    /// [block string]: https://spec.graphql.org/September2025#BlockString
    fn scan_block_string(&mut self) -> LexerResult<'a> {
        // BlockString ::
        //     """BlockStringCharacter[list][opt]"""

        let start_pos = self.position;
        let (start_idx, mut start_ch) = self
            .next_char()
            .ok_or_else(|| Spanning::zero_width(&self.position, LexerError::UnexpectedEndOfFile))?;
        if start_ch != '"' {
            return Err(Spanning::zero_width(
                &self.position,
                LexerError::UnterminatedString,
            ));
        }
        for _ in 0..2 {
            (_, start_ch) = self.next_char().ok_or_else(|| {
                Spanning::zero_width(&self.position, LexerError::UnexpectedEndOfFile)
            })?;
            if start_ch != '"' {
                return Err(Spanning::zero_width(
                    &self.position,
                    LexerError::UnexpectedCharacter(start_ch),
                ));
            }
        }
        let (mut quotes, mut escaped) = (0, false);
        while let Some((idx, ch)) = self.next_char() {
            // BlockStringCharacter ::
            //     SourceCharacter but not """ or \"""
            //     \"""
            match ch {
                '\\' => (quotes, escaped) = (0, true),
                '"' if escaped => (quotes, escaped) = (0, false),
                '"' if quotes < 2 => quotes += 1,
                '"' if quotes == 2 => {
                    return Ok(Spanning::start_end(
                        &start_pos,
                        &self.position,
                        Token::Scalar(ScalarToken::String(StringLiteral::Block(
                            &self.source[start_idx..=idx],
                        ))),
                    ));
                }
                _ => (quotes, escaped) = (0, false),
            }
        }

        Err(Spanning::zero_width(
            &self.position,
            LexerError::UnterminatedBlockString,
        ))
    }

    /// Scans an [escaped unicode] character by this [`Lexer`].
    ///
    /// [escaped unicode]: https://spec.graphql.org/September2025#EscapedUnicode
    fn scan_escaped_unicode(
        &mut self,
        start_pos: &SourcePosition,
    ) -> Result<(), Spanning<LexerError>> {
        // EscapedUnicode ::
        //     {HexDigit[list]}
        //     HexDigit HexDigit HexDigit HexDigit

        let (start_idx, mut curr_ch) = self
            .peek_char()
            .ok_or_else(|| Spanning::zero_width(&self.position, LexerError::UnterminatedString))?;
        let mut end_idx = start_idx;
        let mut len = 0;

        let is_variable_width = curr_ch == '{';
        if is_variable_width {
            _ = self.next_char();
            loop {
                let (idx, ch) = self.next_char().ok_or_else(|| {
                    Spanning::zero_width(&self.position, LexerError::UnterminatedString)
                })?;
                curr_ch = ch;
                end_idx = idx;
                len += 1;
                if !curr_ch.is_alphanumeric() {
                    break;
                }
            }
        } else {
            for _ in 0..4 {
                let (idx, ch) = self.next_char().ok_or_else(|| {
                    Spanning::zero_width(&self.position, LexerError::UnterminatedString)
                })?;
                curr_ch = ch;
                if !curr_ch.is_alphanumeric() {
                    break;
                }
                end_idx = idx;
                len += 1;
            }
        }

        // Make sure we are on a valid char boundary.
        let escape = self
            .source
            .get(start_idx..=end_idx)
            .ok_or_else(|| Spanning::zero_width(&self.position, LexerError::UnterminatedString))?;

        let code_point = if is_variable_width {
            if curr_ch != '}' {
                return Err(Spanning::zero_width(
                    start_pos,
                    LexerError::UnknownEscapeSequence(format!(
                        r"\u{}",
                        &escape[..escape.len() - 1],
                    )),
                ));
            }
            u32::from_str_radix(&escape[1..escape.len() - 1], 16)
        } else {
            if len != 4 {
                return Err(Spanning::zero_width(
                    start_pos,
                    LexerError::UnknownEscapeSequence(format!(r"\u{escape}")),
                ));
            }
            u32::from_str_radix(escape, 16)
        }
        .map_err(|_| {
            Spanning::zero_width(
                start_pos,
                LexerError::UnknownEscapeSequence(format!(r"\u{escape}")),
            )
        })?;

        char::from_u32(code_point)
            .ok_or_else(|| {
                Spanning::zero_width(
                    start_pos,
                    LexerError::UnknownEscapeSequence(format!(r"\u{escape}")),
                )
            })
            .map(drop)
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
            Some('"') => {
                if self.iterator.peek_nth(1).map(|&(_, ch)| ch) == Some('"')
                    && self.iterator.peek_nth(2).map(|&(_, ch)| ch) == Some('"')
                {
                    self.scan_block_string()
                } else {
                    self.scan_string()
                }
            }
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

fn is_name_start(c: char) -> bool {
    c == '_' || c.is_ascii_alphabetic()
}

fn is_name_cont(c: char) -> bool {
    is_name_start(c) || c.is_ascii_digit()
}

fn is_number_start(c: char) -> bool {
    c == '-' || c.is_ascii_digit()
}

#[cfg(test)]
mod test {
    use crate::parser::{
        Lexer, LexerError, ScalarToken, SourcePosition, Spanning,
        StringLiteral::{Block, Quoted},
        Token,
    };

    #[track_caller]
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
                Some(Err(e)) => panic!("error in input stream: {e} for {s:#?}"),
                None => panic!("EOF before `Token::EndOfFile` in {s:#?}"),
            }
        }

        tokens
    }

    #[track_caller]
    fn tokenize_single(s: &str) -> Spanning<Token<'_>> {
        let mut tokens = tokenize_to_vec(s);

        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[1].item, Token::EndOfFile);

        tokens.remove(0)
    }

    #[track_caller]
    fn tokenize_error(s: &str) -> Spanning<LexerError> {
        let mut lexer = Lexer::new(s);

        loop {
            match lexer.next() {
                Some(Ok(t)) => {
                    if t.item == Token::EndOfFile {
                        panic!("lexer did not return error for {s:#?}");
                    }
                }
                Some(Err(e)) => {
                    return e;
                }
                None => panic!("lexer did not return error for {s:#?}"),
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
                LexerError::UnknownCharacter('\u{0007}'),
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

            "#,
            )
            .next(),
            Some(Err(Spanning::zero_width(
                &SourcePosition::new(14, 2, 12),
                LexerError::UnknownCharacter('?'),
            ))),
        );
    }

    #[test]
    fn strings() {
        assert_eq!(
            tokenize_single(r#""simple""#),
            Spanning::start_end(
                &SourcePosition::new(0, 0, 0),
                &SourcePosition::new(8, 0, 8),
                Token::Scalar(ScalarToken::String(Quoted(r#""simple""#))),
            ),
        );

        assert_eq!(
            tokenize_single(r#"" white space ""#),
            Spanning::start_end(
                &SourcePosition::new(0, 0, 0),
                &SourcePosition::new(15, 0, 15),
                Token::Scalar(ScalarToken::String(Quoted(r#"" white space ""#))),
            ),
        );

        assert_eq!(
            tokenize_single(r#""quote \"""#),
            Spanning::start_end(
                &SourcePosition::new(0, 0, 0),
                &SourcePosition::new(10, 0, 10),
                Token::Scalar(ScalarToken::String(Quoted(r#""quote \"""#))),
            ),
        );

        assert_eq!(
            tokenize_single(r#""escaped \n\r\b\t\f""#),
            Spanning::start_end(
                &SourcePosition::new(0, 0, 0),
                &SourcePosition::new(20, 0, 20),
                Token::Scalar(ScalarToken::String(Quoted(r#""escaped \n\r\b\t\f""#))),
            ),
        );

        assert_eq!(
            tokenize_single(r#""slashes \\ \/""#),
            Spanning::start_end(
                &SourcePosition::new(0, 0, 0),
                &SourcePosition::new(15, 0, 15),
                Token::Scalar(ScalarToken::String(Quoted(r#""slashes \\ \/""#))),
            ),
        );

        assert_eq!(
            tokenize_single(r#""unicode \u1234\u5678\u90AB\uCDEF""#),
            Spanning::start_end(
                &SourcePosition::new(0, 0, 0),
                &SourcePosition::new(34, 0, 34),
                Token::Scalar(ScalarToken::String(Quoted(
                    r#""unicode \u1234\u5678\u90AB\uCDEF""#,
                ))),
            ),
        );

        assert_eq!(
            tokenize_single(r#""variable-width unicode \u{1234}\u{5678}\u{90AB}\u{1F4A9}""#),
            Spanning::start_end(
                &SourcePosition::new(0, 0, 0),
                &SourcePosition::new(58, 0, 58),
                Token::Scalar(ScalarToken::String(Quoted(
                    r#""variable-width unicode \u{1234}\u{5678}\u{90AB}\u{1F4A9}""#,
                ))),
            ),
        );

        assert_eq!(
            tokenize_single("\"contains unescaped \u{0007} control char\""),
            Spanning::start_end(
                &SourcePosition::new(0, 0, 0),
                &SourcePosition::new(35, 0, 35),
                Token::Scalar(ScalarToken::String(Quoted(
                    "\"contains unescaped \u{0007} control char\"",
                ))),
            ),
        );

        assert_eq!(
            tokenize_single("\"null-byte is not \u{0000} end of file\""),
            Spanning::start_end(
                &SourcePosition::new(0, 0, 0),
                &SourcePosition::new(32, 0, 32),
                Token::Scalar(ScalarToken::String(Quoted(
                    "\"null-byte is not \u{0000} end of file\"",
                ))),
            ),
        );
    }

    #[test]
    fn string_errors() {
        assert_eq!(
            tokenize_error(r#"""#),
            Spanning::zero_width(
                &SourcePosition::new(1, 0, 1),
                LexerError::UnterminatedString,
            ),
        );

        assert_eq!(
            tokenize_error(r#""no end quote"#),
            Spanning::zero_width(
                &SourcePosition::new(13, 0, 13),
                LexerError::UnterminatedString,
            ),
        );

        assert_eq!(
            tokenize_error("\"multi\nline\""),
            Spanning::zero_width(
                &SourcePosition::new(6, 0, 6),
                LexerError::UnterminatedString,
            ),
        );

        assert_eq!(
            tokenize_error("\"multi\rline\""),
            Spanning::zero_width(
                &SourcePosition::new(6, 0, 6),
                LexerError::UnterminatedString,
            ),
        );

        assert_eq!(
            tokenize_error(r#""bad \z esc""#),
            Spanning::zero_width(
                &SourcePosition::new(6, 0, 6),
                LexerError::UnknownEscapeSequence(r"\z".into()),
            ),
        );

        assert_eq!(
            tokenize_error(r#""bad \x esc""#),
            Spanning::zero_width(
                &SourcePosition::new(6, 0, 6),
                LexerError::UnknownEscapeSequence(r"\x".into()),
            ),
        );

        assert_eq!(
            tokenize_error(r#""bad \u1 esc""#),
            Spanning::zero_width(
                &SourcePosition::new(6, 0, 6),
                LexerError::UnknownEscapeSequence(r"\u1".into()),
            ),
        );

        assert_eq!(
            tokenize_error(r#""bad \u0XX1 esc""#),
            Spanning::zero_width(
                &SourcePosition::new(6, 0, 6),
                LexerError::UnknownEscapeSequence(r"\u0XX1".into()),
            ),
        );

        assert_eq!(
            tokenize_error(r#""bad \uXXXX esc""#),
            Spanning::zero_width(
                &SourcePosition::new(6, 0, 6),
                LexerError::UnknownEscapeSequence(r"\uXXXX".into()),
            ),
        );

        assert_eq!(
            tokenize_error(r#""bad \uFXXX esc""#),
            Spanning::zero_width(
                &SourcePosition::new(6, 0, 6),
                LexerError::UnknownEscapeSequence(r"\uFXXX".into()),
            ),
        );

        assert_eq!(
            tokenize_error(r#""bad \uXXXF esc""#),
            Spanning::zero_width(
                &SourcePosition::new(6, 0, 6),
                LexerError::UnknownEscapeSequence(r"\uXXXF".into()),
            ),
        );

        assert_eq!(
            tokenize_error(r#""bad \u{110000} esc""#),
            Spanning::zero_width(
                &SourcePosition::new(6, 0, 6),
                LexerError::UnknownEscapeSequence(r"\u{110000}".into()),
            ),
        );

        assert_eq!(
            tokenize_error(r#""bad \u{DEAD} esc""#),
            Spanning::zero_width(
                &SourcePosition::new(6, 0, 6),
                LexerError::UnknownEscapeSequence(r"\u{DEAD}".into()),
            ),
        );

        assert_eq!(
            tokenize_error(r#""bad \u{DEA esc""#),
            Spanning::zero_width(
                &SourcePosition::new(6, 0, 6),
                LexerError::UnknownEscapeSequence(r"\u{DEA".into()),
            ),
        );

        assert_eq!(
            tokenize_error(r#""unterminated in string \""#),
            Spanning::zero_width(
                &SourcePosition::new(26, 0, 26),
                LexerError::UnterminatedString,
            ),
        );

        assert_eq!(
            tokenize_error(r#""unterminated \"#),
            Spanning::zero_width(
                &SourcePosition::new(15, 0, 15),
                LexerError::UnterminatedString,
            ),
        );

        // Found by fuzzing.
        assert_eq!(
            tokenize_error(r#""\uɠ^A"#),
            Spanning::zero_width(
                &SourcePosition::new(5, 0, 5),
                LexerError::UnterminatedString,
            ),
        );
    }

    #[test]
    fn block_strings() {
        assert_eq!(
            tokenize_single(r#""""""""#),
            Spanning::start_end(
                &SourcePosition::new(0, 0, 0),
                &SourcePosition::new(6, 0, 6),
                Token::Scalar(ScalarToken::String(Block(r#""""""""#))),
            ),
        );
        assert_eq!(
            tokenize_single(r#""""simple""""#),
            Spanning::start_end(
                &SourcePosition::new(0, 0, 0),
                &SourcePosition::new(12, 0, 12),
                Token::Scalar(ScalarToken::String(Block(r#""""simple""""#))),
            ),
        );
        assert_eq!(
            tokenize_single(r#"""" white space """"#),
            Spanning::start_end(
                &SourcePosition::new(0, 0, 0),
                &SourcePosition::new(19, 0, 19),
                Token::Scalar(ScalarToken::String(Block(r#"""" white space """"#))),
            ),
        );
        assert_eq!(
            tokenize_single(r#""""contains " quote""""#),
            Spanning::start_end(
                &SourcePosition::new(0, 0, 0),
                &SourcePosition::new(22, 0, 22),
                Token::Scalar(ScalarToken::String(Block(r#""""contains " quote""""#))),
            ),
        );
        assert_eq!(
            tokenize_single(r#""""contains \""" triple quote""""#),
            Spanning::start_end(
                &SourcePosition::new(0, 0, 0),
                &SourcePosition::new(32, 0, 32),
                Token::Scalar(ScalarToken::String(Block(
                    r#""""contains \""" triple quote""""#
                ))),
            ),
        );
        assert_eq!(
            tokenize_single(r#""""contains \"" double quote""""#),
            Spanning::start_end(
                &SourcePosition::new(0, 0, 0),
                &SourcePosition::new(31, 0, 31),
                Token::Scalar(ScalarToken::String(Block(
                    r#""""contains \"" double quote""""#
                ))),
            ),
        );
        assert_eq!(
            tokenize_single(r#""""contains \\""" triple quote""""#),
            Spanning::start_end(
                &SourcePosition::new(0, 0, 0),
                &SourcePosition::new(33, 0, 33),
                Token::Scalar(ScalarToken::String(Block(
                    r#""""contains \\""" triple quote""""#
                ))),
            ),
        );
        assert_eq!(
            tokenize_single(r#""""\"""quote" """"#),
            Spanning::start_end(
                &SourcePosition::new(0, 0, 0),
                &SourcePosition::new(17, 0, 17),
                Token::Scalar(ScalarToken::String(Block(r#""""\"""quote" """"#))),
            ),
        );
        assert_eq!(
            tokenize_single(r#""""multi\nline""""#),
            Spanning::start_end(
                &SourcePosition::new(0, 0, 0),
                &SourcePosition::new(17, 0, 17),
                Token::Scalar(ScalarToken::String(Block(r#""""multi\nline""""#))),
            ),
        );
        assert_eq!(
            tokenize_single(r#""""multi\rline\r\nnormalized""""#),
            Spanning::start_end(
                &SourcePosition::new(0, 0, 0),
                &SourcePosition::new(31, 0, 31),
                Token::Scalar(ScalarToken::String(Block(
                    r#""""multi\rline\r\nnormalized""""#
                ))),
            ),
        );
        assert_eq!(
            tokenize_single(r#""""unescaped \\n\\r\\b\\t\\f\\u1234""""#),
            Spanning::start_end(
                &SourcePosition::new(0, 0, 0),
                &SourcePosition::new(38, 0, 38),
                Token::Scalar(ScalarToken::String(Block(
                    r#""""unescaped \\n\\r\\b\\t\\f\\u1234""""#
                ))),
            ),
        );
        assert_eq!(
            tokenize_single(r#""""unescaped unicode outside BMP \u{1f600}""""#),
            Spanning::start_end(
                &SourcePosition::new(0, 0, 0),
                &SourcePosition::new(45, 0, 45),
                Token::Scalar(ScalarToken::String(Block(
                    r#""""unescaped unicode outside BMP \u{1f600}""""#,
                ))),
            ),
        );
        assert_eq!(
            tokenize_single(r#""""slashes \\\\ \\/""""#),
            Spanning::start_end(
                &SourcePosition::new(0, 0, 0),
                &SourcePosition::new(22, 0, 22),
                Token::Scalar(ScalarToken::String(Block(r#""""slashes \\\\ \\/""""#))),
            ),
        );
        assert_eq!(
            tokenize_single(
                r#""""
        
        spans
          multiple
            lines

        """"#,
            ),
            Spanning::start_end(
                &SourcePosition::new(0, 0, 0),
                &SourcePosition::new(76, 6, 11),
                Token::Scalar(ScalarToken::String(Block(
                    r#""""
        
        spans
          multiple
            lines

        """"#,
                ))),
            ),
        );
    }

    #[test]
    fn block_string_errors() {
        assert_eq!(
            tokenize_error(r#""""""#),
            Spanning::zero_width(
                &SourcePosition::new(4, 0, 4),
                LexerError::UnterminatedBlockString,
            ),
        );
        assert_eq!(
            tokenize_error(r#"""""""#),
            Spanning::zero_width(
                &SourcePosition::new(5, 0, 5),
                LexerError::UnterminatedBlockString,
            ),
        );
        assert_eq!(
            tokenize_error(r#""""no end quote"#),
            Spanning::zero_width(
                &SourcePosition::new(15, 0, 15),
                LexerError::UnterminatedBlockString,
            ),
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
                Token::Scalar(ScalarToken::Float(actual)) => assert_eq!(actual, expected),
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
                Token::Scalar(ScalarToken::Int("-4")),
            )
        );

        assert_eq!(
            tokenize_single("9"),
            Spanning::start_end(
                &SourcePosition::new(0, 0, 0),
                &SourcePosition::new(1, 0, 1),
                Token::Scalar(ScalarToken::Int("9")),
            )
        );

        assert_eq!(
            tokenize_single("0"),
            Spanning::start_end(
                &SourcePosition::new(0, 0, 0),
                &SourcePosition::new(1, 0, 1),
                Token::Scalar(ScalarToken::Int("0")),
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
                LexerError::UnexpectedCharacter('0'),
            )
        );

        assert_eq!(
            tokenize_error("+1"),
            Spanning::zero_width(
                &SourcePosition::new(0, 0, 0),
                LexerError::UnknownCharacter('+'),
            )
        );

        assert_eq!(
            tokenize_error("1."),
            Spanning::zero_width(
                &SourcePosition::new(2, 0, 2),
                LexerError::UnexpectedEndOfFile,
            )
        );

        assert_eq!(
            tokenize_error(".123"),
            Spanning::zero_width(
                &SourcePosition::new(0, 0, 0),
                LexerError::UnexpectedCharacter('.'),
            )
        );

        assert_eq!(
            tokenize_error("1.A"),
            Spanning::zero_width(
                &SourcePosition::new(2, 0, 2),
                LexerError::UnexpectedCharacter('A'),
            )
        );

        assert_eq!(
            tokenize_error("-A"),
            Spanning::zero_width(
                &SourcePosition::new(1, 0, 1),
                LexerError::UnexpectedCharacter('A'),
            )
        );

        assert_eq!(
            tokenize_error("1.0e"),
            Spanning::zero_width(
                &SourcePosition::new(4, 0, 4),
                LexerError::UnexpectedEndOfFile,
            )
        );

        assert_eq!(
            tokenize_error("1.0eA"),
            Spanning::zero_width(
                &SourcePosition::new(4, 0, 4),
                LexerError::UnexpectedCharacter('A'),
            )
        );
    }

    #[test]
    fn punctuation() {
        assert_eq!(
            tokenize_single("!"),
            Spanning::single_width(&SourcePosition::new(0, 0, 0), Token::ExclamationMark),
        );

        assert_eq!(
            tokenize_single("$"),
            Spanning::single_width(&SourcePosition::new(0, 0, 0), Token::Dollar),
        );

        assert_eq!(
            tokenize_single("("),
            Spanning::single_width(&SourcePosition::new(0, 0, 0), Token::ParenOpen),
        );

        assert_eq!(
            tokenize_single(")"),
            Spanning::single_width(&SourcePosition::new(0, 0, 0), Token::ParenClose),
        );

        assert_eq!(
            tokenize_single("..."),
            Spanning::start_end(
                &SourcePosition::new(0, 0, 0),
                &SourcePosition::new(3, 0, 3),
                Token::Ellipsis,
            )
        );

        assert_eq!(
            tokenize_single(":"),
            Spanning::single_width(&SourcePosition::new(0, 0, 0), Token::Colon),
        );

        assert_eq!(
            tokenize_single("="),
            Spanning::single_width(&SourcePosition::new(0, 0, 0), Token::Equals),
        );

        assert_eq!(
            tokenize_single("@"),
            Spanning::single_width(&SourcePosition::new(0, 0, 0), Token::At),
        );

        assert_eq!(
            tokenize_single("["),
            Spanning::single_width(&SourcePosition::new(0, 0, 0), Token::BracketOpen),
        );

        assert_eq!(
            tokenize_single("]"),
            Spanning::single_width(&SourcePosition::new(0, 0, 0), Token::BracketClose),
        );

        assert_eq!(
            tokenize_single("{"),
            Spanning::single_width(&SourcePosition::new(0, 0, 0), Token::CurlyOpen),
        );

        assert_eq!(
            tokenize_single("}"),
            Spanning::single_width(&SourcePosition::new(0, 0, 0), Token::CurlyClose),
        );

        assert_eq!(
            tokenize_single("|"),
            Spanning::single_width(&SourcePosition::new(0, 0, 0), Token::Pipe),
        );
    }

    #[test]
    fn punctuation_error() {
        assert_eq!(
            tokenize_error(".."),
            Spanning::zero_width(
                &SourcePosition::new(2, 0, 2),
                LexerError::UnexpectedEndOfFile,
            )
        );

        assert_eq!(
            tokenize_error("?"),
            Spanning::zero_width(
                &SourcePosition::new(0, 0, 0),
                LexerError::UnknownCharacter('?'),
            )
        );

        assert_eq!(
            tokenize_error("\u{203b}"),
            Spanning::zero_width(
                &SourcePosition::new(0, 0, 0),
                LexerError::UnknownCharacter('\u{203b}'),
            )
        );

        assert_eq!(
            tokenize_error("\u{200b}"),
            Spanning::zero_width(
                &SourcePosition::new(0, 0, 0),
                LexerError::UnknownCharacter('\u{200b}'),
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
                Token::Scalar(ScalarToken::String(Quoted(r#""some string""#))),
                r#""some string""#,
            ),
            (
                Token::Scalar(ScalarToken::String(Quoted(
                    r#""string with \\ escape and \" quote""#,
                ))),
                r#""string with \\ escape and \" quote""#,
            ),
            (
                Token::Scalar(ScalarToken::String(Block(
                    r#""""string with \\ escape and \" quote""""#,
                ))),
                r#""""string with \\ escape and \" quote""""#,
            ),
            (
                Token::Scalar(ScalarToken::String(Block(
                    r#""""block string with \\ escape and \" quote""""#,
                ))),
                r#""""block string with \\ escape and \" quote""""#,
            ),
            (
                Token::Scalar(ScalarToken::String(Block(
                    r#""""block
                    multiline
                    string"""#,
                ))),
                r#""""block
                    multiline
                    string"""#,
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
}
