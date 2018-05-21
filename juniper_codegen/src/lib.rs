//! This crate supplies custom derive implementations for the
//! [juniper](https://github.com/graphql-rust/juniper) crate.
//!
//! You should not depend on juniper_codegen directly.
//! You only need the `juniper` crate.

#![recursion_limit = "1024"]

extern crate proc_macro;
#[macro_use]
extern crate quote;
extern crate syn;
#[macro_use]
extern crate lazy_static;
extern crate regex;

mod util;
mod derive_enum;
mod derive_input_object;
mod derive_object;

use proc_macro::TokenStream;

#[proc_macro_derive(GraphQLEnum, attributes(graphql))]
pub fn derive_enum(input: TokenStream) -> TokenStream {
    let ast = syn::parse::<syn::DeriveInput>(input).unwrap();
    let gen = derive_enum::impl_enum(&ast);
    gen.into()
}

#[proc_macro_derive(GraphQLInputObject, attributes(graphql))]
pub fn derive_input_object(input: TokenStream) -> TokenStream {
    let ast = syn::parse::<syn::DeriveInput>(input).unwrap();
    let gen = derive_input_object::impl_input_object(&ast);
    gen.into()
}

#[proc_macro_derive(GraphQLObject, attributes(graphql))]
pub fn derive_object(input: TokenStream) -> TokenStream {
    let ast = syn::parse::<syn::DeriveInput>(input).unwrap();
    let gen = derive_object::impl_object(&ast);
    gen.into()
}
