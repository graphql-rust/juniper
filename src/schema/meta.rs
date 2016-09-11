//! Types used to describe a GraphQL schema


use std::fmt;

use ast::{InputValue, FromInputValue, Type};
use types::base::TypeKind;

/// Scalar type metadata
pub struct ScalarMeta {
    #[doc(hidden)]
    pub name: String,
    #[doc(hidden)]
    pub description: Option<String>,
    #[doc(hidden)]
    pub try_parse_fn: Box<Fn(&InputValue) -> bool + Send + Sync>,
}

/// List type metadata
#[derive(Debug)]
pub struct ListMeta {
    #[doc(hidden)]
    pub of_type: Type,
}

/// Nullable type metadata
#[derive(Debug)]
pub struct NullableMeta {
    #[doc(hidden)]
    pub of_type: Type,
}

/// Object type metadata
#[derive(Debug)]
pub struct ObjectMeta {
    #[doc(hidden)]
    pub name: String,
    #[doc(hidden)]
    pub description: Option<String>,
    #[doc(hidden)]
    pub fields: Vec<Field>,
    #[doc(hidden)]
    pub interface_names: Vec<String>,
}

/// Enum type metadata
pub struct EnumMeta {
    #[doc(hidden)]
    pub name: String,
    #[doc(hidden)]
    pub description: Option<String>,
    #[doc(hidden)]
    pub values: Vec<EnumValue>,
    #[doc(hidden)]
    pub try_parse_fn: Box<Fn(&InputValue) -> bool + Send + Sync>,
}

/// Interface type metadata
#[derive(Debug)]
pub struct InterfaceMeta {
    #[doc(hidden)]
    pub name: String,
    #[doc(hidden)]
    pub description: Option<String>,
    #[doc(hidden)]
    pub fields: Vec<Field>,
}

/// Union type metadata
#[derive(Debug)]
pub struct UnionMeta {
    #[doc(hidden)]
    pub name: String,
    #[doc(hidden)]
    pub description: Option<String>,
    #[doc(hidden)]
    pub of_type_names: Vec<String>,
}

/// Input object metadata
pub struct InputObjectMeta {
    #[doc(hidden)]
    pub name: String,
    #[doc(hidden)]
    pub description: Option<String>,
    #[doc(hidden)]
    pub input_fields: Vec<Argument>,
    #[doc(hidden)]
    pub try_parse_fn: Box<Fn(&InputValue) -> bool + Send + Sync>,
}

/// A placeholder for not-yet-registered types
///
/// After a type's `meta` method has been called but before it has returned, a placeholder type
/// is inserted into a registry to indicate existence.
#[derive(Debug)]
pub struct PlaceholderMeta {
    #[doc(hidden)]
    pub of_type: Type,
}

/// Generic type metadata
#[derive(Debug)]
pub enum MetaType {
    #[doc(hidden)]
    Scalar(ScalarMeta),
    #[doc(hidden)]
    List(ListMeta),
    #[doc(hidden)]
    Nullable(NullableMeta),
    #[doc(hidden)]
    Object(ObjectMeta),
    #[doc(hidden)]
    Enum(EnumMeta),
    #[doc(hidden)]
    Interface(InterfaceMeta),
    #[doc(hidden)]
    Union(UnionMeta),
    #[doc(hidden)]
    InputObject(InputObjectMeta),
    #[doc(hidden)]
    Placeholder(PlaceholderMeta),
}

/// Metadata for a field
#[derive(Debug, Clone)]
pub struct Field {
    #[doc(hidden)]
    pub name: String,
    #[doc(hidden)]
    pub description: Option<String>,
    #[doc(hidden)]
    pub arguments: Option<Vec<Argument>>,
    #[doc(hidden)]
    pub field_type: Type,
    #[doc(hidden)]
    pub deprecation_reason: Option<String>,
}

/// Metadata for an argument to a field
#[derive(Debug, Clone)]
pub struct Argument {
    #[doc(hidden)]
    pub name: String,
    #[doc(hidden)]
    pub description: Option<String>,
    #[doc(hidden)]
    pub arg_type: Type,
    #[doc(hidden)]
    pub default_value: Option<InputValue>,
}

/// Metadata for a single value in an enum
#[derive(Debug, Clone)]
pub struct EnumValue {
    /// The name of the enum value
    ///
    /// This is the string literal representation of the enum in responses.
    pub name: String,
    /// The optional description of the enum value.
    ///
    /// Note: this is not the description of the enum itself; it's the
    /// description of this enum _value_.
    pub description: Option<String>,
    /// The optional deprecation reason
    ///
    /// If this is `Some`, the field will be considered `isDeprecated`.
    pub deprecation_reason: Option<String>,
}

