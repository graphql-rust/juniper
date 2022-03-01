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
    List(Box<Type<'a>>, Option<usize>),
    /// A non-null named type, e.g. `String!`
    NonNullNamed(Cow<'a, str>),
    /// A non-null list type, e.g. `[String]!`.
    ///
    /// The list itself is what's non-null, the containing type might be null.
    NonNullList(Box<Type<'a>>, Option<usize>),
}

#[cfg(feature = "arbitrary")]
impl<'a> arbitrary::Arbitrary<'a> for Type<'a> {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        let num_choices = 4;

        let ty = match u.int_in_range::<u8>(1..=num_choices)? {
            1 => Type::Named(u.arbitrary::<Cow<'a, str>>()?),
            2 => Type::List(
                u.arbitrary::<Box<Type<'a>>>()?,
                u.arbitrary::<Option<usize>>()?,
            ),
            3 => Type::NonNullNamed(u.arbitrary::<Cow<'a, str>>()?),
            4 => Type::NonNullList(
                u.arbitrary::<Box<Type<'a>>>()?,
                u.arbitrary::<Option<usize>>()?,
            ),
            _ => unreachable!(),
        };
        Ok(ty)
    }
}

/// A JSON-like value that can be passed into the query execution, either
/// out-of-band, or in-band as default variable values. These are _not_ constant
/// and might contain variables.
///
/// Lists and objects variants are _spanned_, i.e. they contain a reference to
/// their position in the source file, if available.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub enum InputValue<S = DefaultScalarValue> {
    Null,
    Scalar(S),
    Enum(String),
    Variable(String),
    List(Vec<Spanning<InputValue<S>>>),
    Object(Vec<(Spanning<String>, Spanning<InputValue<S>>)>),
}

#[derive(Clone, PartialEq, Debug)]
#[allow(missing_docs)]
pub struct VariableDefinition<'a, S> {
    pub var_type: Spanning<Type<'a>>,
    pub default_value: Option<Spanning<InputValue<S>>>,
    pub directives: Option<Vec<Spanning<Directive<'a, S>>>>,
}

#[cfg(feature = "arbitrary")]
impl<'a, S> arbitrary::Arbitrary<'a> for VariableDefinition<'a, S>
where
    S: arbitrary::Arbitrary<'a>,
{
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        let var_type: Spanning<Type<'a>> = u.arbitrary()?;
        let default_value: Option<Spanning<InputValue<S>>> = u.arbitrary()?;
        let directives: Option<Vec<Spanning<Directive<'a, S>>>> = u.arbitrary()?;

        Ok(Self {
            var_type,
            default_value,
            directives,
        })
    }
}

#[derive(Clone, PartialEq, Debug)]
#[allow(missing_docs)]
pub struct Arguments<'a, S> {
    pub items: Vec<(Spanning<&'a str>, Spanning<InputValue<S>>)>,
}

#[cfg(feature = "arbitrary")]
impl<'a, S> arbitrary::Arbitrary<'a> for Arguments<'a, S>
where
    S: arbitrary::Arbitrary<'a>,
{
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        let items: Vec<(Spanning<&'a str>, Spanning<InputValue<S>>)> = u.arbitrary()?;
        Ok(Self { items })
    }
}

#[derive(Clone, PartialEq, Debug)]
#[allow(missing_docs)]
pub struct VariableDefinitions<'a, S> {
    pub items: Vec<(Spanning<&'a str>, VariableDefinition<'a, S>)>,
}

#[cfg(feature = "arbitrary")]
impl<'a, S> arbitrary::Arbitrary<'a> for VariableDefinitions<'a, S>
where
    S: arbitrary::Arbitrary<'a>,
{
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        let items: Vec<(Spanning<&'a str>, VariableDefinition<'a, S>)> = u.arbitrary()?;
        Ok(Self { items })
    }
}

#[derive(Clone, PartialEq, Debug)]
#[allow(missing_docs)]
pub struct Field<'a, S> {
    pub alias: Option<Spanning<&'a str>>,
    pub name: Spanning<&'a str>,
    pub arguments: Option<Spanning<Arguments<'a, S>>>,
    pub directives: Option<Vec<Spanning<Directive<'a, S>>>>,
    pub selection_set: Option<Vec<Selection<'a, S>>>,
}

