use std::fmt;

/// A reference to a line and column in an input source file
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Copy)]
pub struct SourcePosition {
    index: usize,
    line: usize,
    col: usize,
}

/// Range of characters in the input source, starting at the character pointed by the `start` field
/// and ending just before the `end` marker.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Span {
    /// Start position of this [`Span`].
    pub start: SourcePosition,

    /// End position of this [`Span`].
    ///
    /// > __NOTE__: This points to the first source position __after__ this [`Span`].
    pub end: SourcePosition,
}

impl Span {
    #[doc(hidden)]
    #[inline]
    pub fn zero_width(pos: SourcePosition) -> Self {
        Self {
            start: pos,
            end: pos,
        }
    }

    #[doc(hidden)]
    #[inline]
    pub fn single_width(pos: SourcePosition) -> Self {
        let mut end = pos;
        end.advance_col();

        Self { start: pos, end }
    }

    #[doc(hidden)]
    #[inline]
    pub fn unlocated() -> Self {
        Self {
            start: SourcePosition::new_origin(),
            end: SourcePosition::new_origin(),
        }
    }
}

/// Data structure used to wrap items into a [`Span`].
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Spanning<T, Sp = Span> {
    /// Wrapped item.
    pub item: T,

    /// [`Span`] of the wrapped item.
    pub span: Sp,
}

impl<T> Spanning<T, Span> {
    #[doc(hidden)]
    pub fn new(span: Span, item: T) -> Self {
        Self { item, span }
    }

    #[doc(hidden)]
    pub fn zero_width(&pos: &SourcePosition, item: T) -> Spanning<T> {
        Self::new(Span::zero_width(pos), item)
    }

    #[doc(hidden)]
    pub fn single_width(&pos: &SourcePosition, item: T) -> Spanning<T> {
        Self::new(Span::single_width(pos), item)
    }

    #[doc(hidden)]
    pub fn start_end(&start: &SourcePosition, &end: &SourcePosition, item: T) -> Spanning<T> {
        Self::new(Span { start, end }, item)
    }

    #[doc(hidden)]
    #[allow(clippy::self_named_constructors)]
    pub fn spanning(v: Vec<Spanning<T>>) -> Option<Spanning<Vec<Spanning<T>>>> {
        if let (Some(start), Some(end)) = (v.first().map(|s| s.span), v.last().map(|s| s.span)) {
            Some(Spanning::new(
                Span {
                    start: start.start,
                    end: end.end,
                },
                v,
            ))
        } else {
            None
        }
    }

    #[doc(hidden)]
    pub fn unlocated(item: T) -> Spanning<T> {
        Self::new(Span::unlocated(), item)
    }

    /// Returns start position of the item.
    #[inline]
    pub fn start(&self) -> SourcePosition {
        self.span.start
    }

    /// Returns end position of the item.
    ///
    /// > __NOTE__: This points to the first source position __after__ the item.
    #[inline]
    pub fn end(&self) -> SourcePosition {
        self.span.end
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

    /// Converts into a [`Spanning`] containing a borrowed item and a borrowed [`Span`].
    pub(crate) fn as_ref(&self) -> Spanning<&'_ T, &'_ Span> {
        Spanning {
            item: &self.item,
            span: &self.span,
        }
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