impl MetaType {
    /// Access the name of the type, if applicable
    ///
    /// Lists, non-null wrappers, and placeholders don't have names.
    pub fn name(&self) -> Option<&str> {
        match *self {
            MetaType::Scalar(ScalarMeta { ref name, .. }) |
            MetaType::Object(ObjectMeta { ref name, .. }) |
            MetaType::Enum(EnumMeta { ref name, .. }) |
            MetaType::Interface(InterfaceMeta { ref name, .. }) |
            MetaType::Union(UnionMeta { ref name, .. }) |
            MetaType::InputObject(InputObjectMeta { ref name, .. }) =>
                Some(name),
            _ => None,
        }
    }

    /// Access the description of the type, if applicable
    ///
    /// Lists, nullable wrappers, and placeholders don't have names.
    pub fn description(&self) -> Option<&String> {
        match *self {
            MetaType::Scalar(ScalarMeta { ref description, .. }) |
            MetaType::Object(ObjectMeta { ref description, .. }) |
            MetaType::Enum(EnumMeta { ref description, .. }) |
            MetaType::Interface(InterfaceMeta { ref description, .. }) |
            MetaType::Union(UnionMeta { ref description, .. }) |
            MetaType::InputObject(InputObjectMeta { ref description, .. }) =>
                description.as_ref(),
            _ => None,
        }
    }

    /// Construct a `TypeKind` for a given type
    ///
    /// # Panics
    /// Panics if the type represents a placeholder or nullable type.
    pub fn type_kind(&self) -> TypeKind {
        match *self {
            MetaType::Scalar(_) => TypeKind::Scalar,
            MetaType::List(_) => TypeKind::List,
            MetaType::Nullable(_) => panic!("Can't take type_kind of nullable meta type"),
            MetaType::Object(_) => TypeKind::Object,
            MetaType::Enum(_) => TypeKind::Enum,
            MetaType::Interface(_) => TypeKind::Interface,
            MetaType::Union(_) => TypeKind::Union,
            MetaType::InputObject(_) => TypeKind::InputObject,
            MetaType::Placeholder(_) => panic!("Can't take type_kind of placeholder meta type"),
        }
    }

    /// Access a field's meta data given its name
    ///
    /// Only objects and interfaces have fields. This method always returns `None` for other types.
    pub fn field_by_name(&self, name: &str) -> Option<&Field> {
        match *self {
            MetaType::Object(ObjectMeta { ref fields, .. }) |
            MetaType::Interface(InterfaceMeta { ref fields, .. }) =>
                fields.iter().filter(|f| f.name == name).next(),
            _ => None,
        }
    }

    /// Access an input field's meta data given its name
    ///
    /// Only input objects have input fields. This method always returns `None` for other types.
    pub fn input_field_by_name(&self, name: &str) -> Option<&Argument> {
        match *self {
            MetaType::InputObject(InputObjectMeta { ref input_fields, .. }) =>
                input_fields.iter().filter(|f| f.name == name).next(),
            _ => None,
        }
    }

    /// Construct a `Type` literal instance based on the metadata
    pub fn as_type(&self) -> Type {
        match *self {
            MetaType::Scalar(ScalarMeta { ref name, .. }) |
            MetaType::Object(ObjectMeta { ref name, .. }) |
            MetaType::Enum(EnumMeta { ref name, .. }) |
            MetaType::Interface(InterfaceMeta { ref name, .. }) |
            MetaType::Union(UnionMeta { ref name, .. }) |
            MetaType::InputObject(InputObjectMeta { ref name, .. }) =>
                Type::NonNullNamed(name.to_owned()),
            MetaType::List(ListMeta { ref of_type }) =>
                Type::NonNullList(Box::new(of_type.clone())),
            MetaType::Nullable(NullableMeta { ref of_type }) =>
                match *of_type {
                    Type::NonNullNamed(ref inner) => Type::Named(inner.to_owned()),
                    Type::NonNullList(ref inner) => Type::List(inner.clone()),
                    ref t => t.clone(),
                },
            MetaType::Placeholder(PlaceholderMeta { ref of_type }) => of_type.clone(),
        }
    }

    /// Access the input value parse function, if applicable
    ///
    /// An input value parse function is a function that takes an `InputValue` instance and returns
    /// `true` if it can be parsed as the provided type.
    ///
    /// Only scalars, enums, and input objects have parse functions.
    pub fn input_value_parse_fn(&self) -> Option<&Box<Fn(&InputValue) -> bool + Send + Sync>> {
        match *self {
            MetaType::Scalar(ScalarMeta { ref try_parse_fn, .. }) |
            MetaType::Enum(EnumMeta { ref try_parse_fn, .. }) |
            MetaType::InputObject(InputObjectMeta { ref try_parse_fn, .. }) =>
                Some(try_parse_fn),
            _ => None,
        }
    }

