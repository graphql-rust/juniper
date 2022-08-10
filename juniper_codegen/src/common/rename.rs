//! Common functions, definitions and extensions for parsing and code generation
//! of `#[graphql(rename_all = ...)]` attribute.

use std::str::FromStr;

use syn::parse::{Parse, ParseStream};

/// Possible ways to rename all [GraphQL fields][1] or [GrqphQL enum values][2].
///
/// [1]: https://spec.graphql.org/October2021#sec-Language.Fields
/// [2]: https://spec.graphql.org/October2021#sec-Enum-Value
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Policy {
    /// Do nothing, and use the default conventions renaming.
    None,

    /// Rename in `camelCase` style.
    CamelCase,

    /// Rename in `SCREAMING_SNAKE_CASE` style.
    ScreamingSnakeCase,
}

impl Policy {
    /// Applies this [`Policy`] to the given `name`.
    pub(crate) fn apply(&self, name: &str) -> String {
        match self {
            Self::None => name.into(),
            Self::CamelCase => to_camel_case(name),
            Self::ScreamingSnakeCase => to_upper_snake_case(name),
        }
    }
}

impl FromStr for Policy {
    type Err = ();

    fn from_str(rule: &str) -> Result<Self, Self::Err> {
        match rule {
            "none" => Ok(Self::None),
            "camelCase" => Ok(Self::CamelCase),
            "SCREAMING_SNAKE_CASE" => Ok(Self::ScreamingSnakeCase),
            _ => Err(()),
        }
    }
}

impl TryFrom<syn::LitStr> for Policy {
    type Error = syn::Error;

    fn try_from(lit: syn::LitStr) -> syn::Result<Self> {
        Self::from_str(&lit.value())
            .map_err(|_| syn::Error::new(lit.span(), "unknown renaming policy"))
    }
}

impl Parse for Policy {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        Self::try_from(input.parse::<syn::LitStr>()?)
    }
}

// NOTE: duplicated from juniper crate!
fn to_camel_case(s: &str) -> String {
    let mut dest = String::new();

    // Handle `_` and `__` to be more friendly with the `_var` convention for
    // unused variables, and GraphQL introspection identifiers.
    let s_iter = if let Some(s) = s.strip_prefix("__") {
        dest.push_str("__");
        s
    } else {
        s.strip_prefix('_').unwrap_or(s)
    }
    .split('_')
    .enumerate();

    for (i, part) in s_iter {
        if i > 0 && part.len() == 1 {
            dest.push_str(&part.to_uppercase());
        } else if i > 0 && part.len() > 1 {
            let first = part
                .chars()
                .next()
                .unwrap()
                .to_uppercase()
                .collect::<String>();
            let second = &part[1..];

            dest.push_str(&first);
            dest.push_str(second);
        } else if i == 0 {
            dest.push_str(part);
        }
    }

    dest
}

fn to_upper_snake_case(s: &str) -> String {
    let mut last_lower = false;
    let mut upper = String::new();
    for c in s.chars() {
        if c == '_' {
            last_lower = false;
        } else if c.is_lowercase() {
            last_lower = true;
        } else if c.is_uppercase() {
            if last_lower {
                upper.push('_');
            }
            last_lower = false;
        }

        for u in c.to_uppercase() {
            upper.push(u);
        }
    }
    upper
}

#[cfg(test)]
mod to_camel_case_tests {
    use super::to_camel_case;

    #[test]
    fn converts_correctly() {
        for (input, expected) in [
            ("test", "test"),
            ("_test", "test"),
            ("__test", "__test"),
            ("first_second", "firstSecond"),
            ("first_", "first"),
            ("a_b_c", "aBC"),
            ("a_bc", "aBc"),
            ("a_b", "aB"),
            ("a", "a"),
            ("", ""),
        ] {
            assert_eq!(to_camel_case(input), expected);
        }
    }
}

#[cfg(test)]
mod to_upper_snake_case_tests {
    use super::to_upper_snake_case;

    #[test]
    fn converts_correctly() {
        for (input, expected) in [
            ("abc", "ABC"),
            ("a_bc", "A_BC"),
            ("ABC", "ABC"),
            ("A_BC", "A_BC"),
            ("SomeInput", "SOME_INPUT"),
            ("someInput", "SOME_INPUT"),
            ("someINpuT", "SOME_INPU_T"),
            ("some_INpuT", "SOME_INPU_T"),
        ] {
            assert_eq!(to_upper_snake_case(input), expected);
        }
    }
}
