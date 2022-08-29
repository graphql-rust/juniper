pub trait Extract<T: ?Sized> {
    fn extract(&self) -> &T;
}

impl<T: ?Sized> Extract<T> for T {
    fn extract(&self) -> &Self {
        self
    }
}
