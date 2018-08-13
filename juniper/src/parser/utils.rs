use std::fmt;
use std::hash::{Hash, Hasher};

/// A reference to a line and column in an input source file
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Copy)]
pub struct SourcePosition {
    index: usize,
    line: usize,
    col: usize,
}

/// Data structure used to wrap items with start and end markers in the input source
///
/// A "span" is a range of characters in the input source, starting at the
/// character pointed by the `start` field and ending just before the `end`
/// marker.
#[derive(Debug)]
pub struct Spanning<T: fmt::Debug> {
    /// The wrapped item
    pub item: T,

    /// Start position of the item
    pub start: SourcePosition,

    /// End position of the item
    ///
    /// This points to the first source position _after_ the wrapped item.
    pub end: SourcePosition,
}

impl<T: fmt::Debug> Spanning<T> {
    #[doc(hidden)]
    pub fn zero_width(pos: &SourcePosition, item: T) -> Spanning<T> {
        Spanning {
            item: item,
            start: pos.clone(),
            end: pos.clone(),
        }
    }

    #[doc(hidden)]
    pub fn single_width(pos: &SourcePosition, item: T) -> Spanning<T> {
        let mut end = pos.clone();
        end.advance_col();

        Spanning {
            item: item,
            start: pos.clone(),
            end: end,
        }
    }

    #[doc(hidden)]
    pub fn start_end(start: &SourcePosition, end: &SourcePosition, item: T) -> Spanning<T> {
        Spanning {
            item: item,
            start: start.clone(),
            end: end.clone(),
        }
    }

    #[doc(hidden)]
    pub fn spanning(v: Vec<Spanning<T>>) -> Option<Spanning<Vec<Spanning<T>>>> {
        if let (Some(start), Some(end)) = (
            v.first().map(|s| s.start.clone()),
            v.last().map(|s| s.end.clone()),
        ) {
            Some(Spanning {
                item: v,
                start: start,
                end: end,
            })
        } else {
            None
        }
    }

    #[doc(hidden)]
    pub fn unlocated(item: T) -> Spanning<T> {
        Spanning {
            item: item,
            start: SourcePosition::new_origin(),
            end: SourcePosition::new_origin(),
        }
    }

    /// Modify the contents of the spanned item
    pub fn map<O: fmt::Debug, F: Fn(T) -> O>(self, f: F) -> Spanning<O> {
        Spanning {
            item: f(self.item),
            start: self.start.clone(),
            end: self.end.clone(),
        }
    }
}

impl<T> Clone for Spanning<T>
where
    T: Clone + fmt::Debug,
{
    fn clone(&self) -> Self {
        Spanning {
            start: self.start.clone(),
            end: self.end.clone(),
            item: self.item.clone(),
        }
    }
}

impl<T> PartialEq for Spanning<T>
where
    T: PartialEq + fmt::Debug,
{
    fn eq(&self, other: &Self) -> bool {
        self.start == other.start && self.end == other.end && self.item == other.item
    }
}

impl<T> Eq for Spanning<T>
where
    T: Eq + fmt::Debug,
{
}

impl<T> Hash for Spanning<T>
where
    T: Hash + fmt::Debug,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.start.hash(state);
        self.end.hash(state);
        self.item.hash(state);
    }
}

impl SourcePosition {
    #[doc(hidden)]
    pub fn new(index: usize, line: usize, col: usize) -> SourcePosition {
        assert!(index >= line + col);

        SourcePosition {
            index: index,
            line: line,
            col: col,
        }
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
