use std::{borrow::Cow, fmt, hash::Hash, slice, vec};

use indexmap::IndexMap;

use crate::{
    executor::Variables,
    parser::Spanning,
    value::{DefaultScalarValue, ScalarValue},
};

/// A type literal in the syntax tree
///
/// This enum carries no semantic information and might refer to types that do
/// not exist.
#[derive(Clone, Eq, PartialEq, Debug)]
pub enum Type<'a> {
    /// A nullable named type, e.g. `String`
    Named(Cow<'a, str>),
    /// A nullable list type, e.g. `[String]`
    ///
    /// The list itself is what's nullable, the containing type might be non-null.
    List(Box<Type<'a>>),
    /// A non-null named type, e.g. `String!`
    NonNullNamed(Cow<'a, str>),
    /// A non-null list type, e.g. `[String]!`.
    ///
    /// The list itself is what's non-null, the containing type might be null.
    NonNullList(Box<Type<'a>>),
}

/// A JSON-like value that can be passed into the query execution, either
/// out-of-band, or in-band as default variable values. These are _not_ constant
/// and might contain variables.
///
/// Lists and objects variants are _spanned_, i.e. they contain a reference to
/// their position in the source file, if available.
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs)]
pub enum InputValue<S = DefaultScalarValue> {
    Null,
    Scalar(S),
    Enum(String),
    Variable(String),
    List(Vec<Spanning<InputValue<S>>>),
    Object(Vec<(Spanning<String>, Spanning<InputValue<S>>)>),
}

#[derive(Clone, PartialEq, Debug)]
pub struct VariableDefinition<'a, S> {
    pub var_type: Spanning<Type<'a>>,
    pub default_value: Option<Spanning<InputValue<S>>>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Arguments<'a, S> {
    pub items: Vec<(Spanning<&'a str>, Spanning<InputValue<S>>)>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct VariableDefinitions<'a, S> {
    pub items: Vec<(Spanning<&'a str>, VariableDefinition<'a, S>)>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Field<'a, S> {
    pub alias: Option<Spanning<&'a str>>,
    pub name: Spanning<&'a str>,
    pub arguments: Option<Spanning<Arguments<'a, S>>>,
    pub directives: Option<Vec<Spanning<Directive<'a, S>>>>,
    pub selection_set: Option<Vec<Selection<'a, S>>>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct FragmentSpread<'a, S> {
    pub name: Spanning<&'a str>,
    pub directives: Option<Vec<Spanning<Directive<'a, S>>>>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct InlineFragment<'a, S> {
    pub type_condition: Option<Spanning<&'a str>>,
    pub directives: Option<Vec<Spanning<Directive<'a, S>>>>,
    pub selection_set: Vec<Selection<'a, S>>,
}

/// Entry in a GraphQL selection set
///
/// This enum represents one of the three variants of a selection that exists
/// in GraphQL: a field, a fragment spread, or an inline fragment. Each of the
/// variants references their location in the query source.
///
/// ```text
/// {
///   field(withArg: 123) { subField }
///   ...fragmentSpread
///   ...on User {
///     inlineFragmentField
///   }
/// }
/// ```
#[derive(Clone, PartialEq, Debug)]
#[allow(missing_docs)]
pub enum Selection<'a, S = DefaultScalarValue> {
    Field(Spanning<Field<'a, S>>),
    FragmentSpread(Spanning<FragmentSpread<'a, S>>),
    InlineFragment(Spanning<InlineFragment<'a, S>>),
}

#[derive(Clone, PartialEq, Debug)]
pub struct Directive<'a, S> {
    pub name: Spanning<&'a str>,
    pub arguments: Option<Spanning<Arguments<'a, S>>>,
}

#[derive(Clone, PartialEq, Debug)]
pub enum OperationType {
    Query,
    Mutation,
    Subscription,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Operation<'a, S> {
    pub operation_type: OperationType,
    pub name: Option<Spanning<&'a str>>,
    pub variable_definitions: Option<Spanning<VariableDefinitions<'a, S>>>,
    pub directives: Option<Vec<Spanning<Directive<'a, S>>>>,
    pub selection_set: Vec<Selection<'a, S>>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Fragment<'a, S> {
    pub name: Spanning<&'a str>,
    pub type_condition: Spanning<&'a str>,
    pub directives: Option<Vec<Spanning<Directive<'a, S>>>>,
    pub selection_set: Vec<Selection<'a, S>>,
}

#[derive(Clone, PartialEq, Debug)]
pub enum Definition<'a, S> {
    Operation(Spanning<Operation<'a, S>>),
    Fragment(Spanning<Fragment<'a, S>>),
}

pub type Document<'a, S> = Vec<Definition<'a, S>>;

/// Parse an unstructured input value into a Rust data type.
///
/// The conversion _can_ fail, and must in that case return None. Implemented
/// automatically by the convenience proc macro `graphql_scalar` or by deriving GraphQLEnum.
///
/// Must be implemented manually when manually exposing new enums or scalars.
pub trait FromInputValue<S = DefaultScalarValue>: Sized {
    /// Performs the conversion.
    fn from_input_value(v: &InputValue<S>) -> Option<Self>;
}

/// Losslessly clones a Rust data type into an InputValue.
pub trait ToInputValue<S = DefaultScalarValue>: Sized {
    /// Performs the conversion.
    fn to_input_value(&self) -> InputValue<S>;
}

impl<'a> Type<'a> {
    /// Get the name of a named type.
    ///
    /// Only applies to named types; lists will return `None`.
    pub fn name(&self) -> Option<&str> {
        match *self {
            Type::Named(ref n) | Type::NonNullNamed(ref n) => Some(n),
            _ => None,
        }
    }

    /// Get the innermost name by unpacking lists
    ///
    /// All type literals contain exactly one named type.
    pub fn innermost_name(&self) -> &str {
        match *self {
            Type::Named(ref n) | Type::NonNullNamed(ref n) => n,
            Type::List(ref l) | Type::NonNullList(ref l) => l.innermost_name(),
        }
    }

    /// Determines if a type only can represent non-null values.
    pub fn is_non_null(&self) -> bool {
        match *self {
            Type::NonNullNamed(_) | Type::NonNullList(_) => true,
            _ => false,
        }
    }
}

impl<'a> fmt::Display for Type<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Type::Named(ref n) => write!(f, "{}", n),
            Type::NonNullNamed(ref n) => write!(f, "{}!", n),
            Type::List(ref t) => write!(f, "[{}]", t),
            Type::NonNullList(ref t) => write!(f, "[{}]!", t),
        }
    }
}

impl<S> InputValue<S>
where
    S: ScalarValue,
{
    /// Construct a null value.
    pub fn null() -> Self {
        InputValue::Null
    }

    /// Construct an integer value.
    #[deprecated(since = "0.11.0", note = "Use `InputValue::scalar` instead")]
    pub fn int(i: i32) -> Self {
        Self::scalar(i)
    }

    /// Construct a floating point value.
    #[deprecated(since = "0.11.0", note = "Use `InputValue::scalar` instead")]
    pub fn float(f: f64) -> Self {
        Self::scalar(f)
    }

    /// Construct a boolean value.
    #[deprecated(since = "0.11.0", note = "Use `InputValue::scalar` instead")]
    pub fn boolean(b: bool) -> Self {
        Self::scalar(b)
    }

    /// Construct a string value.
    #[deprecated(since = "0.11.0", note = "Use `InputValue::scalar` instead")]
    pub fn string<T: AsRef<str>>(s: T) -> Self {
        InputValue::scalar(s.as_ref().to_owned())
    }

    /// Construct a scalar value
    pub fn scalar<T>(v: T) -> Self
    where
        T: Into<S>,
    {
        InputValue::Scalar(v.into())
    }

    /// Construct an enum value.
    pub fn enum_value<T: AsRef<str>>(s: T) -> Self {
        InputValue::Enum(s.as_ref().to_owned())
    }

    /// Construct a variable value.
    pub fn variable<T: AsRef<str>>(v: T) -> Self {
        InputValue::Variable(v.as_ref().to_owned())
    }

    /// Construct an unlocated list.
    ///
    /// Convenience function to make each `InputValue` in the input vector
    /// not contain any location information. Can be used from `ToInputValue`
    /// implementations, where no source code position information is available.
    pub fn list(l: Vec<Self>) -> Self {
        InputValue::List(l.into_iter().map(Spanning::unlocated).collect())
    }

    /// Construct a located list.
    pub fn parsed_list(l: Vec<Spanning<Self>>) -> Self {
        InputValue::List(l)
    }

    /// Construct an unlocated object.
    ///
    /// Similar to `InputValue::list`, it makes each key and value in the given
    /// hash map not contain any location information.
    pub fn object<K>(o: IndexMap<K, Self>) -> Self
    where
        K: AsRef<str> + Eq + Hash,
    {
        InputValue::Object(
            o.into_iter()
                .map(|(k, v)| {
                    (
                        Spanning::unlocated(k.as_ref().to_owned()),
                        Spanning::unlocated(v),
                    )
                })
                .collect(),
        )
    }

    /// Construct a located object.
    pub fn parsed_object(o: Vec<(Spanning<String>, Spanning<Self>)>) -> Self {
        InputValue::Object(o)
    }

    /// Resolve all variables to their values.
    pub fn into_const(self, vars: &Variables<S>) -> Self {
        match self {
            InputValue::Variable(v) => vars.get(&v).map_or_else(InputValue::null, Clone::clone),
            InputValue::List(l) => InputValue::List(
                l.into_iter()
                    .map(|s| s.map(|v| v.into_const(vars)))
                    .collect(),
            ),
            InputValue::Object(o) => InputValue::Object(
                o.into_iter()
                    .map(|(sk, sv)| (sk, sv.map(|v| v.into_const(vars))))
                    .collect(),
            ),
            v => v,
        }
    }

    /// Shorthand form of invoking `FromInputValue::from()`.
    pub fn convert<T>(&self) -> Option<T>
    where
        T: FromInputValue<S>,
    {
        <T as FromInputValue<S>>::from_input_value(self)
    }

    /// Does the value represent null?
    pub fn is_null(&self) -> bool {
        match *self {
            InputValue::Null => true,
            _ => false,
        }
    }

    /// Does the value represent a variable?
    pub fn is_variable(&self) -> bool {
        match *self {
            InputValue::Variable(_) => true,
            _ => false,
        }
    }

    /// View the underlying enum value, if present.
    pub fn as_enum_value(&self) -> Option<&str> {
        match *self {
            InputValue::Enum(ref e) => Some(e),
            _ => None,
        }
    }

    /// View the underlying int value, if present.
    pub fn as_int_value(&self) -> Option<i32> {
        self.as_scalar_value().and_then(|s| s.as_int())
    }

    /// View the underlying float value, if present.
    pub fn as_float_value(&self) -> Option<f64> {
        self.as_scalar_value().and_then(|s| s.as_float())
    }

    /// View the underlying string value, if present.
    pub fn as_string_value(&self) -> Option<&str> {
        self.as_scalar_value().and_then(|s| s.as_str())
    }

    /// View the underlying scalar value, if present.
    pub fn as_scalar(&self) -> Option<&S> {
        match *self {
            InputValue::Scalar(ref s) => Some(s),
            _ => None,
        }
    }

    /// View the underlying scalar value, if present.
    pub fn as_scalar_value<'a, T>(&'a self) -> Option<&'a T>
    where
        T: 'a,
        &'a S: Into<Option<&'a T>>,
    {
        self.as_scalar().and_then(Into::into)
    }

    /// Convert the input value to an unlocated object value.
    ///
    /// This constructs a new IndexMap that contain references to the keys
    /// and values in `self`.
    pub fn to_object_value<'a>(&'a self) -> Option<IndexMap<&'a str, &'a Self>> {
        match *self {
            InputValue::Object(ref o) => Some(
                o.iter()
                    .map(|&(ref sk, ref sv)| (sk.item.as_str(), &sv.item))
                    .collect(),
            ),
            _ => None,
        }
    }

    /// Convert the input value to an unlocated list value.
    ///
    /// This constructs a new vector that contain references to the values
    /// in `self`.
    pub fn to_list_value(&self) -> Option<Vec<&Self>> {
        match *self {
            InputValue::List(ref l) => Some(l.iter().map(|s| &s.item).collect()),
            _ => None,
        }
    }

    /// Recursively find all variables
    pub fn referenced_variables(&self) -> Vec<&str> {
        match *self {
            InputValue::Variable(ref name) => vec![name],
            InputValue::List(ref l) => l
                .iter()
                .flat_map(|v| v.item.referenced_variables())
                .collect(),
            InputValue::Object(ref obj) => obj
                .iter()
                .flat_map(|&(_, ref v)| v.item.referenced_variables())
                .collect(),
            _ => vec![],
        }
    }

    /// Compare equality with another `InputValue` ignoring any source position information.
    pub fn unlocated_eq(&self, other: &Self) -> bool {
        use crate::InputValue::*;

        match (self, other) {
            (&Null, &Null) => true,
            (&Scalar(ref s1), &Scalar(ref s2)) => s1 == s2,
            (&Enum(ref s1), &Enum(ref s2)) | (&Variable(ref s1), &Variable(ref s2)) => s1 == s2,
            (&List(ref l1), &List(ref l2)) => l1
                .iter()
                .zip(l2.iter())
                .all(|(v1, v2)| v1.item.unlocated_eq(&v2.item)),
            (&Object(ref o1), &Object(ref o2)) => {
                o1.len() == o2.len()
                    && o1.iter().all(|&(ref sk1, ref sv1)| {
                        o2.iter().any(|&(ref sk2, ref sv2)| {
                            sk1.item == sk2.item && sv1.item.unlocated_eq(&sv2.item)
                        })
                    })
            }
            _ => false,
        }
    }
}

impl<S> fmt::Display for InputValue<S>
where
    S: ScalarValue,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            InputValue::Null => write!(f, "null"),
            InputValue::Scalar(ref s) => {
                if let Some(s) = s.as_str() {
                    write!(f, "\"{}\"", s)
                } else {
                    write!(f, "{}", s)
                }
            }
            InputValue::Enum(ref v) => write!(f, "{}", v),
            InputValue::Variable(ref v) => write!(f, "${}", v),
            InputValue::List(ref v) => {
                write!(f, "[")?;

                for (i, spanning) in v.iter().enumerate() {
                    spanning.item.fmt(f)?;
                    if i < v.len() - 1 {
                        write!(f, ", ")?;
                    }
                }

                write!(f, "]")
            }
            InputValue::Object(ref o) => {
                write!(f, "{{")?;

                for (i, &(ref k, ref v)) in o.iter().enumerate() {
                    write!(f, "{}: ", k.item)?;
                    v.item.fmt(f)?;
                    if i < o.len() - 1 {
                        write!(f, ", ")?;
                    }
                }

                write!(f, "}}")
            }
        }
    }
}

