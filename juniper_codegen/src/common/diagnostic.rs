use std::fmt;

use proc_macro2::Span;

pub(crate) use self::polyfill::{
    abort_if_dirty, emit_error, entry_point, entry_point_with_preserved_body, Diagnostic, ResultExt,
};

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
        Diagnostic::spanned(span, format!("{self} {}", msg.as_ref())).note(self.spec_link())
    }

    pub(crate) fn error(&self, err: &syn::Error) -> Diagnostic {
        Diagnostic::spanned(err.span(), format!("{self} {err}")).note(self.spec_link())
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

    use std::{
        cell::{Cell, RefCell},
        panic::{catch_unwind, resume_unwind, UnwindSafe},
    };

    use proc_macro2::{Span, TokenStream};
    use quote::{quote, quote_spanned, ToTokens};

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

        /// Aborts macro execution and display this [`Diagnostic`].
        pub(crate) fn abort(self) -> ! {
            self.emit();
            abort_now()
        }

        /// Display this [`Diagnostic`] while not aborting macro execution.
        pub(crate) fn emit(self) {
            check_correctness();
            emit_diagnostic(self);
        }
    }

    impl ToTokens for Diagnostic {
        fn to_tokens(&self, ts: &mut TokenStream) {
            use std::borrow::Cow;

            fn ensure_lf(buf: &mut String, s: &str) {
                if s.ends_with('\n') {
                    buf.push_str(s);
                } else {
                    buf.push_str(s);
                    buf.push('\n');
                }
            }

            fn diag_to_tokens(
                span_range: SpanRange,
                msg: &str,
                suggestions: &[String],
            ) -> TokenStream {
                let message = if suggestions.is_empty() {
                    Cow::Borrowed(msg)
                } else {
                    let mut message = String::new();
                    ensure_lf(&mut message, msg);
                    for note in suggestions {
                        message.push_str("· note: ");
                        ensure_lf(&mut message, note);
                    }
                    Cow::Owned(message)
                };

                let mut msg = proc_macro2::Literal::string(&message);
                msg.set_span(span_range.last);
                let group = quote_spanned!(span_range.last=> { #msg } );
                quote_spanned!(span_range.first=> compile_error!#group)
            }

            ts.extend(diag_to_tokens(
                self.span_range,
                &self.msg,
                self.suggestions.as_ref(),
            ));
        }
    }

    impl From<syn::Error> for Diagnostic {
        fn from(err: syn::Error) -> Self {
            use proc_macro2::{Delimiter, TokenTree};

            fn gut_error(ts: &mut impl Iterator<Item = TokenTree>) -> Option<(SpanRange, String)> {
                let first = ts.next()?.span(); // :
                assert_eq!(ts.next().unwrap().to_string(), ":");
                assert_eq!(ts.next().unwrap().to_string(), "core");
                assert_eq!(ts.next().unwrap().to_string(), ":");
                assert_eq!(ts.next().unwrap().to_string(), ":");
                assert_eq!(ts.next().unwrap().to_string(), "compile_error");
                assert_eq!(ts.next().unwrap().to_string(), "!");

                let lit = match ts.next().unwrap() {
                    TokenTree::Group(group) => {
                        // Currently `syn` builds `compile_error!` invocations
                        // exclusively in `ident{"..."}` (braced) form which is not
                        // followed by `;` (semicolon).
                        //
                        // But if it changes to `ident("...");` (parenthesized)
                        // or `ident["..."];` (bracketed) form,
                        // we will need to skip the `;` as well.
                        // Highly unlikely, but better safe than sorry.

                        if group.delimiter() == Delimiter::Parenthesis
                            || group.delimiter() == Delimiter::Bracket
                        {
                            ts.next().unwrap(); // ;
                        }

                        match group.stream().into_iter().next().unwrap() {
                            TokenTree::Literal(lit) => lit,
                            tt => unreachable!("Diagnostic::gut_error(): TokenTree::Group: {tt}"),
                        }
                    }
                    tt => unreachable!("Diagnostic::gut_error(): {tt}"),
                };

                let last = lit.span();
                let mut msg = lit.to_string();

                // "abc" => abc
                msg.pop();
                msg.remove(0);

                Some((SpanRange { first, last }, msg))
            }

            let mut ts = err.to_compile_error().into_iter();

            let (span_range, msg) = gut_error(&mut ts).unwrap();

            Self {
                span_range,
                msg,
                suggestions: vec![],
            }
        }
    }

    /// Emits a [`syn::Error`] while not aborting macro execution.
    pub(crate) fn emit_error(e: syn::Error) {
        Diagnostic::from(e).emit()
    }

    /// Range of [`Span`]s.
    #[derive(Clone, Copy, Debug)]
    struct SpanRange {
        first: Span,
        last: Span,
    }

    thread_local! {
        static ENTERED_ENTRY_POINT: Cell<usize> = Cell::new(0);
    }

    /// This is the entry point for a macro to support [`Diagnostic`]s.
    pub(crate) fn entry_point<F>(f: F) -> proc_macro::TokenStream
    where
        F: FnOnce() -> proc_macro::TokenStream + UnwindSafe,
    {
        entry_point_with_preserved_body(TokenStream::new(), f)
    }

    /// This is the entry point for an attribute macro to support [`Diagnostic`]s, while preserving
    /// the `body` input [`proc_macro::TokenStream`] on errors.
    pub(crate) fn entry_point_with_preserved_body<F>(
        body: impl Into<TokenStream>,
        f: F,
    ) -> proc_macro::TokenStream
    where
        F: FnOnce() -> proc_macro::TokenStream + UnwindSafe,
    {
        ENTERED_ENTRY_POINT.with(|flag| flag.set(flag.get() + 1));
        let caught = catch_unwind(f);
        let err_storage = ERR_STORAGE.with(|s| s.replace(Vec::new()));
        ENTERED_ENTRY_POINT.with(|flag| flag.set(flag.get() - 1));

        let gen_error = || {
            let body = body.into();

            quote! { #body #( #err_storage )* }
        };

        match caught {
            Ok(ts) => {
                if err_storage.is_empty() {
                    ts
                } else {
                    gen_error().into()
                }
            }

            Err(boxed) => match boxed.downcast_ref::<&str>() {
                Some(p) if *p == "diagnostic::polyfill::abort_now" => gen_error().into(),
                _ => resume_unwind(boxed),
            },
        }
    }

    fn check_correctness() {
        if ENTERED_ENTRY_POINT.get() == 0 {
            panic!(
                "`common::diagnostic` API cannot be used outside of `entry_point()` invocation, \
                 perhaps you forgot to invoke it your #[proc_macro] function",
            );
        }
    }

    thread_local! {
        static ERR_STORAGE: RefCell<Vec<Diagnostic>> = RefCell::new(Vec::new());
    }

    /// Emits the provided [`Diagnostic`], while not aborting macro execution.
    fn emit_diagnostic(diag: Diagnostic) {
        ERR_STORAGE.with(|s| s.borrow_mut().push(diag));
    }

    /// Aborts macro execution. if any [`Diagnostic`]s were emitted before.
    pub(crate) fn abort_if_dirty() {
        check_correctness();
        ERR_STORAGE.with(|s| {
            if !s.borrow().is_empty() {
                abort_now()
            }
        });
    }

    fn abort_now() -> ! {
        check_correctness();
        panic!("diagnostic::polyfill::abort_now")
    }

    /// Extension of `Result<T, Into<Diagnostic>>` with some handy shortcuts.
    pub(crate) trait ResultExt {
        type Ok;

        /// Behaves like [`Result::unwrap()`]: if `self` is [`Ok`] yield the contained value,
        /// otherwise abort macro execution.
        fn unwrap_or_abort(self) -> Self::Ok;

        /// Behaves like [`Result::expect()`]: if `self` is [`Ok`] yield the contained value,
        /// otherwise abort macro execution.
        ///
        /// If it aborts then resulting error message will be preceded with the provided `message`.
        fn expect_or_abort(self, message: &str) -> Self::Ok;
    }

    impl<T, E: Into<Diagnostic>> ResultExt for Result<T, E> {
        type Ok = T;

        fn unwrap_or_abort(self) -> T {
            self.unwrap_or_else(|e| e.into().abort())
        }

        fn expect_or_abort(self, message: &str) -> T {
            self.unwrap_or_else(|e| {
                let mut d = e.into();
                d.msg = format!("{message}: {}", d.msg);
                d.abort()
            })
        }
    }
}
