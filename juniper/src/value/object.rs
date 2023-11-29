use std::mem;

use super::Value;
use indexmap::map::{IndexMap, IntoIter};

/// An Object value
#[derive(Debug, Clone, PartialEq)]
pub struct Object<S> {
    key_value_list: IndexMap<String, Value<S>>,
}

impl<S> Object<S> {
    /// Create a new Object value with a fixed number of
    /// preallocated slots for field-value pairs
    pub fn with_capacity(size: usize) -> Self {
        Object {
            key_value_list: IndexMap::with_capacity(size),
        }
    }

    /// Add a new field with a value
    ///
    /// If there is already a field for the given key
    /// any both values are objects, they are merged.
    ///
    /// Otherwise the existing value is replaced and
    /// returned.
    pub fn add_field<K>(&mut self, k: K, value: Value<S>) -> Option<Value<S>>
    where
        K: AsRef<str> + Into<String>,
    {
        if let Some(v) = self.key_value_list.get_mut(k.as_ref()) {
            Some(mem::replace(v, value))
        } else {
            self.key_value_list.insert(k.into(), value)
        }
    }

    /// Check if the object already contains a field with the given name
    pub fn contains_field<K: AsRef<str>>(&self, k: K) -> bool {
        self.key_value_list.contains_key(k.as_ref())
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
    pub fn get_field_value<K: AsRef<str>>(&self, key: K) -> Option<&Value<S>> {
        self.key_value_list.get(key.as_ref())
    }

    /// Get the mutable value for a given field
    pub fn get_mut_field_value<K: AsRef<str>>(&mut self, key: K) -> Option<&mut Value<S>> {
        self.key_value_list.get_mut(key.as_ref())
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
        Self::Object(o)
    }
}

impl<K, S> FromIterator<(K, Value<S>)> for Object<S>
where
    K: AsRef<str> + Into<String>,
{
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = (K, Value<S>)>,
    {
        let iter = iter.into_iter();
        let mut ret = Self {
            key_value_list: IndexMap::with_capacity(iter.size_hint().0),
        };
        for (k, v) in iter {
            ret.add_field(k, v);
        }
        ret
    }
}
