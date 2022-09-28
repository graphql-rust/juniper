use std::fmt;

use proc_macro2::Span;
use proc_macro_error::{Diagnostic, Level};

/// URL of the GraphQL specification (October 2021 Edition).
pub(crate) const SPEC_URL: &str = "https://spec.graphql.org/October2021";

pub(crate) enum Scope {
    EnumDerive,
    InputObjectDerive,
    InterfaceAttr,
    InterfaceDerive,
    ObjectAttr,
    ObjectDerive,
    ScalarAttr,
    ScalarDerive,
    ScalarValueDerive,
    UnionAttr,
    UnionDerive,
}

impl Scope {
    pub(crate) fn spec_section(&self) -> &str {
        match self {
            Self::EnumDerive => "#sec-Enums",
            Self::InputObjectDerive => "#sec-Input-Objects",
            Self::InterfaceAttr | Self::InterfaceDerive => "#sec-Interfaces",
            Self::ObjectAttr | Self::ObjectDerive => "#sec-Objects",
            Self::ScalarAttr | Self::ScalarDerive => "#sec-Scalars",
            Self::ScalarValueDerive => "#sec-Scalars.Built-in-Scalars",
            Self::UnionAttr | Self::UnionDerive => "#sec-Unions",
        }
    }
}

impl fmt::Display for Scope {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let name = match self {
            Self::EnumDerive => "enum",
            Self::InputObjectDerive => "input object",
            Self::InterfaceAttr | Self::InterfaceDerive => "interface",
            Self::ObjectAttr | Self::ObjectDerive => "object",
            Self::ScalarAttr | Self::ScalarDerive => "scalar",
            Self::ScalarValueDerive => "built-in scalars",
            Self::UnionAttr | Self::UnionDerive => "union",
        };
        write!(f, "GraphQL {name}")
    }
}

impl Scope {
    fn spec_link(&self) -> String {
        format!("{SPEC_URL}{}", self.spec_section())
    }

    pub(crate) fn custom<S: AsRef<str>>(&self, span: Span, msg: S) -> Diagnostic {
        Diagnostic::spanned(span, Level::Error, format!("{self} {}", msg.as_ref()))
            .note(self.spec_link())
    }

    pub(crate) fn error(&self, err: syn::Error) -> Diagnostic {
        Diagnostic::spanned(err.span(), Level::Error, format!("{self} {err}"))
            .note(self.spec_link())
    }

    pub(crate) fn emit_custom<S: AsRef<str>>(&self, span: Span, msg: S) {
        self.custom(span, msg).emit()
    }

    pub(crate) fn custom_error<S: AsRef<str>>(&self, span: Span, msg: S) -> syn::Error {
        syn::Error::new(span, format!("{self} {}", msg.as_ref()))
    }

    pub(crate) fn no_double_underscore(&self, field: Span) {
        Diagnostic::spanned(
            field,
            Level::Error,
            "All types and directives defined within a schema must not have a name which begins \
             with `__` (two underscores), as this is used exclusively by GraphQL’s introspection \
             system."
                .into(),
        )
        .note(format!("{SPEC_URL}#sec-Schema"))
        .emit();
    }
}
