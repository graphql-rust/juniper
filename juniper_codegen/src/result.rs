//!

use crate::util::duplicate::Duplicate;
use proc_macro2::{Span, TokenStream};
use std::fmt;

#[allow(unused_variables)]
pub enum Generator {
    DeriveObject,
    DeriveInputObject,
    DeriveUnion,
    DeriveEnum,
    DeriveScalar,
    ImplUnion,
    ImplScalar,
    ImplObject,
}

impl fmt::Display for Generator {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let name = match self {
            Generator::DeriveObject => "#[derive(GraphQLObject)]",
            Generator::DeriveInputObject => "#[derive(GraphQLInputObject)]",
            Generator::DeriveUnion => "#[derive(GraphQLUnion)]",
            Generator::DeriveEnum => "#[derive(GraphQLEnum)]",
            Generator::DeriveScalar => "#[derive(GraphQLScalar)]",
            Generator::ImplUnion => "#[graphql_union]",
            Generator::ImplScalar => "#[graphql_scalar]",
            Generator::ImplObject => "#[graphql_object]",
        };

        write!(f, "{}", name)
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

impl Generator {
    pub fn custom(&self, span: Span, msg: &str) -> TokenStream {
        syn::Error::new(span, format!("{} {}", self, msg)).to_compile_error()
    }

    pub fn duplicate<'a, T: syn::spanned::Spanned + 'a>(
        &self,
        duplicates: impl Iterator<Item = &'a Duplicate<T>>,
    ) -> TokenStream {
        duplicates
            .map(|dup| {
                (&dup.spanned[1..])
                    .iter()
                    .map(|spanned| {
                        syn::Error::new(
                            spanned.span(),
                            format!(
                                "{} does not allow multiple fields/variants with the same name. There is at least one other field with the same name `{}`",
                                self, dup.name
                            ),
                        )
                            .to_compile_error()
                    })
                    .collect::<TokenStream>()
            })
            .collect()
    }

    pub fn unknown_attribute(&self, attribute: Span, value: String) -> TokenStream {
        syn::Error::new(
            attribute,
            format!("attribute `{}` was not recognized by #[graphql]", value),
        )
        .to_compile_error()
    }

    pub fn unsupported_attribute(
        &self,
        attribute: Span,
        kind: UnsupportedAttribute,
    ) -> TokenStream {
        syn::Error::new(
            attribute,
            format!("attribute `{:?}` is not supported by {}", kind, self),
        )
        .to_compile_error()
    }

    pub fn not_empty(&self, container: Span) -> TokenStream {
        syn::Error::new(
            container,
            format!("{} expects at least one field/variant", self),
        )
        .to_compile_error()
    }
}
