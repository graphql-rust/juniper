use std::mem;

use arcstr::ArcStr;
use foldhash::{HashMap, HashMapExt as _};

use super::Value;

/// Number of fields above which an [`Object`] indexes the names of its fields.
///
/// Up to this number, a lookup compares the names one by one. This is faster than hashing for the
/// small field counts of most GraphQL objects. Above this number, the index keeps each lookup at
/// constant time. A selection set with thousands of aliases then cannot make execution quadratic.
const INDEX_THRESHOLD: usize = 16;

/// Object value with uniquely named fields in insertion order.
///
/// Two objects are equal if they have the same fields with equal values, in any order.
#[derive(Clone, Debug)]
pub struct Object<S> {
    /// Fields in insertion order.
    fields: Vec<(ArcStr, Value<S>)>,

    /// Positions of the `fields` by name, if there are more than [`INDEX_THRESHOLD`] fields.
    ///
    /// The [`Box`] keeps a [`Value`] small, because most objects have no index.
    index: Option<Box<HashMap<ArcStr, usize>>>,
}

impl<S> Object<S> {
    /// Creates an empty object with space for `capacity` fields.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            fields: Vec::with_capacity(capacity),
            index: None,
        }
    }

    /// Adds a field with the `name` and the `value`.
    ///
    /// Replaces the value of an existing field with the `name`, and returns the replaced value.
    pub fn add_field(&mut self, name: impl Into<ArcStr>, value: Value<S>) -> Option<Value<S>> {
        let name = name.into();
        match self.position(&name) {
            Some(position) => Some(mem::replace(&mut self.fields[position].1, value)),
            None => {
                self.push(name, value);
                None
            }
        }
    }

    /// Merges a field with the `name` and the `value` into this object.
    ///
    /// The fields of a GraphQL selection set can have the same response name. Their values then
    /// combine into one value. Objects merge recursively, and lists merge the objects at the same
    /// positions. For other values, the value of the first field stays.
    pub(crate) fn merge_field(&mut self, name: ArcStr, value: Value<S>) {
        let Some(position) = self.position(&name) else {
            self.push(name, value);
            return;
        };
        match (&mut self.fields[position].1, value) {
            (Value::Object(existing), Value::Object(value)) => existing.merge(value),
            (Value::List(existing), Value::List(value)) => {
                for (existing, value) in existing.iter_mut().zip(value) {
                    if let (Value::Object(existing), Value::Object(value)) = (existing, value) {
                        existing.merge(value);
                    }
                }
            }
            _ => {}
        }
    }

    /// Merges all fields of the `other` object into this object with [`Object::merge_field()`].
    pub(crate) fn merge(&mut self, other: Self) {
        for (name, value) in other {
            self.merge_field(name, value);
        }
    }

    /// Indicates whether this object has a field with the `name`.
    pub fn contains_field(&self, name: impl AsRef<str>) -> bool {
        self.position(name.as_ref()).is_some()
    }

    /// Returns an iterator over the fields of this object in insertion order.
    pub fn iter(&self) -> impl Iterator<Item = (&ArcStr, &Value<S>)> {
        self.fields.iter().map(|(name, value)| (name, value))
    }

    /// Returns an iterator over the fields of this object in insertion order, with mutable values.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&ArcStr, &mut Value<S>)> {
        self.fields.iter_mut().map(|(name, value)| (&*name, value))
    }

    /// Returns the number of fields of this object.
    pub fn field_count(&self) -> usize {
        self.fields.len()
    }

    /// Returns the value of the field with the `name`.
    pub fn get_field_value(&self, name: impl AsRef<str>) -> Option<&Value<S>> {
        let position = self.position(name.as_ref())?;
        Some(&self.fields[position].1)
    }

    /// Returns the mutable value of the field with the `name`.
    pub fn get_mut_field_value(&mut self, name: impl AsRef<str>) -> Option<&mut Value<S>> {
        let position = self.position(name.as_ref())?;
        Some(&mut self.fields[position].1)
    }

    /// Returns the position of the field with the `name` in the `fields`.
    fn position(&self, name: &str) -> Option<usize> {
        match &self.index {
            Some(index) => index.get(name).copied(),
            None => self.fields.iter().position(|(n, _)| n == name),
        }
    }

    /// Appends a field. No field of this object has the `name` yet.
    fn push(&mut self, name: ArcStr, value: Value<S>) {
        let position = self.fields.len();
        if let Some(index) = &mut self.index {
            index.insert(name.clone(), position);
        } else if position == INDEX_THRESHOLD {
            let mut index = HashMap::with_capacity(2 * INDEX_THRESHOLD);
            for (position, (name, _)) in self.fields.iter().enumerate() {
                index.insert(name.clone(), position);
            }
            index.insert(name.clone(), position);
            self.index = Some(Box::new(index));
        }
        self.fields.push((name, value));
    }
}

