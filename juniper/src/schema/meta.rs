//! Types used to describe a `GraphQL` schema

use std::borrow::Cow;
use std::fmt;

use ast::{FromInputValue, InputValue, Type};
use types::base::TypeKind;

/// Scalar type metadata
pub struct ScalarMeta<'a> {
    #[doc(hidden)]
    pub name: Cow<'a, str>,
    #[doc(hidden)]
    pub description: Option<String>,
    #[doc(hidden)]
    pub try_parse_fn: Box<Fn(&InputValue) -> bool + Send + Sync>,
}

/// List type metadata
#[derive(Debug)]
pub struct ListMeta<'a> {
    #[doc(hidden)]
    pub of_type: Type<'a>,
}

/// Nullable type metadata
#[derive(Debug)]
pub struct NullableMeta<'a> {
    #[doc(hidden)]
    pub of_type: Type<'a>,
}

/// Object type metadata
#[derive(Debug)]
pub struct ObjectMeta<'a> {
    #[doc(hidden)]
    pub name: Cow<'a, str>,
    #[doc(hidden)]
    pub description: Option<String>,
    #[doc(hidden)]
    pub fields: Vec<Field<'a>>,
    #[doc(hidden)]
    pub interface_names: Vec<String>,
}

/// Enum type metadata
pub struct EnumMeta<'a> {
    #[doc(hidden)]
    pub name: Cow<'a, str>,
    #[doc(hidden)]
    pub description: Option<String>,
    #[doc(hidden)]
    pub values: Vec<EnumValue>,
    #[doc(hidden)]
    pub try_parse_fn: Box<Fn(&InputValue) -> bool + Send + Sync>,
}

/// Interface type metadata
#[derive(Debug)]
pub struct InterfaceMeta<'a> {
    #[doc(hidden)]
    pub name: Cow<'a, str>,
    #[doc(hidden)]
    pub description: Option<String>,
    #[doc(hidden)]
    pub fields: Vec<Field<'a>>,
}

/// Union type metadata
#[derive(Debug)]
pub struct UnionMeta<'a> {
    #[doc(hidden)]
    pub name: Cow<'a, str>,
    #[doc(hidden)]
    pub description: Option<String>,
    #[doc(hidden)]
    pub of_type_names: Vec<String>,
}

/// Input object metadata
pub struct InputObjectMeta<'a> {
    #[doc(hidden)]
    pub name: Cow<'a, str>,
    #[doc(hidden)]
    pub description: Option<String>,
    #[doc(hidden)]
    pub input_fields: Vec<Argument<'a>>,
    #[doc(hidden)]
    pub try_parse_fn: Box<Fn(&InputValue) -> bool + Send + Sync>,
}

/// A placeholder for not-yet-registered types
///
/// After a type's `meta` method has been called but before it has returned, a placeholder type
/// is inserted into a registry to indicate existence.
#[derive(Debug)]
pub struct PlaceholderMeta<'a> {
    #[doc(hidden)]
    pub of_type: Type<'a>,
}

/// Generic type metadata
#[derive(Debug)]
pub enum MetaType<'a> {
    #[doc(hidden)] Scalar(ScalarMeta<'a>),
    #[doc(hidden)] List(ListMeta<'a>),
    #[doc(hidden)] Nullable(NullableMeta<'a>),
    #[doc(hidden)] Object(ObjectMeta<'a>),
    #[doc(hidden)] Enum(EnumMeta<'a>),
    #[doc(hidden)] Interface(InterfaceMeta<'a>),
    #[doc(hidden)] Union(UnionMeta<'a>),
    #[doc(hidden)] InputObject(InputObjectMeta<'a>),
    #[doc(hidden)] Placeholder(PlaceholderMeta<'a>),
}

/// Metadata for a field
#[derive(Debug, Clone)]
pub struct Field<'a> {
    #[doc(hidden)]
    pub name: String,
    #[doc(hidden)]
    pub description: Option<String>,
    #[doc(hidden)]
    pub arguments: Option<Vec<Argument<'a>>>,
    #[doc(hidden)]
    pub field_type: Type<'a>,
    #[doc(hidden)]
    pub deprecation_reason: Option<String>,
}