impl<'a, S> Arguments<'a, S> {
    pub fn into_iter(self) -> vec::IntoIter<(Spanning<&'a str>, Spanning<InputValue<S>>)> {
        self.items.into_iter()
    }

    pub fn iter(&self) -> slice::Iter<(Spanning<&'a str>, Spanning<InputValue<S>>)> {
        self.items.iter()
    }

    pub fn iter_mut(&mut self) -> slice::IterMut<(Spanning<&'a str>, Spanning<InputValue<S>>)> {
        self.items.iter_mut()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn get(&self, key: &str) -> Option<&Spanning<InputValue<S>>> {
        self.items
            .iter()
            .filter(|&&(ref k, _)| k.item == key)
            .map(|&(_, ref v)| v)
            .next()
    }
}

impl<'a, S> VariableDefinitions<'a, S> {
    pub fn iter(&self) -> slice::Iter<(Spanning<&'a str>, VariableDefinition<S>)> {
        self.items.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::InputValue;
    use crate::parser::Spanning;

    #[test]
    fn test_input_value_fmt() {
        let value: InputValue = InputValue::null();
        assert_eq!(format!("{}", value), "null");

        let value: InputValue = InputValue::scalar(123);
        assert_eq!(format!("{}", value), "123");

        let value: InputValue = InputValue::scalar(12.3);
        assert_eq!(format!("{}", value), "12.3");

        let value: InputValue = InputValue::scalar("FOO".to_owned());
        assert_eq!(format!("{}", value), "\"FOO\"");

        let value: InputValue = InputValue::scalar(true);
        assert_eq!(format!("{}", value), "true");

        let value: InputValue = InputValue::enum_value("BAR".to_owned());
        assert_eq!(format!("{}", value), "BAR");

        let value: InputValue = InputValue::variable("baz".to_owned());
        assert_eq!(format!("{}", value), "$baz");

        let list = vec![InputValue::scalar(1), InputValue::scalar(2)];
        let value: InputValue = InputValue::list(list);
        assert_eq!(format!("{}", value), "[1, 2]");

        let object = vec![
            (
                Spanning::unlocated("foo".to_owned()),
                Spanning::unlocated(InputValue::scalar(1)),
            ),
            (
                Spanning::unlocated("bar".to_owned()),
                Spanning::unlocated(InputValue::scalar(2)),
            ),
        ];
        let value: InputValue = InputValue::parsed_object(object);
        assert_eq!(format!("{}", value), "{foo: 1, bar: 2}");
    }
}
