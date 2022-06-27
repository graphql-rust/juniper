#![allow(clippy::single_match)]

pub mod span_container;

use std::{convert::TryFrom, str::FromStr};

use proc_macro_error::abort;
use span_container::SpanContainer;
use syn::{
    parse::{Parse, ParseStream},
    spanned::Spanned,
    Attribute, Lit, Meta, MetaList, MetaNameValue, NestedMeta,
};

/// Compares a path to a one-segment string value,
/// return true if equal.
pub fn path_eq_single(path: &syn::Path, value: &str) -> bool {
    path.segments.len() == 1 && path.segments[0].ident == value
}

#[derive(Debug)]
pub struct DeprecationAttr {
    pub reason: Option<String>,
}

/// Filters given `attrs` to contain attributes only with the given `name`.
pub fn filter_attrs<'a>(
    name: &'a str,
    attrs: &'a [Attribute],
) -> impl Iterator<Item = &'a Attribute> + 'a {
    attrs
        .iter()
        .filter(move |attr| path_eq_single(&attr.path, name))
}

pub fn get_deprecated(attrs: &[Attribute]) -> Option<SpanContainer<DeprecationAttr>> {
    attrs
        .iter()
        .filter_map(|attr| match attr.parse_meta() {
            Ok(Meta::List(ref list)) if list.path.is_ident("deprecated") => {
                let val = get_deprecated_meta_list(list);
                Some(SpanContainer::new(list.path.span(), None, val))
            }
            Ok(Meta::Path(ref path)) if path.is_ident("deprecated") => Some(SpanContainer::new(
                path.span(),
                None,
                DeprecationAttr { reason: None },
            )),
            _ => None,
        })
        .next()
}

fn get_deprecated_meta_list(list: &MetaList) -> DeprecationAttr {
    for meta in &list.nested {
        if let NestedMeta::Meta(Meta::NameValue(ref nv)) = *meta {
            if nv.path.is_ident("note") {
                match nv.lit {
                    Lit::Str(ref strlit) => {
                        return DeprecationAttr {
                            reason: Some(strlit.value()),
                        };
                    }
                    _ => abort!(syn::Error::new(
                        nv.lit.span(),
                        "only strings are allowed for deprecation",
                    )),
                }
            } else {
                abort!(syn::Error::new(
                    nv.path.span(),
                    "unrecognized setting on #[deprecated(..)] attribute",
                ));
            }
        }
    }
    DeprecationAttr { reason: None }
}

// Gets doc comment.
pub fn get_doc_comment(attrs: &[Attribute]) -> Option<SpanContainer<String>> {
    if let Some(items) = get_doc_attr(attrs) {
        if let Some(doc_strings) = get_doc_strings(&items) {
            return Some(doc_strings.map(|strings| join_doc_strings(&strings)));
        }
    }
    None
}

// Concatenates doc strings into one string.
fn join_doc_strings(docs: &[String]) -> String {
    // Note: this is guaranteed since this function is only called
    // from get_doc_strings().
    debug_assert!(!docs.is_empty());

    let last_index = docs.len() - 1;
    docs.iter()
        .map(|s| s.as_str().trim_end())
        // Trim leading space.
        .map(|s| s.strip_prefix(' ').unwrap_or(s))
        // Add newline, exept when string ends in a continuation backslash or is the last line.
        .enumerate()
        .fold(String::new(), |mut buffer, (index, s)| {
            if index == last_index {
                buffer.push_str(s);
            } else if s.ends_with('\\') {
                buffer.push_str(s.trim_end_matches('\\'));
                buffer.push(' ');
            } else {
                buffer.push_str(s);
                buffer.push('\n');
            }
            buffer
        })
}

