//!

use crate::util::duplicate::Duplicate;
use proc_macro2::Span;
use proc_macro_error::{Diagnostic, Level};
use std::fmt;

/// URL of the GraphQL specification (June 2018 Edition).
pub const SPEC_URL: &str = "https://spec.graphql.org/June2018/";

#[allow(unused_variables)]
pub enum GraphQLScope {
    InterfaceAttr,
    UnionAttr,
    UnionDerive,
    DeriveObject,
    DeriveInputObject,
    DeriveEnum,
    DeriveScalar,
    ImplScalar,
    ImplObject,
}

impl GraphQLScope {
    pub fn spec_section(&self) -> &str {
        match self {
            Self::InterfaceAttr => "#sec-Interfaces",
            Self::UnionAttr | Self::UnionDerive => "#sec-Unions",
            Self::DeriveObject | Self::ImplObject => "#sec-Objects",
            Self::DeriveInputObject => "#sec-Input-Objects",
            Self::DeriveEnum => "#sec-Enums",
            Self::DeriveScalar | Self::ImplScalar => "#sec-Scalars",
        }
    }
}

impl fmt::Display for GraphQLScope {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let name = match self {
            Self::InterfaceAttr => "interface",
            Self::UnionAttr | Self::UnionDerive => "union",
            Self::DeriveObject | Self::ImplObject => "object",
            Self::DeriveInputObject => "input object",
            Self::DeriveEnum => "enum",
            Self::DeriveScalar | Self::ImplScalar => "scalar",
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
    Deprecation,
    Default,
}

impl GraphQLScope {
    fn spec_link(&self) -> String {
        format!("{}{}", SPEC_URL, self.spec_section())
    }

    pub fn custom<S: AsRef<str>>(&self, span: Span, msg: S) -> Diagnostic {
        Diagnostic::spanned(span, Level::Error, format!("{} {}", self, msg.as_ref()))
            .note(self.spec_link())
    }

    pub fn emit_custom<S: AsRef<str>>(&self, span: Span, msg: S) {
        self.custom(span, msg).emit()
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
            "All types and directives defined within a schema must not have a name which begins \
             with `__` (two underscores), as this is used exclusively by GraphQL’s introspection \
             system."
                .into(),
        )
        .note(format!("{}#sec-Schema", SPEC_URL))
        .emit();
    }
}