    /// Returns true if the type is a composite type
    ///
    /// Objects, interfaces, and unions are composite.
    pub fn is_composite(&self) -> bool {
        match *self {
            MetaType::Object(_) |
            MetaType::Interface(_) |
            MetaType::Union(_) => true,
            _ => false,
        }
    }

    /// Returns true if the type can occur in leaf positions in queries
    ///
    /// Only enums and scalars are leaf types.
    pub fn is_leaf(&self) -> bool {
        match *self {
            MetaType::Enum(_) |
            MetaType::Scalar(_) => true,
            _ => false,
        }
    }

    /// Returns true if the type is abstract
    ///
    /// Only interfaces and unions are abstract types.
    pub fn is_abstract(&self) -> bool {
        match *self {
            MetaType::Interface(_) |
            MetaType::Union(_) => true,
            _ => false,
        }
    }

    /// Returns true if the type can be used in input positions, e.g. arguments or variables
    ///
    /// Only scalars, enums, and input objects are input types.
    pub fn is_input(&self) -> bool {
        match *self {
            MetaType::Scalar(_) |
            MetaType::Enum(_) |
            MetaType::InputObject(_) => true,
            _ => false,
        }
    }
}

impl ScalarMeta {
    /// Build a new scalar type metadata with the specified name
    pub fn new<T: FromInputValue>(name: &str) -> ScalarMeta {
        ScalarMeta {
            name: name.to_owned(),
            description: None,
            try_parse_fn: Box::new(
                |v: &InputValue| <T as FromInputValue>::from(v).is_some()),
        }
    }

    /// Set the description for the given scalar type
    ///
    /// If a description already was set prior to calling this method, it will be overwritten.
    pub fn description(mut self, description: &str) -> ScalarMeta {
        self.description = Some(description.to_owned());
        self
    }

    /// Wrap the scalar in a generic meta type
    pub fn into_meta(self) -> MetaType {
        MetaType::Scalar(self)
    }
}

impl ListMeta {
    /// Build a new list type by wrapping the specified type
    pub fn new(of_type: Type) -> ListMeta {
        ListMeta {
            of_type: of_type,
        }
    }

    /// Wrap the list in a generic meta type
    pub fn into_meta(self) -> MetaType {
        MetaType::List(self)
    }
}

impl NullableMeta {
    /// Build a new nullable type by wrapping the specified type
    pub fn new(of_type: Type) -> NullableMeta {
        NullableMeta {
            of_type: of_type,
        }
    }

    /// Wrap the nullable type in a generic meta type
    pub fn into_meta(self) -> MetaType {
        MetaType::Nullable(self)
    }
}

impl ObjectMeta {
    /// Build a new object type with the specified name and fields
    pub fn new(name: &str, fields: &[Field]) -> ObjectMeta {
        ObjectMeta {
            name: name.to_owned(),
            description: None,
            fields: fields.to_vec(),
            interface_names: vec![],
        }
    }

    /// Set the description for the object
    ///
    /// If a description was provided prior to calling this method, it will be overwritten.
    pub fn description(mut self, description: &str) -> ObjectMeta {
        self.description = Some(description.to_owned());
        self
    }

    /// Set the interfaces this type implements
    ///
    /// If a list of interfaces already was provided prior to calling this method, they will be 
    /// overwritten.
    pub fn interfaces(mut self, interfaces: &[Type]) -> ObjectMeta {
        self.interface_names = interfaces.iter()
            .map(|t| t.innermost_name().to_owned()).collect();
        self
    }

    /// Wrap this object type in a generic meta type
    pub fn into_meta(self) -> MetaType {
        MetaType::Object(self)
    }
}

impl EnumMeta {
    /// Build a new enum type with the specified name and possible values
    pub fn new<T: FromInputValue>(name: &str, values: &[EnumValue]) -> EnumMeta {
        EnumMeta {
            name: name.to_owned(),
            description: None,
            values: values.to_vec(),
            try_parse_fn: Box::new(
                |v: &InputValue| <T as FromInputValue>::from(v).is_some()),
        }
    }

    /// Set the description of the type
    ///
    /// If a description was provided prior to calling this method, it will be overwritten
    pub fn description(mut self, description: &str) -> EnumMeta {
        self.description = Some(description.to_owned());
        self
    }

    /// Wrap this enum type in a generic meta type
    pub fn into_meta(self) -> MetaType {
        MetaType::Enum(self)
    }
}

impl InterfaceMeta {
    /// Build a new interface type with the specified name and fields
    pub fn new(name: &str, fields: &[Field]) -> InterfaceMeta {
        InterfaceMeta {
            name: name.to_owned(),
            description: None,
            fields: fields.to_vec(),
        }
    }

