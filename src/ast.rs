use std::fmt;
use std::collections::HashMap;
use std::hash::Hash;
use std::vec;
use std::slice;

use rustc_serialize::json::{ToJson, Json};

use parser::Spanning;

/// A type literal in the syntax tree
///
/// This enum carries no semantic information and might refer to types that do
/// not exist.
#[derive(Clone, Eq, PartialEq, Debug)]
pub enum Type<'a> {
    /// A nullable named type, e.g. `String`
    Named(&'a str),
    /// A nullable list type, e.g. `[String]`
    ///
    /// The list itself is what's nullable, the containing type might be non-null.
    List(Box<Type<'a>>),
    /// A non-null named type, e.g. `String!`
    NonNullNamed(&'a str),
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
#[derive(Clone, PartialEq, Debug)]
#[allow(missing_docs)]
pub enum InputValue {
    Null,
    Int(i64),
    Float(f64),
    String(String),
    Boolean(bool),
    Enum(String),
    Variable(String),
    List(Vec<Spanning<InputValue>>),
    Object(Vec<(Spanning<String>, Spanning<InputValue>)>),
}

#[derive(Clone, PartialEq, Debug)]
pub struct VariableDefinition<'a> {
    pub var_type: Spanning<Type<'a>>,
    pub default_value: Option<Spanning<InputValue>>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Arguments<'a> {
    pub items: Vec<(Spanning<&'a str>, Spanning<InputValue>)>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct VariableDefinitions<'a> {
    pub items: Vec<(Spanning<&'a str>, VariableDefinition<'a>)>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Field<'a> {
    pub alias: Option<Spanning<&'a str>>,
    pub name: Spanning<&'a str>,
    pub arguments: Option<Spanning<Arguments<'a>>>,
    pub directives: Option<Vec<Spanning<Directive<'a>>>>,
    pub selection_set: Option<Vec<Selection<'a>>>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct FragmentSpread<'a> {
    pub name: Spanning<&'a str>,
    pub directives: Option<Vec<Spanning<Directive<'a>>>>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct InlineFragment<'a> {
    pub type_condition: Option<Spanning<&'a str>>,
    pub directives: Option<Vec<Spanning<Directive<'a>>>>,
    pub selection_set: Vec<Selection<'a>>,
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
pub enum Selection<'a> {
    Field(Spanning<Field<'a>>),
    FragmentSpread(Spanning<FragmentSpread<'a>>),
    InlineFragment(Spanning<InlineFragment<'a>>),
}

#[derive(Clone, PartialEq, Debug)]
pub struct Directive<'a> {
    pub name: Spanning<&'a str>,
    pub arguments: Option<Spanning<Arguments<'a>>>,
}

#[derive(Clone, PartialEq, Debug)]
pub enum OperationType {
    Query,
    Mutation,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Operation<'a> {
    pub operation_type: OperationType,
    pub name: Option<Spanning<&'a str>>,
    pub variable_definitions: Option<Spanning<VariableDefinitions<'a>>>,
    pub directives: Option<Vec<Spanning<Directive<'a>>>>,
    pub selection_set: Vec<Selection<'a>>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Fragment<'a> {
    pub name: Spanning<&'a str>,
    pub type_condition: Spanning<&'a str>,
    pub directives: Option<Vec<Spanning<Directive<'a>>>>,
    pub selection_set: Vec<Selection<'a>>,
}

#[derive(Clone, PartialEq, Debug)]
pub enum Definition<'a> {
    Operation(Spanning<Operation<'a>>),
    Fragment(Spanning<Fragment<'a>>),
}

pub type Document<'a> = Vec<Definition<'a>>;

/// Parse an unstructured input value into a Rust data type.
///
/// The conversion _can_ fail, and must in that case return None. Implemented
/// automatically by the convenience macros `graphql_enum!` and
/// `graphql_scalar!`. Must be implemented manually when manually exposing new
/// enums or scalars.
pub trait FromInputValue: Sized {
    /// Performs the conversion.
    fn from(v: &InputValue) -> Option<Self>;
}

/// Losslessly clones a Rust data type into an InputValue.
pub trait ToInputValue: Sized {
    /// Performs the conversion.
    fn to(&self) -> InputValue;
}

impl<'a> Type<'a> {
    /// Get the name of a named type.
    ///
    /// Only applies to named types; lists will return `None`.
    pub fn name(&self) -> Option<&str> {
        match *self {
            Type::Named(ref n) | Type::NonNullNamed(ref n) => Some(n),
            _ => None
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

impl InputValue {
    /// Construct a null value.
    pub fn null() -> InputValue { InputValue::Null }

    /// Construct an integer value.
    pub fn int(i: i64) -> InputValue { InputValue::Int(i) }

    /// Construct a floating point value.
    pub fn float(f: f64) -> InputValue { InputValue::Float(f) }

    /// Construct a boolean value.
    pub fn boolean(b: bool) -> InputValue { InputValue::Boolean(b) }

    /// Construct a string value.
    pub fn string<T: AsRef<str>>(s: T) -> InputValue {
        InputValue::String(s.as_ref().to_owned())
    }

    /// Construct an enum value.
    pub fn enum_value<T: AsRef<str>>(s: T) -> InputValue {
        InputValue::Enum(s.as_ref().to_owned())
    }

    /// Construct a variable value.
    pub fn variable<T: AsRef<str>>(v: T) -> InputValue {
        InputValue::Variable(v.as_ref().to_owned())
    }

    /// Construct an unlocated list.
    ///
    /// Convenience function to make each `InputValue` in the input vector
    /// not contain any location information. Can be used from `ToInputValue`
    /// implementations, where no source code position information is available.
    pub fn list(l: Vec<InputValue>) -> InputValue {
        InputValue::List(l.into_iter().map(|i| Spanning::unlocated(i)).collect())
    }

    /// Construct a located list.
    pub fn parsed_list(l: Vec<Spanning<InputValue>>) -> InputValue {
        InputValue::List(l)
    }

    /// Construct an unlocated object.
    ///
    /// Similar to `InputValue::list`, it makes each key and value in the given
    /// hash map not contain any location information.
    pub fn object<K>(o: HashMap<K, InputValue>) -> InputValue
        where K: AsRef<str> + Eq + Hash
    {
        InputValue::Object(
            o.into_iter()
                .map(|(k, v)|
                    (Spanning::unlocated(k.as_ref().to_owned()), Spanning::unlocated(v)))
                .collect()
        )
    }

    /// Construct a located object.
    pub fn parsed_object(o: Vec<(Spanning<String>, Spanning<InputValue>)>) -> InputValue {
        InputValue::Object(o)
    }

    /// Convert a `Json` structure into an `InputValue`.
    ///
    /// This consumes the JSON instance.
    ///
    /// Notes:
    /// * No enums or variables will be produced by this method.
    /// * All lists and objects will be unlocated
    pub fn from_json(json: Json) -> InputValue {
        match json {
            Json::I64(i) => InputValue::int(i),
            Json::U64(u) => InputValue::float(u as f64),
            Json::F64(f) => InputValue::float(f),
            Json::String(s) => InputValue::string(s),
            Json::Boolean(b) => InputValue::boolean(b),
            Json::Array(a) => InputValue::list(a.into_iter().map(InputValue::from_json).collect()),
            Json::Object(o) => InputValue::object(o.into_iter().map(|(k,v)| (k, InputValue::from_json(v))).collect()),
            Json::Null => InputValue::null(),
        }
    }

    /// Resolve all variables to their values.
    pub fn into_const(self, vars: &HashMap<String, InputValue>) -> InputValue {
        match self {
            InputValue::Variable(v) => vars.get(&v)
                .map_or_else(InputValue::null, Clone::clone),
            InputValue::List(l) => InputValue::List(
                l.into_iter().map(|s| s.map(|v| v.into_const(vars))).collect()
            ),
            InputValue::Object(o) => InputValue::Object(
                o.into_iter().map(|(sk, sv)| (sk, sv.map(|v| v.into_const(vars)))).collect()
            ),
            v => v,
        }
    }

    /// Shorthand form of invoking `FromInputValue::from()`.
    pub fn convert<T>(&self) -> Option<T> where T: FromInputValue {
        <T as FromInputValue>::from(self)
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
    pub fn as_int_value(&self) -> Option<i64> {
        match *self {
            InputValue::Int(i) => Some(i),
            _ => None,
        }
    }

    /// View the underlying string value, if present.
    pub fn as_string_value(&self) -> Option<&str> {
        match *self {
            InputValue::String(ref s) => Some(s),
            _ => None,
        }
    }

    /// Convert the input value to an unlocated object value.
    ///
    /// This constructs a new hashmap that contain references to the keys
    /// and values in `self`.
    pub fn to_object_value(&self) -> Option<HashMap<&str, &InputValue>> {
        match *self {
            InputValue::Object(ref o) => Some(
                o.iter().map(|&(ref sk, ref sv)| (sk.item.as_str(), &sv.item)).collect()),
            _ => None,
        }
    }

    /// Convert the input value to an unlocated list value.
    ///
    /// This constructs a new vector that contain references to the values
    /// in `self`.
    pub fn to_list_value(&self) -> Option<Vec<&InputValue>> {
        match *self {
            InputValue::List(ref l) => Some(l.iter().map(|s| &s.item).collect()),
            _ => None,
        }
    }

    /// Recursively find all variables
    pub fn referenced_variables(&self) -> Vec<&str> {
        match *self {
            InputValue::Variable(ref name) => vec![name],
            InputValue::List(ref l) => l.iter().flat_map(|v| v.item.referenced_variables()).collect(),
            InputValue::Object(ref obj) => obj.iter().flat_map(|&(_, ref v)| v.item.referenced_variables()).collect(),
            _ => vec![],
        }
    }

    /// Compare equality with another `InputValue` ignoring any source position information.
    pub fn unlocated_eq(&self, other: &InputValue) -> bool {
        use InputValue::*;

        match (self, other) {
            (&Null, &Null) => true,
            (&Int(i1), &Int(i2)) => i1 == i2,
            (&Float(f1), &Float(f2)) => f1 == f2,
            (&String(ref s1), &String(ref s2)) |
            (&Enum(ref s1), &Enum(ref s2)) |
            (&Variable(ref s1), &Variable(ref s2)) => s1 == s2,
            (&Boolean(b1), &Boolean(b2)) => b1 == b2,
            (&List(ref l1), &List(ref l2)) =>
                l1.iter().zip(l2.iter()).all(|(ref v1, ref v2)| v1.item.unlocated_eq(&v2.item)),
            (&Object(ref o1), &Object(ref o2)) =>
                o1.len() == o2.len()
                && o1.iter()
                    .all(|&(ref sk1, ref sv1)| o2.iter().any(
                        |&(ref sk2, ref sv2)| sk1.item == sk2.item && sv1.item.unlocated_eq(&sv2.item))),
            _ => false
        }
    }
}

impl ToJson for InputValue {
    fn to_json(&self) -> Json {
        match *self {
            InputValue::Null | InputValue::Variable(_) => Json::Null,
            InputValue::Int(i) => Json::I64(i),
            InputValue::Float(f) => Json::F64(f),
            InputValue::String(ref s) | InputValue::Enum(ref s) => Json::String(s.clone()),
            InputValue::Boolean(b) => Json::Boolean(b),
            InputValue::List(ref l) => Json::Array(l.iter().map(|x| x.item.to_json()).collect()),
            InputValue::Object(ref o) => Json::Object(o.iter().map(|&(ref k, ref v)| (k.item.clone(), v.item.to_json())).collect()),
       }
    }
}

impl<'a> Arguments<'a> {
    pub fn into_iter(self) -> vec::IntoIter<(Spanning<&'a str>, Spanning<InputValue>)> {
        self.items.into_iter()
    }

    pub fn iter(&self) -> slice::Iter<(Spanning<&'a str>, Spanning<InputValue>)> {
        self.items.iter()
    }

    pub fn iter_mut(&mut self) -> slice::IterMut<(Spanning<&'a str>, Spanning<InputValue>)> {
        self.items.iter_mut()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn get(&self, key: &str) -> Option<&Spanning<InputValue>> {
        self.items
            .iter()
            .filter(|&&(ref k, _)| k.item == key)
            .map(|&(_, ref v)| v)
            .next()
    }
}

impl<'a> VariableDefinitions<'a> {
    pub fn iter(&self) -> slice::Iter<(Spanning<&'a str>, VariableDefinition)> {
        self.items.iter()
    }
}