/// Metadata for an argument to a field
#[derive(Debug, Clone)]
pub struct Argument<'a> {
    #[doc(hidden)]
    pub name: String,
    #[doc(hidden)]
    pub description: Option<String>,
    #[doc(hidden)]
    pub arg_type: Type<'a>,
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

impl<'a> MetaType<'a> {
    /// Access the name of the type, if applicable
    ///
    /// Lists, non-null wrappers, and placeholders don't have names.
    pub fn name(&self) -> Option<&str> {
        match *self {
            MetaType::Scalar(ScalarMeta { ref name, .. })
            | MetaType::Object(ObjectMeta { ref name, .. })
            | MetaType::Enum(EnumMeta { ref name, .. })
            | MetaType::Interface(InterfaceMeta { ref name, .. })
            | MetaType::Union(UnionMeta { ref name, .. })
            | MetaType::InputObject(InputObjectMeta { ref name, .. }) => Some(name),
            _ => None,
        }
    }

    /// Access the description of the type, if applicable
    ///
    /// Lists, nullable wrappers, and placeholders don't have names.
    pub fn description(&self) -> Option<&String> {
        match *self {
            MetaType::Scalar(ScalarMeta {
                ref description, ..
            })
            | MetaType::Object(ObjectMeta {
                ref description, ..
            })
            | MetaType::Enum(EnumMeta {
                ref description, ..
            })
            | MetaType::Interface(InterfaceMeta {
                ref description, ..
            })
            | MetaType::Union(UnionMeta {
                ref description, ..
            })
            | MetaType::InputObject(InputObjectMeta {
                ref description, ..
            }) => description.as_ref(),
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
            MetaType::Object(ObjectMeta { ref fields, .. })
            | MetaType::Interface(InterfaceMeta { ref fields, .. }) => {
                fields.iter().find(|f| f.name == name)
            }
            _ => None,
        }
    }

    /// Access an input field's meta data given its name
    ///
    /// Only input objects have input fields. This method always returns `None` for other types.
    pub fn input_field_by_name(&self, name: &str) -> Option<&Argument> {
        match *self {
            MetaType::InputObject(InputObjectMeta {
                ref input_fields, ..
            }) => input_fields.iter().find(|f| f.name == name),
            _ => None,
        }
    }

    /// Construct a `Type` literal instance based on the metadata
    pub fn as_type(&self) -> Type<'a> {
        match *self {
            MetaType::Scalar(ScalarMeta { ref name, .. })
            | MetaType::Object(ObjectMeta { ref name, .. })
            | MetaType::Enum(EnumMeta { ref name, .. })
            | MetaType::Interface(InterfaceMeta { ref name, .. })
            | MetaType::Union(UnionMeta { ref name, .. })
            | MetaType::InputObject(InputObjectMeta { ref name, .. }) => {
                Type::NonNullNamed(name.clone())
            }
            MetaType::List(ListMeta { ref of_type }) => {
                Type::NonNullList(Box::new(of_type.clone()))
            }
            MetaType::Nullable(NullableMeta { ref of_type }) => match *of_type {
                Type::NonNullNamed(ref inner) => Type::Named(inner.clone()),
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
            MetaType::Scalar(ScalarMeta {
                ref try_parse_fn, ..
            })
            | MetaType::Enum(EnumMeta {
                ref try_parse_fn, ..
            })
            | MetaType::InputObject(InputObjectMeta {
                ref try_parse_fn, ..
            }) => Some(try_parse_fn),
            _ => None,
        }
    }

    /// Returns true if the type is a composite type
    ///
    /// Objects, interfaces, and unions are composite.
    pub fn is_composite(&self) -> bool {
        match *self {
            MetaType::Object(_) | MetaType::Interface(_) | MetaType::Union(_) => true,
            _ => false,
        }
    }

    /// Returns true if the type can occur in leaf positions in queries
    ///
    /// Only enums and scalars are leaf types.
    pub fn is_leaf(&self) -> bool {
        match *self {
            MetaType::Enum(_) | MetaType::Scalar(_) => true,
            _ => false,
        }
    }

    /// Returns true if the type is abstract
    ///
    /// Only interfaces and unions are abstract types.
    pub fn is_abstract(&self) -> bool {
        match *self {
            MetaType::Interface(_) | MetaType::Union(_) => true,
            _ => false,
        }
    }

