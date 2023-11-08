use std::fmt;

/// A reference to a line and column in an input source file
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Copy)]
pub struct SourcePosition {
    index: usize,
    line: usize,
    col: usize,
}

/// A "span" is a range of characters in the input source, starting at the
/// character pointed by the `start` field and ending just before the `end`
/// marker.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
pub struct Span {
    /// Start position of the span
    pub start: SourcePosition,

    /// End position of the span
    ///
    /// This points to the first source position _after_ the span.
    pub end: SourcePosition,
}

impl Span {
    #[doc(hidden)]
    pub fn new(start: &SourcePosition, end: &SourcePosition) -> Span {
        Self {
            start: *start,
            end: *end,
        }
    }

    #[doc(hidden)]
    pub fn zero_width(pos: &SourcePosition) -> Span {
        Self::new(pos, pos)
    }

    #[doc(hidden)]
    pub fn single_width(pos: &SourcePosition) -> Span {
        let mut end = *pos;
        end.advance_col();

        Self { start: *pos, end }
    }

    #[doc(hidden)]
    pub fn unlocated() -> Span {
        Self {
            start: SourcePosition::new_origin(),
            end: SourcePosition::new_origin(),
        }
    }
}

/// Data structure used to wrap items with start and end markers in the input source
#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
pub struct Spanning<T> {
    /// The wrapped item
    pub item: T,

    /// The span
    pub span: Span,
}

impl<T> Spanning<T> {
    #[doc(hidden)]
    pub fn new(span: Span, item: T) -> Spanning<T> {
        Self { item, span }
    }

    #[doc(hidden)]
    pub fn zero_width(pos: &SourcePosition, item: T) -> Spanning<T> {
        Self::new(Span::zero_width(pos), item)
    }

    #[doc(hidden)]
    pub fn single_width(pos: &SourcePosition, item: T) -> Spanning<T> {
        Self::new(Span::single_width(pos), item)
    }

    #[doc(hidden)]
    pub fn start_end(start: &SourcePosition, end: &SourcePosition, item: T) -> Spanning<T> {
        Self::new(Span::new(start, end), item)
    }

    #[doc(hidden)]
    #[allow(clippy::self_named_constructors)]
    pub fn spanning(v: Vec<Spanning<T>>) -> Option<Spanning<Vec<Spanning<T>>>> {
        if let (Some(start), Some(end)) = (v.first().map(|s| s.span), v.last().map(|s| s.span)) {
            Some(Spanning::new(Span::new(&start.start, &end.end), v))
        } else {
            None
        }
    }

    #[doc(hidden)]
    pub fn unlocated(item: T) -> Spanning<T> {
        Self::new(Span::unlocated(), item)
    }

    #[doc(hidden)]
    pub fn start(&self) -> &SourcePosition {
        &self.span.start
    }

    #[doc(hidden)]
    pub fn end(&self) -> &SourcePosition {
        &self.span.end
    }

    /// Modify the contents of the spanned item.
    pub fn map<O, F: Fn(T) -> O>(self, f: F) -> Spanning<O> {
        Spanning::new(self.span, f(self.item))
    }

    /// Modifies the contents of the spanned item in case `f` returns [`Some`],
    /// or returns [`None`] otherwise.
    pub fn and_then<O, F: Fn(T) -> Option<O>>(self, f: F) -> Option<Spanning<O>> {
        f(self.item).map(|item| Spanning::new(self.span, item))
    }
}

impl<T: fmt::Display> fmt::Display for Spanning<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}. At {}", self.item, self.span.start)
    }
}

impl<T: std::error::Error> std::error::Error for Spanning<T> {}

impl SourcePosition {
    #[doc(hidden)]
    pub fn new(index: usize, line: usize, col: usize) -> SourcePosition {
        assert!(index >= line + col);

        SourcePosition { index, line, col }
    }

    #[doc(hidden)]
    pub fn new_origin() -> SourcePosition {
        SourcePosition {
            index: 0,
            line: 0,
            col: 0,
        }
    }

    #[doc(hidden)]
    pub fn advance_col(&mut self) {
        self.index += 1;
        self.col += 1;
    }

    #[doc(hidden)]
    pub fn advance_line(&mut self) {
        self.index += 1;
        self.line += 1;
        self.col = 0;
    }

    /// The index of the character in the input source
    ///
    /// Zero-based index. Take a substring of the original source starting at
    /// this index to access the item pointed to by this `SourcePosition`.
    pub fn index(&self) -> usize {
        self.index
    }

    /// The line of the character in the input source
    ///
    /// Zero-based index: the first line is line zero.
    pub fn line(&self) -> usize {
        self.line
    }

    /// The column of the character in the input source
    ///
    /// Zero-based index: the first column is column zero.
    pub fn column(&self) -> usize {
        self.col
    }
}

impl fmt::Display for SourcePosition {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.col)
    }
}
