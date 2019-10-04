
pub trait SyncObject<Val> {
    fn add_field<K>(&mut self, k: K, value: Val) -> Option<Val>;
//    fn iter(&self) -> FieldIter<Val>;
//    fn iter_mut(&mut self) -> FieldIterMut<Val>;
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
