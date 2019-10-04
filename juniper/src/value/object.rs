use std::{iter::FromIterator, vec::IntoIter};

use super::Value;
use crate::value::base_object::{FieldIter, FieldIterMut, SyncObject};

/// A Object value
#[derive(Debug, Clone, PartialEq)]
pub struct Object<S> {
    key_value_list: Vec<(String, Value<S>)>,
}

impl<S> Object<S> {
    /// Create a new Object value with a fixed number of
    /// preallocated slots for field-value pairs
    pub fn with_capacity(size: usize) -> Self {
        Object {
            key_value_list: Vec::with_capacity(size),
        }
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

    /// Get the current number of fields
    pub fn field_count(&self) -> usize {
        self.key_value_list.len()
    }

    /// Get the value for a given field
    pub fn get_field_value<K>(&self, key: K) -> Option<&Value<S>>
    where
        for<'a> &'a str: PartialEq<K>,
    {
        self.key_value_list
            .iter()
            .find(|&&(ref k, _)| (k as &str) == key)
            .map(|&(_, ref value)| value)
    }

    /// Recursively sort all keys by field.
    pub fn sort_by_field(&mut self) {
        self.key_value_list
            .sort_by(|(key1, _), (key2, _)| key1.cmp(key2));
        for (_, ref mut value) in &mut self.key_value_list {
            match value {
                Value::Object(ref mut o) => {
                    o.sort_by_field();
                }
                _ => {}
            }
        }
    }
}

impl<S> SyncObject<Value<S>> for Object<S> {
    fn add_field<K>(&mut self, k: K, value: Value<S>) -> Option<Value<S>>
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

    fn iter(&self) -> FieldIter<Value<S>> {
        FieldIter {
            inner: self.key_value_list.iter(),
        }
    }

    fn iter_mut(&mut self) -> FieldIterMut<Value<S>> {
        FieldIterMut {
            inner: self.key_value_list.iter_mut(),
        }
    }
}

impl<S> IntoIterator for Object<S> {
    type Item = (String, Value<S>);
    type IntoIter = IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.key_value_list.into_iter()
    }
}

impl<S> From<Object<S>> for Value<S> {
    fn from(o: Object<S>) -> Self {
        Value::Object(o)
    }
}

impl<K, S> FromIterator<(K, Value<S>)> for Object<S>
where
    K: Into<String>,
    for<'a> &'a str: PartialEq<K>,
{
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = (K, Value<S>)>,
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


