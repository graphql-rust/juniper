//! Code generation mode.

/// Code generation mode for macros.
#[derive(Debug)]
pub enum Mode {
    /// Generated code is intended to be used by library users.
    Public,

    /// Generated code is use only inside the library itself.
    Internal,
}

impl Mode {
    pub fn crate_path(&self) -> syn::Path {
        syn::parse_str::<syn::Path>(match self {
            Self::Public => "::juniper",
            Self::Internal => "crate",
        })
        .unwrap_or_else(|e| proc_macro_error::abort!(e))
    }
}

// TODO: Remove once all macros are refactored with `Mode`.
impl From<bool> for Mode {
    fn from(is_internal: bool) -> Self {
        if is_internal {
            Mode::Internal
        } else {
            Mode::Public
        }
    }
}