// Gets doc strings from doc comment attributes.
fn get_doc_strings(items: &[MetaNameValue]) -> Option<SpanContainer<Vec<String>>> {
    let mut span = None;
    let comments = items
        .iter()
        .filter_map(|item| {
            if item.path.is_ident("doc") {
                match item.lit {
                    Lit::Str(ref strlit) => {
                        if span.is_none() {
                            span = Some(strlit.span());
                        }
                        Some(strlit.value())
                    }
                    _ => abort!(syn::Error::new(
                        item.lit.span(),
                        "doc attributes only have string literal"
                    )),
                }
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    span.map(|span| SpanContainer::new(span, None, comments))
}

// Gets doc comment attributes.
fn get_doc_attr(attrs: &[Attribute]) -> Option<Vec<MetaNameValue>> {
    let mut docs = Vec::new();
    for attr in attrs {
        match attr.parse_meta() {
            Ok(Meta::NameValue(ref nv)) if nv.path.is_ident("doc") => docs.push(nv.clone()),
            _ => {}
        }
    }
    if !docs.is_empty() {
        return Some(docs);
    }
    None
}

// Note: duplicated from juniper crate!
#[doc(hidden)]
pub fn to_camel_case(s: &str) -> String {
    let mut dest = String::new();

    // Handle `_` and `__` to be more friendly with the `_var` convention for unused variables, and
    // GraphQL introspection identifiers.
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

pub(crate) fn to_upper_snake_case(s: &str) -> String {
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

/// The different possible ways to change case of fields in a struct, or variants in an enum.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RenameRule {
    /// Don't apply a default rename rule.
    None,
    /// Rename to "camelCase" style.
    CamelCase,
    /// Rename to "SCREAMING_SNAKE_CASE" style
    ScreamingSnakeCase,
}

impl RenameRule {
    pub fn apply(&self, field: &str) -> String {
        match self {
            Self::None => field.to_owned(),
            Self::CamelCase => to_camel_case(field),
            Self::ScreamingSnakeCase => to_upper_snake_case(field),
        }
    }
}

impl FromStr for RenameRule {
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

impl TryFrom<syn::LitStr> for RenameRule {
    type Error = syn::Error;

    fn try_from(lit: syn::LitStr) -> syn::Result<Self> {
        Self::from_str(&lit.value()).map_err(|_| syn::Error::new(lit.span(), "unknown rename rule"))
    }
}

impl Parse for RenameRule {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        Self::try_from(input.parse::<syn::LitStr>()?)
    }
}

#[cfg(test)]
mod test {
    use proc_macro2::Span;
    use syn::{Ident, LitStr};

    use super::*;

    fn is_valid_name(field_name: &str) -> bool {
        let mut chars = field_name.chars();

        match chars.next() {
            // first char can't be a digit
            Some(c) if c.is_ascii_alphabetic() || c == '_' => (),
            // can't be an empty string or any other character
            _ => return false,
        };

        chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
    }

    fn strs_to_strings(source: Vec<&str>) -> Vec<String> {
        source
            .iter()
            .map(|x| (*x).to_string())
            .collect::<Vec<String>>()
    }

    fn litstr(s: &str) -> Lit {
        Lit::Str(LitStr::new(s, Span::call_site()))
    }

    fn ident(s: &str) -> Ident {
        quote::format_ident!("{}", s)
    }

    mod test_get_doc_strings {
        use super::*;

        #[test]
        fn test_single() {
            let result = get_doc_strings(&[MetaNameValue {
                path: ident("doc").into(),
                eq_token: Default::default(),
                lit: litstr("foo"),
            }]);
            assert_eq!(
                &result.unwrap(),
                Some(&strs_to_strings(vec!["foo"])).unwrap()
            );
        }

        #[test]
        fn test_many() {
            let result = get_doc_strings(&[
                MetaNameValue {
                    path: ident("doc").into(),
                    eq_token: Default::default(),
                    lit: litstr("foo"),
                },
                MetaNameValue {
                    path: ident("doc").into(),
                    eq_token: Default::default(),
                    lit: litstr("\n"),
                },
                MetaNameValue {
                    path: ident("doc").into(),
                    eq_token: Default::default(),
                    lit: litstr("bar"),
                },
            ]);
            assert_eq!(
                &result.unwrap(),
                Some(&strs_to_strings(vec!["foo", "\n", "bar"])).unwrap()
            );
        }

        #[test]
        fn test_not_doc() {
            let result = get_doc_strings(&[MetaNameValue {
                path: ident("blah").into(),
                eq_token: Default::default(),
                lit: litstr("foo"),
            }]);
            assert_eq!(&result, &None);
        }
    }

    mod test_join_doc_strings {
        use super::*;

        #[test]
        fn test_single() {
            let result = join_doc_strings(&strs_to_strings(vec!["foo"]));
            assert_eq!(&result, "foo");
        }
        #[test]
        fn test_multiple() {
            let result = join_doc_strings(&strs_to_strings(vec!["foo", "bar"]));
            assert_eq!(&result, "foo\nbar");
        }

        #[test]
        fn test_trims_spaces() {
            let result = join_doc_strings(&strs_to_strings(vec![" foo ", "bar ", " baz"]));
            assert_eq!(&result, "foo\nbar\nbaz");
        }

        #[test]
        fn test_empty() {
            let result = join_doc_strings(&strs_to_strings(vec!["foo", "", "bar"]));
            assert_eq!(&result, "foo\n\nbar");
        }

        #[test]
        fn test_newline_spaces() {
            let result = join_doc_strings(&strs_to_strings(vec!["foo ", "", " bar"]));
            assert_eq!(&result, "foo\n\nbar");
        }

        #[test]
        fn test_continuation_backslash() {
            let result = join_doc_strings(&strs_to_strings(vec!["foo\\", "x\\", "y", "bar"]));
            assert_eq!(&result, "foo x y\nbar");
        }
    }

    #[test]
    fn test_to_camel_case() {
        assert_eq!(&to_camel_case("test")[..], "test");
        assert_eq!(&to_camel_case("_test")[..], "test");
        assert_eq!(&to_camel_case("__test")[..], "__test");
        assert_eq!(&to_camel_case("first_second")[..], "firstSecond");
        assert_eq!(&to_camel_case("first_")[..], "first");
        assert_eq!(&to_camel_case("a_b_c")[..], "aBC");
        assert_eq!(&to_camel_case("a_bc")[..], "aBc");
        assert_eq!(&to_camel_case("a_b")[..], "aB");
        assert_eq!(&to_camel_case("a")[..], "a");
        assert_eq!(&to_camel_case("")[..], "");
    }

    #[test]
    fn test_to_upper_snake_case() {
        assert_eq!(to_upper_snake_case("abc"), "ABC");
        assert_eq!(to_upper_snake_case("a_bc"), "A_BC");
        assert_eq!(to_upper_snake_case("ABC"), "ABC");
        assert_eq!(to_upper_snake_case("A_BC"), "A_BC");
        assert_eq!(to_upper_snake_case("SomeInput"), "SOME_INPUT");
        assert_eq!(to_upper_snake_case("someInput"), "SOME_INPUT");
        assert_eq!(to_upper_snake_case("someINpuT"), "SOME_INPU_T");
        assert_eq!(to_upper_snake_case("some_INpuT"), "SOME_INPU_T");
    }

    #[test]
    fn test_is_valid_name() {
        assert_eq!(is_valid_name("yesItIs"), true);
        assert_eq!(is_valid_name("NoitIsnt"), true);
        assert_eq!(is_valid_name("iso6301"), true);
        assert_eq!(is_valid_name("thisIsATest"), true);
        assert_eq!(is_valid_name("i6Op"), true);
        assert_eq!(is_valid_name("i!"), false);
        assert_eq!(is_valid_name(""), false);
        assert_eq!(is_valid_name("aTest"), true);
        assert_eq!(is_valid_name("__Atest90"), true);
    }
}
