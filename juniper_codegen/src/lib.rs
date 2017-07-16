#![recursion_limit = "1024"]

extern crate proc_macro;
extern crate syn;
#[macro_use]
extern crate quote;

mod util;
mod enums;
mod input_objects;

use proc_macro::TokenStream;

#[proc_macro_derive(GraphQLEnum, attributes(graphql))]
pub fn derive_enum(input: TokenStream) -> TokenStream {
    let s = input.to_string();
    let ast = syn::parse_derive_input(&s).unwrap();
    let gen = enums::impl_enum(&ast);
    gen.parse().unwrap()
}

#[proc_macro_derive(GraphQLInputObject, attributes(graphql))]
pub fn derive_input_object(input: TokenStream) -> TokenStream {
    let s = input.to_string();
    let ast = syn::parse_derive_input(&s).unwrap();
    let gen = input_objects::impl_input_object(&ast);
    gen.parse().unwrap()
}