#[cfg(feature = "arbitrary")]
impl<'a, S> arbitrary::Arbitrary<'a> for Field<'a, S>
where
    S: arbitrary::Arbitrary<'a>,
{
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        let alias: Option<Spanning<&'a str>> = u.arbitrary()?;
        let name: Spanning<&'a str> = u.arbitrary()?;
        let arguments: Option<Spanning<Arguments<'a, S>>> = u.arbitrary()?;
        let directives: Option<Vec<Spanning<Directive<'a, S>>>> = u.arbitrary()?;
        let selection_set: Option<Vec<Selection<'a, S>>> = u.arbitrary()?;

        Ok(Self {
            alias,
            name,
            arguments,
            directives,
            selection_set,
        })
    }
}

#[derive(Clone, PartialEq, Debug)]
#[allow(missing_docs)]
pub struct FragmentSpread<'a, S> {
    pub name: Spanning<&'a str>,
    pub directives: Option<Vec<Spanning<Directive<'a, S>>>>,
}

#[cfg(feature = "arbitrary")]
impl<'a, S> arbitrary::Arbitrary<'a> for FragmentSpread<'a, S>
where
    S: arbitrary::Arbitrary<'a>,
{
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        let name: Spanning<&'a str> = u.arbitrary()?;
        let directives: Option<Vec<Spanning<Directive<'a, S>>>> = u.arbitrary()?;

        Ok(Self { name, directives })
    }
}

#[derive(Clone, PartialEq, Debug)]
#[allow(missing_docs)]
pub struct InlineFragment<'a, S> {
    pub type_condition: Option<Spanning<&'a str>>,
    pub directives: Option<Vec<Spanning<Directive<'a, S>>>>,
    pub selection_set: Vec<Selection<'a, S>>,
}

#[cfg(feature = "arbitrary")]
impl<'a, S> arbitrary::Arbitrary<'a> for InlineFragment<'a, S>
where
    S: arbitrary::Arbitrary<'a>,
{
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        let type_condition: Option<Spanning<&'a str>> = u.arbitrary()?;
        let directives: Option<Vec<Spanning<Directive<'a, S>>>> = u.arbitrary()?;
        let selection_set: Vec<Selection<'a, S>> = u.arbitrary()?;

        Ok(Self {
            type_condition,
            directives,
            selection_set,
        })
    }
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

#[cfg(feature = "arbitrary")]
impl<'a, S> arbitrary::Arbitrary<'a> for Selection<'a, S>
where
    S: arbitrary::Arbitrary<'a>,
{
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        let num_choices = 3;

        let ty = match u.int_in_range::<u8>(1..=num_choices)? {
            1 => Selection::Field(u.arbitrary::<Spanning<Field<'a, S>>>()?),
            2 => Selection::FragmentSpread(u.arbitrary::<Spanning<FragmentSpread<'a, S>>>()?),
            3 => Selection::InlineFragment(u.arbitrary::<Spanning<InlineFragment<'a, S>>>()?),
            _ => unreachable!(),
        };
        Ok(ty)
    }
}

#[derive(Clone, PartialEq, Debug)]
#[allow(missing_docs)]
pub struct Directive<'a, S> {
    pub name: Spanning<&'a str>,
    pub arguments: Option<Spanning<Arguments<'a, S>>>,
}

#[cfg(feature = "arbitrary")]
impl<'a, S> arbitrary::Arbitrary<'a> for Directive<'a, S>
where
    S: arbitrary::Arbitrary<'a>,
{
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        let name: Spanning<&'a str> = u.arbitrary()?;
        let arguments: Option<Spanning<Arguments<'a, S>>> = u.arbitrary()?;
        Ok(Self { name, arguments })
    }
}

#[derive(Clone, PartialEq, Debug)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[allow(missing_docs)]
pub enum OperationType {
    Query,
    Mutation,
    Subscription,
}

