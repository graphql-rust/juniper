//!

use crate::util::duplicate::Duplicate;
use proc_macro2::Span;
use proc_macro_error::{Diagnostic, Level};
use std::fmt;

/// URL of the GraphQL specification (June 2018 Edition).
pub const SPEC_URL: &'static str = "https://spec.graphql.org/June2018/";

#[allow(unused_variables)]
pub enum GraphQLScope {
    DeriveObject,
    DeriveInputObject,
    DeriveUnion,
    DeriveEnum,
    DeriveScalar,
    ImplUnion,
    ImplScalar,
    ImplObject,
}

impl GraphQLScope {
    pub fn spec_section(&self) -> &str {
        match self {
            GraphQLScope::DeriveObject | GraphQLScope::ImplObject => "#sec-Objects",
            GraphQLScope::DeriveInputObject => "#sec-Input-Objects",
            GraphQLScope::DeriveUnion | GraphQLScope::ImplUnion => "#sec-Unions",
            GraphQLScope::DeriveEnum => "#sec-Enums",
            GraphQLScope::DeriveScalar | GraphQLScope::ImplScalar => "#sec-Scalars",
        }
    }
}

impl fmt::Display for GraphQLScope {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let name = match self {
            GraphQLScope::DeriveObject | GraphQLScope::ImplObject => "object",
            GraphQLScope::DeriveInputObject => "input object",
            GraphQLScope::DeriveUnion | GraphQLScope::ImplUnion => "union",
            GraphQLScope::DeriveEnum => "enum",
            GraphQLScope::DeriveScalar | GraphQLScope::ImplScalar => "scalar",
        };

        write!(f, "GraphQL {}", name)
    }
}

#[allow(unused_variables)]
#[derive(Debug)]
pub enum UnsupportedAttribute {
    Skip,
    Interface,
    Scalar,
    Description,
    Deprecation,
    Default,
}

impl GraphQLScope {
    fn spec_link(&self) -> String {
        format!("{}{}", SPEC_URL, self.spec_section())
    }

    pub fn custom<S: AsRef<str>>(&self, span: Span, msg: S) {
        Diagnostic::spanned(span, Level::Error, format!("{} {}", self, msg.as_ref()))
            .note(self.spec_link())
            .emit();
    }

    pub fn custom_error<S: AsRef<str>>(&self, span: Span, msg: S) -> syn::Error {
        syn::Error::new(span, format!("{} {}", self, msg.as_ref()))
    }

    pub fn unsupported_attribute(&self, attribute: Span, kind: UnsupportedAttribute) {
        Diagnostic::spanned(
            attribute,
            Level::Error,
            format!("attribute `{:?}` can not be used at the top level of {}", kind, self),
        )
        .note("The macro is known to Juniper. However, not all valid #[graphql] attributes are available for each macro".to_string())
        .emit();
    }

    pub fn unsupported_attribute_within(&self, attribute: Span, kind: UnsupportedAttribute) {
        Diagnostic::spanned(
            attribute,
            Level::Error,
            format!("attribute `{:?}` can not be used inside of {}", kind, self),
        )
        .note("The macro is known to Juniper. However, not all valid #[graphql] attributes are available for each macro".to_string())
        .emit();
    }

    pub fn not_empty(&self, container: Span) {
        Diagnostic::spanned(
            container,
            Level::Error,
            format!("{} expects at least one field", self),
        )
        .note(self.spec_link())
        .emit();
    }

    pub fn duplicate<'a, T: syn::spanned::Spanned + 'a>(
        &self,
        duplicates: impl IntoIterator<Item = &'a Duplicate<T>>,
    ) {
        duplicates
            .into_iter()
            .for_each(|dup| {
                (&dup.spanned[1..])
                    .iter()
                    .for_each(|spanned| {
                        Diagnostic::spanned(
                            spanned.span(),
                            Level::Error,
                            format!(
                                "{} does not allow fields with the same name",
                                self
                            ),
                        )
                            .help(format!("There is at least one other field with the same name `{}`, possibly renamed via the #[graphql] attribute", dup.name))
                            .note(self.spec_link())
                            .emit();
                    });
            })
    }

    pub fn no_double_underscore(&self, field: Span) {
        Diagnostic::spanned(
            field,
            Level::Error,
            "All types and directives defined within a schema must not have a name which begins with `__` (two underscores), as this is used exclusively by GraphQL’s introspection system.".to_string(),
        )
            .note(format!("{}#sec-Schema", SPEC_URL))
            .emit();
    }
}
