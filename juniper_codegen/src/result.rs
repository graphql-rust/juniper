//!

use crate::util::duplicate::Duplicate;
use proc_macro2::{Span, TokenStream};
use proc_macro_error::{Diagnostic, Level};
use std::fmt;

pub const GRAPHQL_SPECIFICATION: &'static str = "https://spec.graphql.org/June2018/";

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
    pub fn specification_section(&self) -> &str {
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
    Context,
    Description,
}

impl GraphQLScope {
    fn specification_link(&self) -> String {
        format!("{}{}", GRAPHQL_SPECIFICATION, self.specification_section())
    }

    pub fn custom<S: AsRef<str>>(&self, span: Span, msg: S) {
        Diagnostic::spanned(span, Level::Error, format!("{} {}", self, msg.as_ref()))
            .note(self.specification_link())
            .emit();
    }

    pub fn unknown_attribute(&self, attribute: Span, value: String) -> TokenStream {
        syn::Error::new(
            attribute,
            format!("attribute `{}` was not recognized by #[graphql]", value),
        )
        .to_compile_error()
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
        .note(self.specification_link())
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
                            .help(format!("There is at least one other field with the same name `{}` propably renamed with a #[graphql] attribute", dup.name))
                            .note(self.specification_link())
                            .emit();
                    });
            })
    }

    pub fn only_input_objects(&self, field: Span) {
        Diagnostic::spanned(
            field,
            Level::Error,
            format!(
                "{} requires all fields to be input objects or scalars",
                self
            ),
        )
        .help(format!(
            "Create a new object with the same fields and use #[derive(GraphQLInputObject)]"
        ))
        .note(self.specification_link())
        .emit();
    }

    pub fn no_input_objects(&self, field: Span) {
        Diagnostic::spanned(
            field,
            Level::Error,
            format!("{} does not allow input objects as fields", self),
        )
        .help(format!(
            "Create a new object with the same fields and use #[derive(GraphQLObject)]"
        ))
        .note(self.specification_link())
        .emit();
    }

    pub fn only_objects(&self, field: Span) {
        Diagnostic::spanned(
            field,
            Level::Error,
            format!("{} requires all fields to be objects", self),
        )
        .help(format!(
            "Using enums, scalars and input objects is not allowed. Warp enums and scalars with objects. For input objects create a new object with the same fields and use #[derive(GraphQLObject)]"
        ))
        .note(self.specification_link())
        .emit();
    }
}