impl<S: PartialEq> PartialEq for Object<S> {
    fn eq(&self, other: &Self) -> bool {
        self.field_count() == other.field_count()
            && self
                .iter()
                .all(|(name, value)| other.get_field_value(name) == Some(value))
    }
}

impl<S> IntoIterator for Object<S> {
    type Item = (ArcStr, Value<S>);
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.fields.into_iter()
    }
}

impl<K, S> FromIterator<(K, Value<S>)> for Object<S>
where
    K: Into<ArcStr>,
{
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = (K, Value<S>)>,
    {
        let iter = iter.into_iter();
        let mut object = Self::with_capacity(iter.size_hint().0);
        for (name, value) in iter {
            object.add_field(name, value);
        }
        object
    }
}

#[cfg(test)]
mod tests {
    mod add_field {
        use crate::{DefaultScalarValue, Object, Value};

        #[test]
        fn appends_new_field() {
            let mut target: Object<DefaultScalarValue> =
                graphql_value!({"a": 1}).into_object().unwrap();

            assert_eq!(target.add_field("b", graphql_value!(2)), None);

            assert_eq!(Value::Object(target), graphql_value!({"a": 1, "b": 2}));
        }

        #[test]
        fn replaces_existing_field() {
            let mut target: Object<DefaultScalarValue> =
                graphql_value!({"a": 1, "b": 2}).into_object().unwrap();

            assert_eq!(
                target.add_field("a", graphql_value!(3)),
                Some(graphql_value!(1)),
            );

            assert_eq!(target.field_count(), 2);
            assert_eq!(Value::Object(target), graphql_value!({"a": 3, "b": 2}));
        }
    }

    mod merge_field {
        use arcstr::literal;

        use crate::{DefaultScalarValue, Object, Value};

        #[test]
        fn appends_new_field() {
            let mut target: Object<DefaultScalarValue> =
                graphql_value!({"a": 1}).into_object().unwrap();

            target.merge_field(literal!("b"), graphql_value!(2));

            assert_eq!(Value::Object(target), graphql_value!({"a": 1, "b": 2}));
        }

        #[test]
        fn merges_objects() {
            let mut target: Object<DefaultScalarValue> =
                graphql_value!({"a": {"x": 1, "y": {"z": 2}}})
                    .into_object()
                    .unwrap();

            target.merge_field(literal!("a"), graphql_value!({"y": {"w": 3}, "v": 4}));

            assert_eq!(
                Value::Object(target),
                graphql_value!({"a": {"x": 1, "y": {"z": 2, "w": 3}, "v": 4}}),
            );
        }

        #[test]
        fn merges_objects_in_lists_by_position() {
            let mut target: Object<DefaultScalarValue> =
                graphql_value!({"a": [{"x": 1}, {"x": 2}]})
                    .into_object()
                    .unwrap();

            target.merge_field(literal!("a"), graphql_value!([{"y": 3}, {"y": 4}]));

            assert_eq!(
                Value::Object(target),
                graphql_value!({"a": [{"x": 1, "y": 3}, {"x": 2, "y": 4}]}),
            );
        }

