use std::{
    borrow::Borrow,
    error::Error,
    fmt::{Display, Formatter, Result as FmtResult},
};

use arcstr::ArcStr;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Name(ArcStr);

impl Name {
    pub fn new(input: ArcStr) -> Result<Self, NameParseError> {
        if Self::is_valid(&input) {
            Ok(Name(input))
        } else {
            Err(NameParseError(format!(
                "Names must match /^[_a-zA-Z][_a-zA-Z0-9]*$/ but \"{input}\" does not",
            )))
        }
    }

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

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct NameParseError(String);

impl Display for NameParseError {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        self.0.fmt(f)
    }
}

impl Error for NameParseError {
    fn description(&self) -> &str {
        &self.0
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
