use std::{iter::FromIterator, vec::IntoIter};

use super::Value;
use crate::ValuesIterator;

use crate::value::base_object::{FieldIter, FieldIterMut};

// todo: clone, PartialEq
//#[derive(Debug)]
pub struct IterObject<S>
where
    S: 'static,
{
    key_value_list: Vec<(String, ValuesIterator<S>)>,
}

// todo: better debug
impl<S> std::fmt::Debug for IterObject<S>
where
    S: 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "IterObject: {:?}", self.key_value_list.iter().map(|(string, obj)| {
                string
            }).collect::<Vec<_>>()
        )
    }
}

impl<S> IterObject<S>
where
    S: 'static,
{
    /// Create a new IterObject value with a fixed number of
    /// preallocated slots for field-value pairs
    pub fn with_capacity(size: usize) -> Self {
        IterObject {
            key_value_list: Vec::with_capacity(size),
        }
    }

    /// Add a new field with a value
    ///
    /// If there is already a field with the same name the old value
    /// is returned
    pub fn add_field<K>(&mut self, k: K, value: ValuesIterator<S>) -> Option<ValuesIterator<S>>
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
    pub fn iter(&self) -> impl Iterator<Item = &(String, ValuesIterator<S>)> {
        //todo: we have 3 `FieldIter`s in different modules
        FieldIter {
            inner: self.key_value_list.iter(),
        }
    }

    /// Get a iterator over all mutable field value pairs
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut (String, ValuesIterator<S>)> {
         //todo: we have 3 `FieldIterMut`s in different modules
         FieldIterMut {
            inner: self.key_value_list.iter_mut(),
        }
    }

    /// Get the current number of fields
    pub fn field_count(&self) -> usize {
        self.key_value_list.len()
    }

    /// Get the value for a given field
    pub fn get_field_value<K>(&self, key: K) -> Option<&ValuesIterator<S>>
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

    pub fn into_joined_iterator(self) -> ValuesIterator<S> {
        use std::iter::Iterator;

        let iterators = self
            .key_value_list
            .into_iter()
            .map(|(_, iter)| iter)
            .flatten();

        Box::new(iterators)
    }

    //todo: more functions for return type
    pub fn into_key_value_list(self) -> Vec<(String, ValuesIterator<S>)> {
        self.key_value_list
    }
}

impl<S> IntoIterator for IterObject<S> {
    type Item = (String, ValuesIterator<S>);
    type IntoIter = IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.key_value_list.into_iter()
    }
}

//todo: from that async type
//impl<S> From<IterObject<S>> for Value<S> {
//    fn from(o: IterObject<S>) -> Self {
//        Value::IterObject(o)
//    }
//}

impl<K, S> FromIterator<(K, ValuesIterator<S>)> for IterObject<S>
where
    K: Into<String>,
    S: 'static,
    for<'a> &'a str: PartialEq<K>,
{
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = (K, ValuesIterator<S>)>,
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
