use std::borrow::Borrow;

use arcstr::ArcStr;
use derive_more::with_trait::{Display, Error};

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Name(ArcStr);

impl Name {
    /// Creates a new [`Name`] out of the provided `input` string, if it [`is_valid`].
    ///
    /// [`is_valid`]: Name::is_valid
    pub fn new<S>(input: S) -> Result<Self, NameParseError>
    where
        S: AsRef<str> + Into<ArcStr>,
    {
        if Self::is_valid(input.as_ref()) {
            Ok(Name(input.into()))
        } else {
            Err(NameParseError(arcstr::format!(
                "`Name` must match /^[_a-zA-Z][_a-zA-Z0-9]*$/ but \"{}\" does not",
                input.as_ref(),
            )))
        }
    }

    /// Validates the provided `input` string to represent a valid [`Name`].
    #[must_use]
    pub fn is_valid(input: &str) -> bool {
        for (i, c) in input.chars().enumerate() {
            let is_valid = c.is_ascii_alphabetic() || c == '_' || (i > 0 && c.is_ascii_digit());
            if !is_valid {
                return false;
            }
        }
        !input.is_empty()
    }
}

impl Borrow<ArcStr> for Name {
    fn borrow(&self) -> &ArcStr {
        &self.0
    }
}

impl Borrow<str> for Name {
    fn borrow(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Display, Eq, Error, Ord, PartialEq, PartialOrd)]
pub struct NameParseError(#[error(not(source))] ArcStr);

#[test]
fn test_name_is_valid() {
    assert!(Name::is_valid("Foo"));
    assert!(Name::is_valid("foo42"));
    assert!(Name::is_valid("_Foo"));
    assert!(Name::is_valid("_Foo42"));
    assert!(Name::is_valid("_foo42"));
    assert!(Name::is_valid("_42Foo"));

    assert!(!Name::is_valid("42_Foo"));
    assert!(!Name::is_valid("Foo-42"));
    assert!(!Name::is_valid("Foo???"));
}
