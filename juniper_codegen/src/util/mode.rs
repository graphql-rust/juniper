//! Code generation mode.

/// Code generation mode for macros.
#[derive(Clone, Copy, Debug)]
pub enum Mode {
    /// Generated code is intended to be used by library users.
    Public,

    /// Generated code is use only inside the library itself.
    Internal,
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