    /// Returns true if the type can be used in input positions, e.g. arguments or variables
    ///
    /// Only scalars, enums, and input objects are input types.
    pub fn is_input(&self) -> bool {
        match *self {
            MetaType::Scalar(_) | MetaType::Enum(_) | MetaType::InputObject(_) => true,
            _ => false,
        }
    }
}

impl<'a> ScalarMeta<'a> {
    /// Build a new scalar type metadata with the specified name
    pub fn new<T: FromInputValue>(name: Cow<'a, str>) -> ScalarMeta<'a> {
        ScalarMeta {
            name: name,
            description: None,
            try_parse_fn: Box::new(|v: &InputValue| {
                <T as FromInputValue>::from_input_value(v).is_some()
            }),
        }
    }

    /// Set the description for the given scalar type
    ///
    /// If a description already was set prior to calling this method, it will be overwritten.
    pub fn description(mut self, description: &str) -> ScalarMeta<'a> {
        self.description = Some(description.to_owned());
        self
    }

    /// Wrap the scalar in a generic meta type
    pub fn into_meta(self) -> MetaType<'a> {
        MetaType::Scalar(self)
    }
}

impl<'a> ListMeta<'a> {
    /// Build a new list type by wrapping the specified type
    pub fn new(of_type: Type<'a>) -> ListMeta<'a> {
        ListMeta { of_type: of_type }
    }

    /// Wrap the list in a generic meta type
    pub fn into_meta(self) -> MetaType<'a> {
        MetaType::List(self)
    }
}

impl<'a> NullableMeta<'a> {
    /// Build a new nullable type by wrapping the specified type
    pub fn new(of_type: Type<'a>) -> NullableMeta<'a> {
        NullableMeta { of_type: of_type }
    }

    /// Wrap the nullable type in a generic meta type
    pub fn into_meta(self) -> MetaType<'a> {
        MetaType::Nullable(self)
    }
}

impl<'a> ObjectMeta<'a> {
    /// Build a new object type with the specified name and fields
    pub fn new(name: Cow<'a, str>, fields: &[Field<'a>]) -> ObjectMeta<'a> {
        ObjectMeta {
            name: name,
            description: None,
            fields: fields.to_vec(),
            interface_names: vec![],
        }
    }

    /// Set the description for the object
    ///
    /// If a description was provided prior to calling this method, it will be overwritten.
    pub fn description(mut self, description: &str) -> ObjectMeta<'a> {
        self.description = Some(description.to_owned());
        self
    }

    /// Set the interfaces this type implements
    ///
    /// If a list of interfaces already was provided prior to calling this method, they will be
    /// overwritten.
    pub fn interfaces(mut self, interfaces: &[Type<'a>]) -> ObjectMeta<'a> {
        self.interface_names = interfaces
            .iter()
            .map(|t| t.innermost_name().to_owned())
            .collect();
        self
    }

    /// Wrap this object type in a generic meta type
    pub fn into_meta(self) -> MetaType<'a> {
        MetaType::Object(self)
    }
}

impl<'a> EnumMeta<'a> {
    /// Build a new enum type with the specified name and possible values
    pub fn new<T: FromInputValue>(name: Cow<'a, str>, values: &[EnumValue]) -> EnumMeta<'a> {
        EnumMeta {
            name: name,
            description: None,
            values: values.to_vec(),
            try_parse_fn: Box::new(|v: &InputValue| {
                <T as FromInputValue>::from_input_value(v).is_some()
            }),
        }
    }

    /// Set the description of the type
    ///
    /// If a description was provided prior to calling this method, it will be overwritten
    pub fn description(mut self, description: &str) -> EnumMeta<'a> {
        self.description = Some(description.to_owned());
        self
    }

    /// Wrap this enum type in a generic meta type
    pub fn into_meta(self) -> MetaType<'a> {
        MetaType::Enum(self)
    }
}

