use syn::{
    parse::{Parse, ParseBuffer},
    token::Token,
};

pub trait ParseBufferExt {
    /// Tries to parse `T` as the next token.
    ///
    /// Doesn't move [`ParseStream`]'s cursor if there is no `T`.
    fn try_parse<T: Default + Parse + Token>(&self) -> syn::Result<Option<T>>;
}

impl<'a> ParseBufferExt for ParseBuffer<'a> {
    fn try_parse<T: Default + Parse + Token>(&self) -> syn::Result<Option<T>> {
        Ok(if self.lookahead1().peek(|_| T::default()) {
            Some(self.parse()?)
        } else {
            None
        })
    }
}
