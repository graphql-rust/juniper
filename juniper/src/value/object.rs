use std::{
    collections::{hash_map::IntoIter, HashMap},
    iter::FromIterator,
};

use super::Value;

/// A Object value
#[derive(Debug, Clone)]
pub struct Object<S> {
    key_value_list: HashMap<String, Value<S>>,
}

impl<S: PartialEq> PartialEq for Object<S> {
    fn eq(&self, _: &Object<S>) -> bool {
        match self {
            Object { key_value_list } => self.key_value_list == *key_value_list,
        }
    }
}

impl<S> Object<S> {
    /// Create a new Object value with a fixed number of
    /// preallocated slots for field-value pairs
    pub fn with_capacity(size: usize) -> Self {
        Object {
            key_value_list: HashMap::with_capacity(size),
        }
    }

    /// Add a new field with a value
    ///
    /// If there is already a field with the same name the old value
    /// is returned
    pub fn add_field<K>(&mut self, k: K, value: Value<S>) -> Option<Value<S>>
    where
        K: Into<String>,
        for<'a> &'a str: PartialEq<K>,
    {
        self.key_value_list.insert(k.into(), value)
    }

    /// Check if the object already contains a field with the given name
    pub fn contains_field<K>(&self, f: K) -> bool
    where
        K: Into<String>,
        for<'a> &'a str: PartialEq<K>,
    {
        self.key_value_list.contains_key(&f.into())
    }

    /// Get a iterator over all field value pairs
    pub fn iter(&self) -> impl Iterator<Item = (&String, &Value<S>)> {
        self.key_value_list.iter()
    }

    /// Get a iterator over all mutable field value pairs
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&String, &mut Value<S>)> {
        self.key_value_list.iter_mut()
    }

    /// Get the current number of fields
    pub fn field_count(&self) -> usize {
        self.key_value_list.len()
    }

    /// Get the value for a given field
    pub fn get_field_value<K>(&self, key: K) -> Option<&Value<S>>
    where
        K: Into<String>,
        for<'a> &'a str: PartialEq<K>,
    {
        self.key_value_list.get(&key.into())
    }
}

impl<S> IntoIterator for Object<S> {
    type Item = (String, Value<S>);
    type IntoIter = IntoIter<String, Value<S>>;

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
            key_value_list: HashMap::with_capacity(iter.size_hint().0),
        };
        for (k, v) in iter {
            ret.add_field(k, v);
        }
        ret
    }
}