impl<'a> InterfaceMeta<'a> {
    /// Build a new interface type with the specified name and fields
    pub fn new(name: Cow<'a, str>, fields: &[Field<'a>]) -> InterfaceMeta<'a> {
        InterfaceMeta {
            name: name,
            description: None,
            fields: fields.to_vec(),
        }
    }

    /// Set the description of the type
    ///
    /// If a description was provided prior to calling this method, it will be overwritten.
    pub fn description(mut self, description: &str) -> InterfaceMeta<'a> {
        self.description = Some(description.to_owned());
        self
    }

    /// Wrap this interface type in a generic meta type
    pub fn into_meta(self) -> MetaType<'a> {
        MetaType::Interface(self)
    }
}

impl<'a> UnionMeta<'a> {
    /// Build a new union type with the specified name and possible types
    pub fn new(name: Cow<'a, str>, of_types: &[Type]) -> UnionMeta<'a> {
        UnionMeta {
            name: name,
            description: None,
            of_type_names: of_types
                .iter()
                .map(|t| t.innermost_name().to_owned())
                .collect(),
        }
    }

    /// Set the description of the type
    ///
    /// If a description was provided prior to calling this method, it will be overwritten.
    pub fn description(mut self, description: &str) -> UnionMeta<'a> {
        self.description = Some(description.to_owned());
        self
    }

    /// Wrap this union type in a generic meta type
    pub fn into_meta(self) -> MetaType<'a> {
        MetaType::Union(self)
    }
}

impl<'a> InputObjectMeta<'a> {
    /// Build a new input type with the specified name and input fields
    pub fn new<T: FromInputValue>(
        name: Cow<'a, str>,
        input_fields: &[Argument<'a>],
    ) -> InputObjectMeta<'a> {
        InputObjectMeta {
            name: name,
            description: None,
            input_fields: input_fields.to_vec(),
            try_parse_fn: Box::new(|v: &InputValue| {
                <T as FromInputValue>::from_input_value(v).is_some()
            }),
        }
    }

    /// Set the description of the type
    ///
    /// If a description was provided prior to calling this method, it will be overwritten.
    pub fn description(mut self, description: &str) -> InputObjectMeta<'a> {
        self.description = Some(description.to_owned());
        self
    }

    /// Wrap this union type in a generic meta type
    pub fn into_meta(self) -> MetaType<'a> {
        MetaType::InputObject(self)
    }
}

impl<'a> Field<'a> {
    /// Set the description of the field
    ///
    /// This overwrites the description if any was previously set.
    pub fn description(mut self, description: &str) -> Field<'a> {
        self.description = Some(description.to_owned());
        self
    }

    /// Add an argument to the field
    ///
    /// Arguments are unordered and can't contain duplicates by name.
    pub fn argument(mut self, argument: Argument<'a>) -> Field<'a> {
        match self.arguments {
            None => {
                self.arguments = Some(vec![argument]);
            }
            Some(ref mut args) => {
                args.push(argument);
            }
        };

        self
    }

    /// Set the deprecation reason
    ///
    /// This overwrites the deprecation reason if any was previously set.
    pub fn deprecated(mut self, reason: &str) -> Field<'a> {
        self.deprecation_reason = Some(reason.to_owned());
        self
    }
}

impl<'a> Argument<'a> {
    #[doc(hidden)]
    pub fn new(name: &str, arg_type: Type<'a>) -> Argument<'a> {
        Argument {
            name: name.to_owned(),
            description: None,
            arg_type: arg_type,
            default_value: None,
        }
    }

    /// Set the description of the argument
    ///
    /// This overwrites the description if any was previously set.
    pub fn description(mut self, description: &str) -> Argument<'a> {
        self.description = Some(description.to_owned());
        self
    }

    /// Set the default value of the argument
    ///
    /// This overwrites the description if any was previously set.
    pub fn default_value(mut self, default_value: InputValue) -> Argument<'a> {
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

impl<'a> fmt::Debug for ScalarMeta<'a> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("ScalarMeta")
            .field("name", &self.name)
            .field("description", &self.description)
            .finish()
    }
}

impl<'a> fmt::Debug for EnumMeta<'a> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("EnumMeta")
            .field("name", &self.name)
            .field("description", &self.description)
            .field("values", &self.values)
            .finish()
    }
}

impl<'a> fmt::Debug for InputObjectMeta<'a> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("InputObjectMeta")
            .field("name", &self.name)
            .field("description", &self.description)
            .field("input_fields", &self.input_fields)
            .finish()
    }
}
