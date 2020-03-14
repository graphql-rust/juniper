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
        input.parse::<syn::token::Match>()?;
        input.parse::<syn::token::SelfValue>()?;

        let match_body;
        syn::braced!( match_body in input );

        let mut variants = Vec::new();
        while !match_body.is_empty() {
            let ty = match_body.parse::<syn::Type>()?;
            match_body.parse::<syn::token::FatArrow>()?;
            let resolver = match_body.parse::<syn::Expr>()?;

            variants.push(ResolverVariant { ty, resolver });

            // Optinal trailing comma.
            match_body.parse::<syn::token::Comma>().ok();
        }

        if !input.is_empty() {
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
    let _impl = util::parse_impl::ImplBlock::parse(attrs, body);

    // Validate trait target name, if present.
    if let Some((name, path)) = &_impl.target_trait {
        if !(name == "GraphQLUnion" || name == "juniper.GraphQLUnion") {
            return Err(MacroError::new(
                path.span(),
                "Invalid impl target trait: expected 'GraphQLUnion'".to_string(),
            ));
        }
    }

    let type_ident = &_impl.type_ident;
    let name = _impl
        .attrs
        .name
        .clone()
        .unwrap_or_else(|| type_ident.to_string());
    let crate_name = util::juniper_path(is_internal);

    let scalar = _impl
        .attrs
        .scalar
        .as_ref()
        .map(|s| quote!( #s ))
        .unwrap_or_else(|| {
            quote! { #crate_name::DefaultScalarValue }
        });

    if !_impl.has_resolve_method() {
        return Err(MacroError::new(
            _impl.target_type.span(),
            "Invalid impl body: expected one method with signature: fn resolve(&self) { ... }"
                .to_string(),
        ));
    }

    let method = _impl
        .methods
        .iter()
        .find(|&m| _impl.parse_resolve_method(&m).is_ok());

    if _impl.methods.is_empty() || method.is_none() {
        return Err(MacroError::new(
            _impl.target_type.span(),
            "Invalid impl body: expected one method with signature: fn resolve(&self) { ... }"
                .to_string(),
        ));
    }

    let method = method.expect("checked above");
    let resolve_args = _impl
        .parse_resolve_method(method)
        .expect("Invalid impl body: expected one method with signature: fn resolve(&self) { ... }");

    let stmts = &method.block.stmts;
    let body_raw = quote!( #( #stmts )* );
    let body = syn::parse::<ResolveBody>(body_raw.into())?;

    let meta_types = body.variants.iter().map(|var| {
        let var_ty = &var.ty;

        quote! {
            registry.get_type::<&#var_ty>(&(())),
        }
    });

    let concrete_type_resolver = body.variants.iter().map(|var| {
        let var_ty = &var.ty;
        let resolve = &var.resolver;

        quote! {
            if ({#resolve} as std::option::Option<&#var_ty>).is_some() {
                return <#var_ty as #crate_name::GraphQLType<#scalar>>::name(&()).unwrap().to_string();
            }
        }
    });

    let resolve_into_type = body.variants.iter().map(|var| {
        let var_ty = &var.ty;
        let resolve = &var.resolver;

        quote! {
            if type_name == (<#var_ty as #crate_name::GraphQLType<#scalar>>::name(&())).unwrap() {
                return executor.resolve(&(), &{ #resolve });
            }
        }
    });

    let generics = _impl.generics;
    let (impl_generics, _, where_clause) = generics.split_for_impl();

    let description = match _impl.description.as_ref() {
        Some(value) => quote!( .description( #value ) ),
        None => quote!(),
    };
    let context = _impl
        .attrs
        .context
        .map(|c| quote! { #c })
        .unwrap_or_else(|| quote! { () });

    let ty = _impl.target_type;

    let output = quote! {
        impl #impl_generics #crate_name::GraphQLType<#scalar> for #ty #where_clause
        {
            type Context = #context;
            type TypeInfo = ();

            fn name(_ : &Self::TypeInfo) -> Option<&str> {
                Some(#name)
            }

            fn meta<'r>(
                info: &Self::TypeInfo,
                registry: &mut #crate_name::Registry<'r, #scalar>
            ) -> #crate_name::meta::MetaType<'r, #scalar>
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
                _: Option<&[#crate_name::Selection<#scalar>]>,
                executor: &#crate_name::Executor<Self::Context, #scalar>,
            ) -> #crate_name::ExecutionResult<#scalar> {
                let context = &executor.context();
                #( #resolve_args )*

                #( #resolve_into_type )*

                 panic!("Concrete type not handled by instance resolvers on {}", #name);
            }
        }


    };
    Ok(output.into())
}
