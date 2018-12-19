//! This crate supplies custom derive implementations for the
//! [juniper](https://github.com/graphql-rust/juniper) crate.
//!
//! You should not depend on juniper_codegen directly.
//! You only need the `juniper` crate.

#![recursion_limit = "1024"]

extern crate proc_macro;
extern crate proc_macro2;
#[macro_use]
extern crate quote;
#[macro_use]
extern crate syn;
#[macro_use]
extern crate lazy_static;
extern crate regex;

mod derive_enum;
mod derive_input_object;
mod derive_object;
mod derive_scalar_value;
mod util;

use proc_macro::TokenStream;

#[proc_macro_derive(GraphQLEnum, attributes(graphql))]
pub fn derive_enum(input: TokenStream) -> TokenStream {
    let ast = syn::parse::<syn::DeriveInput>(input).unwrap();
    let gen = derive_enum::impl_enum(&ast, false);
    gen.into()
}

#[proc_macro_derive(GraphQLEnumInternal, attributes(graphql))]
#[doc(hidden)]
pub fn derive_enum_internal(input: TokenStream) -> TokenStream {
    let ast = syn::parse::<syn::DeriveInput>(input).unwrap();
    let gen = derive_enum::impl_enum(&ast, true);
    gen.into()
}

#[proc_macro_derive(GraphQLInputObject, attributes(graphql))]
pub fn derive_input_object(input: TokenStream) -> TokenStream {
    let ast = syn::parse::<syn::DeriveInput>(input).unwrap();
    let gen = derive_input_object::impl_input_object(&ast, false);
    gen.into()
}

#[proc_macro_derive(GraphQLInputObjectInternal, attributes(graphql))]
#[doc(hidden)]
pub fn derive_input_object_internal(input: TokenStream) -> TokenStream {
    let ast = syn::parse::<syn::DeriveInput>(input).unwrap();
    let gen = derive_input_object::impl_input_object(&ast, true);
    gen.into()
}

#[proc_macro_derive(GraphQLObject, attributes(graphql))]
pub fn derive_object(input: TokenStream) -> TokenStream {
    let ast = syn::parse::<syn::DeriveInput>(input).unwrap();
    let gen = derive_object::impl_object(&ast);
    gen.into()
}

#[proc_macro_derive(GraphQLScalarValue)]
pub fn derive_scalar_value(input: TokenStream) -> TokenStream {
    let ast = syn::parse::<syn::DeriveInput>(input).unwrap();
    let gen = derive_scalar_value::impl_scalar_value(&ast, false);
    gen.into()
}

#[deprecated(note = "ScalarValue has been renamed to GraphQLScalarValue")]
#[proc_macro_derive(ScalarValue)]
pub fn derive_scalar_value_deprecated(input: TokenStream) -> TokenStream {
    derive_scalar_value(input)
}

#[proc_macro_derive(GraphQLScalarValueInternal)]
#[doc(hidden)]
pub fn derive_scalar_value_internal(input: TokenStream) -> TokenStream {
    let ast = syn::parse::<syn::DeriveInput>(input).unwrap();
    let gen = derive_scalar_value::impl_scalar_value(&ast, true);
    gen.into()
}