#[derive(Clone, PartialEq, Debug)]
#[allow(missing_docs)]
pub struct Operation<'a, S> {
    pub operation_type: OperationType,
    pub name: Option<Spanning<&'a str>>,
    pub variable_definitions: Option<Spanning<VariableDefinitions<'a, S>>>,
    pub directives: Option<Vec<Spanning<Directive<'a, S>>>>,
    pub selection_set: Vec<Selection<'a, S>>,
}

#[derive(Clone, PartialEq, Debug)]
#[allow(missing_docs)]
pub struct Fragment<'a, S> {
    pub name: Spanning<&'a str>,
    pub type_condition: Spanning<&'a str>,
    pub directives: Option<Vec<Spanning<Directive<'a, S>>>>,
    pub selection_set: Vec<Selection<'a, S>>,
}

#[derive(Clone, PartialEq, Debug)]
#[allow(missing_docs)]
pub enum Definition<'a, S> {
    Operation(Spanning<Operation<'a, S>>),
    Fragment(Spanning<Fragment<'a, S>>),
}

#[doc(hidden)]
pub type Document<'a, S> = [Definition<'a, S>];
#[doc(hidden)]
pub type OwnedDocument<'a, S> = Vec<Definition<'a, S>>;

/// Parsing of an unstructured input value into a Rust data type.
///
/// The conversion _can_ fail, and must in that case return [`Err`]. Thus not
/// restricted in the definition of this trait, the returned [`Err`] should be
/// convertible with [`IntoFieldError`] to fit well into the library machinery.
///
/// Implemented automatically by the convenience proc macro [`graphql_scalar!`]
/// or by deriving `GraphQLEnum`.
///
/// Must be implemented manually when manually exposing new enums or scalars.
///
/// [`graphql_scalar!`]: macro@crate::graphql_scalar
/// [`IntoFieldError`]: crate::IntoFieldError
pub trait FromInputValue<S = DefaultScalarValue>: Sized {
    /// Type of this conversion error.
    ///
    /// Thus not restricted, it should be convertible with [`IntoFieldError`] to
    /// fit well into the library machinery.
    ///
    /// [`IntoFieldError`]: crate::IntoFieldError
    type Error;

    /// Performs the conversion.
    fn from_input_value(v: &InputValue<S>) -> Result<Self, Self::Error>;

    /// Performs the conversion from an absent value (e.g. to distinguish
    /// between implicit and explicit `null`).
    ///
    /// The default implementation just calls [`from_input_value()`] as if an
    /// explicit `null` was provided.
    ///
    /// [`from_input_value()`]: FromInputValue::from_input_value
    fn from_implicit_null() -> Result<Self, Self::Error> {
        Self::from_input_value(&InputValue::<S>::Null)
    }
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
            Type::List(ref l, _) | Type::NonNullList(ref l, _) => l.innermost_name(),
        }
    }

    /// Determines if a type only can represent non-null values.
    pub fn is_non_null(&self) -> bool {
        matches!(*self, Type::NonNullNamed(_) | Type::NonNullList(..))
    }
}

impl<'a> fmt::Display for Type<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Named(n) => write!(f, "{}", n),
            Self::NonNullNamed(n) => write!(f, "{}!", n),
            Self::List(t, _) => write!(f, "[{}]", t),
            Self::NonNullList(t, _) => write!(f, "[{}]!", t),
        }
    }
}

impl<S> InputValue<S> {
    /// Construct a `null` value.
    pub fn null() -> Self {
        Self::Null
    }

    /// Construct a scalar value
    pub fn scalar<T>(v: T) -> Self
    where
        S: From<T>,
    {
        Self::Scalar(v.into())
    }

    /// Construct an enum value.
    pub fn enum_value<T: AsRef<str>>(s: T) -> Self {
        Self::Enum(s.as_ref().to_owned())
    }

    /// Construct a variable value.
    pub fn variable<T: AsRef<str>>(v: T) -> Self {
        Self::Variable(v.as_ref().to_owned())
    }

    /// Construct a [`Spanning::unlocated`] list.
    ///
    /// Convenience function to make each [`InputValue`] in the input vector
    /// not contain any location information. Can be used from [`ToInputValue`]
    /// implementations, where no source code position information is available.
    pub fn list(l: Vec<Self>) -> Self {
        Self::List(l.into_iter().map(Spanning::unlocated).collect())
    }