        #[test]
        fn keeps_first_scalar() {
            let mut target: Object<DefaultScalarValue> =
                graphql_value!({"a": 1}).into_object().unwrap();

            target.merge_field(literal!("a"), graphql_value!(2));

            assert_eq!(Value::Object(target), graphql_value!({"a": 1}));
        }

        #[test]
        fn keeps_first_value_of_different_kind() {
            let mut target: Object<DefaultScalarValue> =
                graphql_value!({"a": {"x": 1}}).into_object().unwrap();

            target.merge_field(literal!("a"), graphql_value!([1, 2]));

            assert_eq!(Value::Object(target), graphql_value!({"a": {"x": 1}}));
        }
    }

    mod eq {
        use crate::Value;

        #[test]
        fn ignores_field_order() {
            let value: Value = graphql_value!({"a": 1, "b": 2});

            assert_eq!(value, graphql_value!({"b": 2, "a": 1}));
        }

        #[test]
        fn compares_values() {
            let value: Value = graphql_value!({"a": 1, "b": 2});

            assert_ne!(value, graphql_value!({"a": 1, "b": 3}));
        }

        #[test]
        fn compares_field_names() {
            let value: Value = graphql_value!({"a": 1});

            assert_ne!(value, graphql_value!({"b": 1}));
        }

        #[test]
        fn compares_field_counts() {
            let value: Value = graphql_value!({"a": 1});

            assert_ne!(value, graphql_value!({"a": 1, "b": 2}));
        }
    }

    mod wide {
        use arcstr::ArcStr;

        use super::super::INDEX_THRESHOLD;
        use crate::{DefaultScalarValue, Object, Value};

        /// Returns an object with more fields than [`INDEX_THRESHOLD`], so it uses the index.
        fn wide_object() -> Object<DefaultScalarValue> {
            (0..2 * INDEX_THRESHOLD)
                .map(|i| (format!("f{i}"), Value::scalar(i32::try_from(i).unwrap())))
                .collect()
        }

        #[test]
        fn finds_all_fields() {
            let target = wide_object();

            for i in 0..2 * INDEX_THRESHOLD {
                let expected = Value::scalar(i32::try_from(i).unwrap());
                assert_eq!(target.get_field_value(format!("f{i}")), Some(&expected));
            }
            assert!(!target.contains_field("missing"));
        }

        #[test]
        fn replaces_existing_field() {
            let mut target = wide_object();

            let replaced = target.add_field("f3", Value::scalar(-1));

            assert_eq!(replaced, Some(Value::scalar(3)));
            assert_eq!(target.get_field_value("f3"), Some(&Value::scalar(-1)));
            assert_eq!(target.field_count(), 2 * INDEX_THRESHOLD);
        }

        #[test]
        fn merges_existing_field() {
            let mut target = wide_object();
            target.add_field("nested", graphql_value!({"a": 1}));

            target.merge_field(ArcStr::from("nested"), graphql_value!({"b": 2}));

            assert_eq!(
                target.get_field_value("nested"),
                Some(&graphql_value!({"a": 1, "b": 2})),
            );
        }

        #[test]
        fn keeps_insertion_order() {
            let target = wide_object();

            let names = target
                .iter()
                .map(|(name, _)| name.to_string())
                .collect::<Vec<_>>();

            let expected = (0..2 * INDEX_THRESHOLD)
                .map(|i| format!("f{i}"))
                .collect::<Vec<_>>();
            assert_eq!(names, expected);
        }
    }

    #[test]
    fn iterates_in_insertion_order() {
        let target: crate::Object<crate::DefaultScalarValue> =
            graphql_value!({"b": 1, "a": 2, "c": 3})
                .into_object()
                .unwrap();

        let names = target
            .iter()
            .map(|(name, _)| name.as_str())
            .collect::<Vec<_>>();

        assert_eq!(names, ["b", "a", "c"]);
    }
}
