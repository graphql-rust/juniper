use std::{iter::FromIterator, vec::IntoIter};

use super::Value;
use crate::ValuesStream;

// todo: clone, PartialEq
//#[derive()]
pub struct StreamObject<S>
where
    S: 'static,
{
    key_value_list: Vec<(String, ValuesStream<S>)>,
}

// todo: better debug
impl<S> std::fmt::Debug for StreamObject<S>
where
    S: 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "StreamObject {{ key_value_list: Vec<(String, SubscriptionTypeAsync)> }}")
    }
}

impl<S> StreamObject<S>
where
    S: 'static,
{
    /// Create a new StreamObject value with a fixed number of
    /// preallocated slots for field-value pairs
    pub fn with_capacity(size: usize) -> Self {
        StreamObject {
            key_value_list: Vec::with_capacity(size),
        }
    }

    /// Add a new field with a value
    ///
    /// If there is already a field with the same name the old value
    /// is returned
    pub fn add_field<K>(&mut self, k: K, value: ValuesStream<S>) -> Option<ValuesStream<S>>
    where
        K: Into<String>,
        for<'a> &'a str: PartialEq<K>,
    {
        if let Some(item) = self
            .key_value_list
            .iter_mut()
            .find(|&&mut (ref key, _)| (key as &str) == k)
        {
            return Some(::std::mem::replace(&mut item.1, value));
        }
        self.key_value_list.push((k.into(), value));
        None
    }

    /// Check if the object already contains a field with the given name
    pub fn contains_field<K>(&self, f: K) -> bool
    where
        for<'a> &'a str: PartialEq<K>,
    {
        self.key_value_list
            .iter()
            .any(|&(ref key, _)| (key as &str) == f)
    }

    /// Get a iterator over all field value pairs
    pub fn iter(&self) -> impl Iterator<Item = &(String, ValuesStream<S>)> {
        FieldIter {
            inner: self.key_value_list.iter(),
        }
    }

    /// Get a iterator over all mutable field value pairs
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut (String, ValuesStream<S>)> {
        FieldIterMut {
            inner: self.key_value_list.iter_mut(),
        }
    }

    /// Get the current number of fields
    pub fn field_count(&self) -> usize {
        self.key_value_list.len()
    }

    /// Get the value for a given field
    pub fn get_field_value<K>(&self, key: K) -> Option<&ValuesStream<S>>
    where
        for<'a> &'a str: PartialEq<K>,
    {
        self.key_value_list
            .iter()
            .find(|&&(ref k, _)| (k as &str) == key)
            .map(|&(_, ref value)| value)
    }

    //todo: implement if needed
    //      or think about how to implement it later
//    /// Recursively sort all keys by field.
//    pub fn sort_by_field(&mut self) {
//        self.key_value_list
//            .sort_by(|(key1, _), (key2, _)| key1.cmp(key2));
//        for (_, ref mut value) in &mut self.key_value_list {
//            match value {
//                Value::Object(ref mut o) => {
//                    o.sort_by_field();
//                }
//                _ => {}
//            }
//        }
//    }

    pub fn into_joined_stream(self) -> ValuesStream<S> {
        use futures::stream::StreamExt;
        Box::pin(
            futures::stream::iter(self.key_value_list)
                .map(|(_, stream)| stream)
                .flatten()
        )
    }
}

impl<S> IntoIterator for StreamObject<S> {
    type Item = (String, ValuesStream<S>);
    type IntoIter = IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.key_value_list.into_iter()
    }
}

//todo: from that async type
//impl<S> From<StreamObject<S>> for Value<S> {
//    fn from(o: StreamObject<S>) -> Self {
//        Value::StreamObject(o)
//    }
//}

impl<K, S> FromIterator<(K, ValuesStream<S>)> for StreamObject<S>
where
    K: Into<String>,
    S: 'static,
    for<'a> &'a str: PartialEq<K>,
{
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = (K, ValuesStream<S>)>,
    {
        let iter = iter.into_iter();
        let mut ret = Self {
            key_value_list: Vec::with_capacity(iter.size_hint().0),
        };
        for (k, v) in iter {
            ret.add_field(k, v);
        }
        ret
    }
}

#[doc(hidden)]
pub struct FieldIter<'a, S: 'a> {
    inner: ::std::slice::Iter<'a, (String, ValuesStream<S>)>,
}

impl<'a, S> Iterator for FieldIter<'a, S> {
    type Item = &'a (String, ValuesStream<S>);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

#[doc(hidden)]
pub struct FieldIterMut<'a, S: 'a> {
    inner: ::std::slice::IterMut<'a, (String, ValuesStream<S>)>,
}

impl<'a, S> Iterator for FieldIterMut<'a, S> {
    type Item = &'a mut (String, ValuesStream<S>);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}