    /// Construct a located list.
    pub fn parsed_list(l: Vec<Spanning<Self>>) -> Self {
        Self::List(l)
    }

    /// Construct aa [`Spanning::unlocated`] object.
    ///
    /// Similarly to [`InputValue::list`] it makes each key and value in the
    /// given hash map not contain any location information.
    pub fn object<K>(o: IndexMap<K, Self>) -> Self
    where
        K: AsRef<str> + Eq + Hash,
    {
        Self::Object(
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
        Self::Object(o)
    }

    /// Resolve all variables to their values.
    #[must_use]
    pub fn into_const(self, vars: &Variables<S>) -> Self
    where
        S: Clone,
    {
        match self {
            Self::Variable(v) => vars.get(&v).map_or_else(InputValue::null, Clone::clone),
            Self::List(l) => Self::List(
                l.into_iter()
                    .map(|s| s.map(|v| v.into_const(vars)))
                    .collect(),
            ),
            Self::Object(o) => Self::Object(
                o.into_iter()
                    .map(|(sk, sv)| (sk, sv.map(|v| v.into_const(vars))))
                    .collect(),
            ),
            v => v,
        }
    }

    /// Shorthand form of invoking [`FromInputValue::from_input_value()`].
    pub fn convert<T: FromInputValue<S>>(&self) -> Result<T, T::Error> {
        T::from_input_value(self)
    }

    /// Does the value represent a `null`?
    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    /// Does the value represent a variable?
    pub fn is_variable(&self) -> bool {
        matches!(self, Self::Variable(_))
    }

    /// View the underlying enum value, if present.
    pub fn as_enum_value(&self) -> Option<&str> {
        match self {
            Self::Enum(e) => Some(e.as_str()),
            _ => None,
        }
    }

    /// View the underlying int value, if present.
    pub fn as_int_value(&self) -> Option<i32>
    where
        S: ScalarValue,
    {
        self.as_scalar_value().and_then(|s| s.as_int())
    }

    /// View the underlying float value, if present.
    pub fn as_float_value(&self) -> Option<f64>
    where
        S: ScalarValue,
    {
        self.as_scalar_value().and_then(|s| s.as_float())
    }

    /// View the underlying string value, if present.
    pub fn as_string_value(&self) -> Option<&str>
    where
        S: ScalarValue,
    {
        self.as_scalar_value().and_then(|s| s.as_str())
    }

    /// View the underlying scalar value, if present.
    pub fn as_scalar(&self) -> Option<&S> {
        match self {
            Self::Scalar(s) => Some(s),
            _ => None,
        }
    }

    /// View the underlying scalar value, if present.
    pub fn as_scalar_value<'a, T>(&'a self) -> Option<&'a T>
    where
        T: 'a,
        Option<&'a T>: From<&'a S>,
    {
        self.as_scalar().and_then(Into::into)
    }

    /// Converts this [`InputValue`] to a [`Spanning::unlocated`] object value.
    ///
    /// This constructs a new [`IndexMap`] containing references to the keys
    /// and values of `self`.
    pub fn to_object_value(&self) -> Option<IndexMap<&str, &Self>> {
        match self {
            Self::Object(o) => Some(
                o.iter()
                    .map(|(sk, sv)| (sk.item.as_str(), &sv.item))
                    .collect(),
            ),
            _ => None,
        }
    }

    /// Converts this [`InputValue`] to a [`Spanning::unlocated`] list value.
    ///
    /// This constructs a new [`Vec`] containing references to the values of
    /// `self`.
    pub fn to_list_value(&self) -> Option<Vec<&Self>> {
        match self {
            Self::List(l) => Some(l.iter().map(|s| &s.item).collect()),
            _ => None,
        }
    }

    /// Recursively finds all variables
    pub fn referenced_variables(&self) -> Vec<&str> {
        match self {
            Self::Variable(name) => vec![name.as_str()],
            Self::List(l) => l
                .iter()
                .flat_map(|v| v.item.referenced_variables())
                .collect(),
            Self::Object(o) => o
                .iter()
                .flat_map(|(_, v)| v.item.referenced_variables())
                .collect(),
            _ => vec![],
        }
    }

