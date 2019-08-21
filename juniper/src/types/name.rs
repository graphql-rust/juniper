use std::{
    borrow::Borrow,
    error::Error,
    fmt::{Display, Formatter, Result as FmtResult},
    str::FromStr,
};

// Helper functions until the corresponding AsciiExt methods
// stabilise (https://github.com/rust-lang/rust/issues/39658).

fn is_ascii_alphabetic(c: char) -> bool {
    c >= 'a' && c <= 'z' || c >= 'A' && c <= 'Z'
}

fn is_ascii_digit(c: char) -> bool {
    c >= '0' && c <= '9'
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Name(String);

impl Name {
    pub fn is_valid(input: &str) -> bool {
        for (i, c) in input.chars().enumerate() {
            let is_valid = is_ascii_alphabetic(c) || c == '_' || (i > 0 && is_ascii_digit(c));
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

impl FromStr for Name {
    type Err = NameParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if Name::is_valid(s) {
            Ok(Name(s.to_string()))
        } else {
            Err(NameParseError(format!(
                "Names must match /^[_a-zA-Z][_a-zA-Z0-9]*$/ but \"{}\" does not",
                s
            )))
        }
    }
}

impl Borrow<String> for Name {
    fn borrow(&self) -> &String {
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
