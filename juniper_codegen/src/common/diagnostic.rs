use std::fmt;

use proc_macro2::Span;

use self::polyfill::Diagnostic;

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
        Diagnostic::spanned(span, format!("{self} {}", msg.as_ref()))
            .note(self.spec_link())
    }

    pub(crate) fn error(&self, err: &syn::Error) -> Diagnostic {
        Diagnostic::spanned(err.span(), format!("{self} {err}"))
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
            "All types and directives defined within a schema must not have a name which begins \
             with `__` (two underscores), as this is used exclusively by GraphQL’s introspection \
             system.",
        )
        .note(format!("{SPEC_URL}#sec-Schema"))
        .emit();
    }
}

mod polyfill {
    //! Simplified version of [`proc_macro_error`] machinery for this crate purposes.
    //!
    //! [`proc_macro_error`]: https://docs.rs/proc-macro-error/1

    use proc_macro2::Span;

    /// Representation of a single diagnostic message.
    #[derive(Debug)]
    pub(crate) struct Diagnostic {
        span_range: SpanRange,
        msg: String,
        suggestions: Vec<String>,
    }

    impl Diagnostic {
        /// Create a new [`Diagnostic`] message that points to the provided [`Span`].
        pub(crate) fn spanned(span: Span, message: impl Into<String>) -> Self {
            Self {
                span_range: SpanRange {
                    first: span,
                    last: span,
                },
                msg: message.into(),
                suggestions: vec![],
            }
        }

        /// Attaches a note to the main message of this [`Diagnostic`].
        pub(crate) fn note(mut self, msg: impl Into<String>) -> Self {
            self.suggestions.push(msg.into());
            self
        }

        /// Display this [`Diagnostic`] while not aborting macro execution.
        pub fn emit(self) {
            check_correctness();
            crate::imp::emit_diagnostic(self);
        }

    }

    /// Range of [`Span`]s.
    #[derive(Clone, Copy, Debug)]
    struct SpanRange {
        first: Span,
        last: Span,
    }
}


