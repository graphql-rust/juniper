pub(crate) mod parse;

/// Retrieves the innermost non-parenthesized [`syn::Type`] from the given one (unwraps nested
/// [`syn::TypeParen`]s asap).
pub(crate) fn unparenthesize(ty: &syn::Type) -> &syn::Type {
    match ty {
        syn::Type::Paren(ty) => unparenthesize(&*ty.elem),
        _ => ty,
    }
}