    /// Compares equality with another [`InputValue``] ignoring any source
    /// position information.
    pub fn unlocated_eq(&self, other: &Self) -> bool
    where
        S: PartialEq,
    {
        match (self, other) {
            (Self::Null, Self::Null) => true,
            (Self::Scalar(s1), Self::Scalar(s2)) => s1 == s2,
            (Self::Enum(s1), Self::Enum(s2)) | (Self::Variable(s1), Self::Variable(s2)) => s1 == s2,
            (Self::List(l1), Self::List(l2)) => l1
                .iter()
                .zip(l2.iter())
                .all(|(v1, v2)| v1.item.unlocated_eq(&v2.item)),
            (Self::Object(o1), Self::Object(o2)) => {
                o1.len() == o2.len()
                    && o1.iter().all(|(sk1, sv1)| {
                        o2.iter().any(|(sk2, sv2)| {
                            sk1.item == sk2.item && sv1.item.unlocated_eq(&sv2.item)
                        })
                    })
            }
            _ => false,
        }
    }
}

impl<S: ScalarValue> fmt::Display for InputValue<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Null => write!(f, "null"),
            Self::Scalar(s) => {
                if let Some(s) = s.as_str() {
                    write!(f, "\"{}\"", s)
                } else {
                    write!(f, "{}", s)
                }
            }
            Self::Enum(v) => write!(f, "{}", v),
            Self::Variable(v) => write!(f, "${}", v),
            Self::List(v) => {
                write!(f, "[")?;
                for (i, spanning) in v.iter().enumerate() {
                    spanning.item.fmt(f)?;
                    if i < v.len() - 1 {
                        write!(f, ", ")?;
                    }
                }
                write!(f, "]")
            }
            Self::Object(o) => {
                write!(f, "{{")?;
                for (i, (k, v)) in o.iter().enumerate() {
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

impl<S, T> From<Option<T>> for InputValue<S>
where
    Self: From<T>,
{
    fn from(v: Option<T>) -> Self {
        match v {
            Some(v) => v.into(),
            None => Self::Null,
        }
    }
}

impl<'a, S: From<String>> From<&'a str> for InputValue<S> {
    fn from(s: &'a str) -> Self {
        Self::scalar(s.to_owned())
    }
}

impl<'a, S: From<String>> From<Cow<'a, str>> for InputValue<S> {
    fn from(s: Cow<'a, str>) -> Self {
        Self::scalar(s.into_owned())
    }
}

impl<S: From<String>> From<String> for InputValue<S> {
    fn from(s: String) -> Self {
        Self::scalar(s)
    }
}

impl<S: From<i32>> From<i32> for InputValue<S> {
    fn from(i: i32) -> Self {
        Self::scalar(i)
    }
}

impl<S: From<f64>> From<f64> for InputValue<S> {
    fn from(f: f64) -> Self {
        Self::scalar(f)
    }
}

impl<S: From<bool>> From<bool> for InputValue<S> {
    fn from(b: bool) -> Self {
        Self::scalar(b)
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
    use crate::graphql_input_value;

    use super::InputValue;

    #[test]
    fn test_input_value_fmt() {
        let value: InputValue = graphql_input_value!(null);
        assert_eq!(format!("{}", value), "null");

        let value: InputValue = graphql_input_value!(123);
        assert_eq!(format!("{}", value), "123");

        let value: InputValue = graphql_input_value!(12.3);
        assert_eq!(format!("{}", value), "12.3");

        let value: InputValue = graphql_input_value!("FOO");
        assert_eq!(format!("{}", value), "\"FOO\"");

        let value: InputValue = graphql_input_value!(true);
        assert_eq!(format!("{}", value), "true");

        let value: InputValue = graphql_input_value!(BAR);
        assert_eq!(format!("{}", value), "BAR");

        let value: InputValue = graphql_input_value!(@baz);
        assert_eq!(format!("{}", value), "$baz");

        let value: InputValue = graphql_input_value!([1, 2]);
        assert_eq!(format!("{}", value), "[1, 2]");

        let value: InputValue = graphql_input_value!({"foo": 1,"bar": 2});
        assert_eq!(format!("{}", value), "{foo: 1, bar: 2}");
    }
}
