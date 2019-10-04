
pub trait SyncObject<Val> {
    /// Add a new field with a value
    ///
    /// If there is already a field with the same name the old value
    /// is returned
    fn add_field<K>(&mut self, k: K, value: Val) -> Option<Val>
    where
        K: Into<String>,
        for<'a> &'a str: PartialEq<K>;

    /// Get a iterator over all field value pairs
    fn iter(&self) -> FieldIter<Val>;

    /// Get a iterator over all mutable field value pairs
    fn iter_mut(&mut self) -> FieldIterMut<Val>;
}


#[doc(hidden)]
pub struct FieldIter<'a, S: 'a> {
    pub inner: ::std::slice::Iter<'a, (String, S)>,
}

impl<'a, S> Iterator for FieldIter<'a, S> {
    type Item = &'a (String, S);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

#[doc(hidden)]
pub struct FieldIterMut<'a, S: 'a> {
    pub inner: ::std::slice::IterMut<'a, (String, S)>,
}

impl<'a, S> Iterator for FieldIterMut<'a, S> {
    type Item = &'a mut (String, S);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}