    /// Set the description of the type
    ///
    /// If a description was provided prior to calling this method, it will be overwritten.
    pub fn description(mut self, description: &str) -> InterfaceMeta {
        self.description = Some(description.to_owned());
        self
    }

    /// Wrap this interface type in a generic meta type
    pub fn into_meta(self) -> MetaType {
        MetaType::Interface(self)
    }
}

impl UnionMeta {
    /// Build a new union type with the specified name and possible types
    pub fn new(name: &str, of_types: &[Type]) -> UnionMeta {
        UnionMeta {
            name: name.to_owned(),
            description: None,
            of_type_names: of_types.iter()
                .map(|t| t.innermost_name().to_owned()).collect(),
        }
    }

    /// Set the description of the type
    ///
    /// If a description was provided prior to calling this method, it will be overwritten.
    pub fn description(mut self, description: &str) -> UnionMeta {
        self.description = Some(description.to_owned());
        self
    }

    /// Wrap this union type in a generic meta type
    pub fn into_meta(self) -> MetaType {
        MetaType::Union(self)
    }
}

impl InputObjectMeta {
    /// Build a new input type with the specified name and input fields
    pub fn new<T: FromInputValue>(name: &str, input_fields: &[Argument]) -> InputObjectMeta {
        InputObjectMeta {
            name: name.to_owned(),
            description: None,
            input_fields: input_fields.to_vec(),
            try_parse_fn: Box::new(
                |v: &InputValue| <T as FromInputValue>::from(v).is_some()),
        }
    }

    /// Set the description of the type
    ///
    /// If a description was provided prior to calling this method, it will be overwritten.
    pub fn description(mut self, description: &str) -> InputObjectMeta {
        self.description = Some(description.to_owned());
        self
    }

    /// Wrap this union type in a generic meta type
    pub fn into_meta(self) -> MetaType {
        MetaType::InputObject(self)
    }
}

impl Field {
    /// Set the description of the field
    ///
    /// This overwrites the description if any was previously set.
    pub fn description(mut self, description: &str) -> Field {
        self.description = Some(description.to_owned());
        self
    }

    /// Add an argument to the field
    ///
    /// Arguments are unordered and can't contain duplicates by name.
    pub fn argument(mut self, argument: Argument) -> Field {
        match self.arguments {
            None => { self.arguments = Some(vec![argument]); }
            Some(ref mut args) => { args.push(argument); }
        };

        self
    }

    /// Set the deprecation reason
    ///
    /// This overwrites the deprecation reason if any was previously set.
    pub fn deprecated(mut self, reason: &str) -> Field {
        self.deprecation_reason = Some(reason.to_owned());
        self
    }
}

impl Argument {
    #[doc(hidden)]
    pub fn new(name: &str, arg_type: Type) -> Argument {
        Argument {
            name: name.to_owned(),
            description: None,
            arg_type: arg_type,
            default_value: None
        }
    }

    /// Set the description of the argument
    ///
    /// This overwrites the description if any was previously set.
    pub fn description(mut self, description: &str) -> Argument {
        self.description = Some(description.to_owned());
        self
    }

    /// Set the default value of the argument
    ///
    /// This overwrites the description if any was previously set.
    pub fn default_value(mut self, default_value: InputValue) -> Argument {
        self.default_value = Some(default_value);
        self
    }
}

impl EnumValue {
    /// Construct a new enum value with the provided name
    pub fn new(name: &str) -> EnumValue {
        EnumValue {
            name: name.to_owned(),
            description: None,
            deprecation_reason: None,
        }
    }

    /// Set the description of the enum value
    ///
    /// This overwrites the description if any was previously set.
    pub fn description(mut self, description: &str) -> EnumValue {
        self.description = Some(description.to_owned());
        self
    }

    /// Set the deprecation reason for the enum value
    ///
    /// This overwrites the deprecation reason if any was previously set.
    pub fn deprecated(mut self, reason: &str) -> EnumValue {
        self.deprecation_reason = Some(reason.to_owned());
        self
    }
}

impl fmt::Debug for ScalarMeta {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("ScalarMeta")
            .field("name", &self.name)
            .field("description", &self.description)
            .finish()
    }
}

impl fmt::Debug for EnumMeta {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("EnumMeta")
            .field("name", &self.name)
            .field("description", &self.description)
            .field("values", &self.values)
            .finish()
    }
}

impl fmt::Debug for InputObjectMeta {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("InputObjectMeta")
            .field("name", &self.name)
            .field("description", &self.description)
            .field("input_fields", &self.input_fields)
            .finish()
    }
}
