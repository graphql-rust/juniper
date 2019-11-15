use proc_macro::TokenStream;

use proc_macro_error::MacroError;
use quote::quote;
use syn::spanned::Spanned;

use crate::util;

struct ResolverVariant {
    pub ty: syn::Type,
    pub resolver: syn::Expr,
}

struct ResolveBody {
    pub variants: Vec<ResolverVariant>,
}

impl syn::parse::Parse for ResolveBody {
    fn parse(input: syn::parse::ParseStream) -> Result<Self, syn::parse::Error> {
        input.parse::<syn::token::Fn>()?;
        let ident = input.parse::<syn::Ident>()?;
        if ident != "resolve" {
            return Err(input.error("Expected method named 'resolve'"));
        }

        let args;
        syn::parenthesized!(args in input);
        args.parse::<syn::token::And>()?;
        args.parse::<syn::token::SelfValue>()?;
        if !args.is_empty() {
            return Err(
                input.error("Unexpected extra tokens: only one '&self' parameter is allowed")
            );
        }

        let body;
        syn::braced!( body in input );

        body.parse::<syn::token::Match>()?;
        body.parse::<syn::token::SelfValue>()?;

        let match_body;
        syn::braced!( match_body in body );

        let mut variants = Vec::new();
        while !match_body.is_empty() {
            let ty = match_body.parse::<syn::Type>()?;
            match_body.parse::<syn::token::FatArrow>()?;
            let resolver = match_body.parse::<syn::Expr>()?;

            variants.push(ResolverVariant { ty, resolver });

            // Optinal trailing comma.
            match_body.parse::<syn::token::Comma>().ok();
        }

        if !body.is_empty() {
            return Err(input.error("Unexpected input"));
        }

        Ok(Self { variants })
    }
}

pub fn impl_union(
    is_internal: bool,
    attrs: TokenStream,
    body: TokenStream,
) -> Result<TokenStream, MacroError> {
    // We are re-using the object attributes since they are almost the same.
    let attrs = syn::parse::<util::ObjectAttributes>(attrs)?;

    let item = syn::parse::<syn::ItemImpl>(body)?;

    if item.items.len() != 1 {
        return Err(MacroError::new(
            item.span(),
            "Invalid impl body: expected one method with signature: fn resolve(&self) { ... }"
                .to_string(),
        ));
    }

    let body_item = item.items.first().unwrap();
    let body = quote! { #body_item };
    let variants = syn::parse::<ResolveBody>(body.into())?.variants;

    let ty = &item.self_ty;

    let ty_ident = util::name_of_type(&*ty).ok_or_else(|| {
        MacroError::new(
            ty.span(),
            "Expected a path ending in a simple type identifier".to_string(),
        )
    })?;
    let name = attrs.name.unwrap_or_else(|| ty_ident.to_string());

    let juniper = util::juniper_path(is_internal);

    let meta_types = variants.iter().map(|var| {
        let var_ty = &var.ty;

        quote! {
            registry.get_type::<&#var_ty>(&(())),
        }
    });

    let concrete_type_resolver = variants.iter().map(|var| {
        let var_ty = &var.ty;
        let resolve = &var.resolver;

        quote! {
            if ({#resolve} as std::option::Option<&#var_ty>).is_some() {
                return <#var_ty as #juniper::GraphQLType<_>>::name(&()).unwrap().to_string();
            }
        }
    });

    let resolve_into_type = variants.iter().map(|var| {
        let var_ty = &var.ty;
        let resolve = &var.resolver;

        quote! {
            if type_name == (<#var_ty as #juniper::GraphQLType<_>>::name(&())).unwrap() {
                return executor.resolve(&(), &{ #resolve });
            }
        }
    });

    let scalar = attrs
        .scalar
        .as_ref()
        .map(|s| quote!( #s ))
        .unwrap_or_else(|| {
            quote! { #juniper::DefaultScalarValue }
        });

    let generics = item.generics.clone();
    let (impl_generics, _, where_clause) = generics.split_for_impl();

    let description = match attrs.description.as_ref() {
        Some(value) => quote!( .description( #value ) ),
        None => quote!(),
    };
    let context = attrs
        .context
        .map(|c| quote! { #c })
        .unwrap_or_else(|| quote! { () });

    let output = quote! {
        impl #impl_generics #juniper::GraphQLType<#scalar> for #ty #where_clause
        {
            type Context = #context;
            type TypeInfo = ();

            fn name(_ : &Self::TypeInfo) -> Option<&str> {
                Some(#name)
            }

            fn meta<'r>(
                info: &Self::TypeInfo,
                registry: &mut #juniper::Registry<'r, #scalar>
            ) -> #juniper::meta::MetaType<'r, #scalar>
                where
                    #scalar: 'r,
            {
                let types = &[
                    #( #meta_types )*
                ];
                registry.build_union_type::<#ty>(
                    info, types
                )
                    #description
                    .into_meta()
            }

            #[allow(unused_variables)]
            fn concrete_type_name(&self, context: &Self::Context, _info: &Self::TypeInfo) -> String {
                #( #concrete_type_resolver )*

                panic!("Concrete type not handled by instance resolvers on {}", #name);
            }

            fn resolve_into_type(
                &self,
                _info: &Self::TypeInfo,
                type_name: &str,
                _: Option<&[#juniper::Selection<#scalar>]>,
                executor: &#juniper::Executor<Self::Context, #scalar>,
            ) -> #juniper::ExecutionResult<#scalar> {
                let context = &executor.context();

                #( #resolve_into_type )*

                 panic!("Concrete type not handled by instance resolvers on {}", #name);
            }
        }


    };
    Ok(output.into())
}
