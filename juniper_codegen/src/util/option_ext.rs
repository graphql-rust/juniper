/// Handy extension of [`Option`] methods used in this crate.
pub trait OptionExt {
    type Inner;

    /// Transforms the `Option<T>` into a `Result<(), E>`, mapping `None` to `Ok(())` and `Some(v)`
    /// to `Err(err(v))`.
    fn none_or_else<E, F>(self, err: F) -> Result<(), E>
    where
        F: FnOnce(Self::Inner) -> E;
}

impl<T> OptionExt for Option<T> {
    type Inner = T;

    fn none_or_else<E, F>(self, err: F) -> Result<(), E>
    where
        F: FnOnce(T) -> E,
    {
        match self {
            Some(v) => Err(err(v)),
            None => Ok(()),
        }
    }
}
