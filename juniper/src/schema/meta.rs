//! Types used to describe a `GraphQL` schema

use juniper::IntoFieldError;
use std::{borrow::Cow, fmt};

use crate::{
    ast::{FromInputValue, InputValue, Type},
    parser::{ParseError, ScalarToken},
    schema::model::SchemaType,
    types::base::TypeKind,
    value::{DefaultScalarValue, ParseScalarValue},
    FieldError,
};

/// Whether an item is deprecated, with context.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum DeprecationStatus {
    /// The field/variant is not deprecated.
    Current,
    /// The field/variant is deprecated, with an optional reason
    Deprecated(Option<String>),
}

impl DeprecationStatus {
    /// If this deprecation status indicates the item is deprecated.
    pub fn is_deprecated(&self) -> bool {
        match self {
            DeprecationStatus::Current => false,
            DeprecationStatus::Deprecated(_) => true,
        }
    }

    /// An optional reason for the deprecation, or none if `Current`.
    pub fn reason(&self) -> Option<&str> {
        match self {
            DeprecationStatus::Current => None,
            DeprecationStatus::Deprecated(rsn) => rsn.as_deref(),
        }
    }
}

/// Scalar type metadata
pub struct ScalarMeta<'a, S> {
    #[doc(hidden)]
    pub name: Cow<'a, str>,
    #[doc(hidden)]
    pub description: Option<String>,
    #[doc(hidden)]
    pub specified_by_url: Option<Cow<'a, str>>,
    pub(crate) try_parse_fn: InputValueParseFn<S>,
    pub(crate) parse_fn: ScalarTokenParseFn<S>,
}

/// Shortcut for an [`InputValue`] parsing function.
pub type InputValueParseFn<S> = for<'b> fn(&'b InputValue<S>) -> Result<(), FieldError<S>>;

/// Shortcut for a [`ScalarToken`] parsing function.
pub type ScalarTokenParseFn<S> = for<'b> fn(ScalarToken<'b>) -> Result<S, ParseError>;

/// List type metadata
#[derive(Debug)]
pub struct ListMeta<'a> {
    #[doc(hidden)]
    pub of_type: Type<'a>,

    #[doc(hidden)]
    pub expected_size: Option<usize>,
}

/// Nullable type metadata
#[derive(Debug)]
pub struct NullableMeta<'a> {
    #[doc(hidden)]
    pub of_type: Type<'a>,
}

/// Object type metadata
#[derive(Debug)]
pub struct ObjectMeta<'a, S> {
    #[doc(hidden)]
    pub name: Cow<'a, str>,
    #[doc(hidden)]
    pub description: Option<String>,
    #[doc(hidden)]
    pub fields: Vec<Field<'a, S>>,
    #[doc(hidden)]
    pub interface_names: Vec<String>,
}

/// Enum type metadata
pub struct EnumMeta<'a, S> {
    #[doc(hidden)]
    pub name: Cow<'a, str>,
    #[doc(hidden)]
    pub description: Option<String>,
    #[doc(hidden)]
    pub values: Vec<EnumValue>,
    pub(crate) try_parse_fn: InputValueParseFn<S>,
}

/// Interface type metadata
#[derive(Debug)]
pub struct InterfaceMeta<'a, S> {
    #[doc(hidden)]
    pub name: Cow<'a, str>,
    #[doc(hidden)]
    pub description: Option<String>,
    #[doc(hidden)]
    pub fields: Vec<Field<'a, S>>,
    #[doc(hidden)]
    pub interface_names: Vec<String>,
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
pub struct InputObjectMeta<'a, S> {
    #[doc(hidden)]
    pub name: Cow<'a, str>,
    #[doc(hidden)]
    pub description: Option<String>,
    #[doc(hidden)]
    pub input_fields: Vec<Argument<'a, S>>,
    pub(crate) try_parse_fn: InputValueParseFn<S>,
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
pub enum MetaType<'a, S = DefaultScalarValue> {
    #[doc(hidden)]
    Scalar(ScalarMeta<'a, S>),
    #[doc(hidden)]
    List(ListMeta<'a>),
    #[doc(hidden)]
    Nullable(NullableMeta<'a>),
    #[doc(hidden)]
    Object(ObjectMeta<'a, S>),
    #[doc(hidden)]
    Enum(EnumMeta<'a, S>),
    #[doc(hidden)]
    Interface(InterfaceMeta<'a, S>),
    #[doc(hidden)]
    Union(UnionMeta<'a>),
    #[doc(hidden)]
    InputObject(InputObjectMeta<'a, S>),
    #[doc(hidden)]
    Placeholder(PlaceholderMeta<'a>),
}

/// Metadata for a field
#[derive(Debug, Clone)]
pub struct Field<'a, S> {
    #[doc(hidden)]
    pub name: smartstring::alias::String,
    #[doc(hidden)]
    pub description: Option<String>,
    #[doc(hidden)]
    pub arguments: Option<Vec<Argument<'a, S>>>,
    #[doc(hidden)]
    pub field_type: Type<'a>,
    #[doc(hidden)]
    pub deprecation_status: DeprecationStatus,
}

impl<'a, S> Field<'a, S> {
    /// Returns true if the type is built-in to GraphQL.
    pub fn is_builtin(&self) -> bool {
        // "used exclusively by GraphQL’s introspection system"
        self.name.starts_with("__")
    }
}

/// Metadata for an argument to a field
#[derive(Debug, Clone)]
pub struct Argument<'a, S> {
    #[doc(hidden)]
    pub name: String,
    #[doc(hidden)]
    pub description: Option<String>,
    #[doc(hidden)]
    pub arg_type: Type<'a>,
    #[doc(hidden)]
    pub default_value: Option<InputValue<S>>,
}

impl<'a, S> Argument<'a, S> {
    /// Returns true if the type is built-in to GraphQL.
    pub fn is_builtin(&self) -> bool {
        // "used exclusively by GraphQL’s introspection system"
        self.name.starts_with("__")
    }
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
    /// Whether the field is deprecated or not, with an optional reason.
    pub deprecation_status: DeprecationStatus,
}

impl<'a, S> MetaType<'a, S> {
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
    pub fn description(&self) -> Option<&str> {
        match self {
            MetaType::Scalar(ScalarMeta { description, .. })
            | MetaType::Object(ObjectMeta { description, .. })
            | MetaType::Enum(EnumMeta { description, .. })
            | MetaType::Interface(InterfaceMeta { description, .. })
            | MetaType::Union(UnionMeta { description, .. })
            | MetaType::InputObject(InputObjectMeta { description, .. }) => description.as_deref(),
            _ => None,
        }
    }

    /// Accesses the [specification URL][0], if applicable.
    ///
    /// Only custom GraphQL scalars can have a [specification URL][0].
    ///
    /// [0]: https://spec.graphql.org/October2021#sec--specifiedBy
    pub fn specified_by_url(&self) -> Option<&str> {
        match self {
            Self::Scalar(ScalarMeta {
                specified_by_url, ..
            }) => specified_by_url.as_deref(),
            _ => None,
        }
    }

    /// Construct a `TypeKind` for a given type
    ///
    /// # Panics
    ///
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
    pub fn field_by_name(&self, name: &str) -> Option<&Field<S>> {
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
    pub fn input_field_by_name(&self, name: &str) -> Option<&Argument<S>> {
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
            MetaType::List(ListMeta {
                ref of_type,
                expected_size,
            }) => Type::NonNullList(Box::new(of_type.clone()), expected_size),
            MetaType::Nullable(NullableMeta { ref of_type }) => match *of_type {
                Type::NonNullNamed(ref inner) => Type::Named(inner.clone()),
                Type::NonNullList(ref inner, expected_size) => {
                    Type::List(inner.clone(), expected_size)
                }
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
    pub fn input_value_parse_fn(&self) -> Option<InputValueParseFn<S>> {
        match *self {
            MetaType::Scalar(ScalarMeta {
                ref try_parse_fn, ..
            })
            | MetaType::Enum(EnumMeta {
                ref try_parse_fn, ..
            })
            | MetaType::InputObject(InputObjectMeta {
                ref try_parse_fn, ..
            }) => Some(*try_parse_fn),
            _ => None,
        }
    }

    /// Returns true if the type is a composite type
    ///
    /// Objects, interfaces, and unions are composite.
    pub fn is_composite(&self) -> bool {
        matches!(
            *self,
            MetaType::Object(_) | MetaType::Interface(_) | MetaType::Union(_)
        )
    }

    /// Returns true if the type can occur in leaf positions in queries
    ///
    /// Only enums and scalars are leaf types.
    pub fn is_leaf(&self) -> bool {
        matches!(*self, MetaType::Enum(_) | MetaType::Scalar(_))
    }

    /// Returns true if the type is abstract
    ///
    /// Only interfaces and unions are abstract types.
    pub fn is_abstract(&self) -> bool {
        matches!(*self, MetaType::Interface(_) | MetaType::Union(_))
    }

    /// Returns true if the type can be used in input positions, e.g. arguments or variables
    ///
    /// Only scalars, enums, and input objects are input types.
    pub fn is_input(&self) -> bool {
        matches!(
            *self,
            MetaType::Scalar(_) | MetaType::Enum(_) | MetaType::InputObject(_)
        )
    }

    /// Returns true if the type is built-in to GraphQL.
    pub fn is_builtin(&self) -> bool {
        if let Some(name) = self.name() {
            // "used exclusively by GraphQL’s introspection system"
            {
                name.starts_with("__") ||
            // https://spec.graphql.org/October2021#sec-Scalars
            name == "Boolean" || name == "String" || name == "Int" || name == "Float" || name == "ID" ||
            // Our custom empty markers
            name == "_EmptyMutation" || name == "_EmptySubscription"
            }
        } else {
            false
        }
    }

    pub(crate) fn fields<'b>(&self, schema: &'b SchemaType<S>) -> Option<Vec<&'b Field<'b, S>>> {
        schema
            .lookup_type(&self.as_type())
            .and_then(|tpe| match *tpe {
                MetaType::Interface(ref i) => Some(i.fields.iter().collect()),
                MetaType::Object(ref o) => Some(o.fields.iter().collect()),
                MetaType::Union(ref u) => Some(
                    u.of_type_names
                        .iter()
                        .filter_map(|n| schema.concrete_type_by_name(n))
                        .filter_map(|t| t.fields(schema))
                        .flatten()
                        .collect(),
                ),
                _ => None,
            })
    }
}

impl<'a, S> ScalarMeta<'a, S> {
    /// Builds a new [`ScalarMeta`] type with the specified `name`.
    pub fn new<T>(name: Cow<'a, str>) -> Self
    where
        T: FromInputValue<S> + ParseScalarValue<S>,
        T::Error: IntoFieldError<S>,
    {
        Self {
            name,
            description: None,
            specified_by_url: None,
            try_parse_fn: try_parse_fn::<S, T>,
            parse_fn: <T as ParseScalarValue<S>>::from_str,
        }
    }

    /// Sets the `description` of this [`ScalarMeta`] type.
    ///
    /// Overwrites any previously set description.
    #[must_use]
    pub fn description(mut self, description: &str) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Sets the [specification URL][0] for this [`ScalarMeta`] type.
    ///
    /// Overwrites any previously set [specification URL][0].
    ///
    /// [0]: https://spec.graphql.org/October2021#sec--specifiedBy
    #[must_use]
    pub fn specified_by_url(mut self, url: impl Into<Cow<'a, str>>) -> Self {
        self.specified_by_url = Some(url.into());
        self
    }

    /// Wraps this [`ScalarMeta`] type into a generic [`MetaType`].
    pub fn into_meta(self) -> MetaType<'a, S> {
        MetaType::Scalar(self)
    }
}

impl<'a> ListMeta<'a> {
    /// Build a new [`ListMeta`] type by wrapping the specified [`Type`].
    ///
    /// Specifying `expected_size` will be used to ensure that values of this
    /// type will always match it.
    pub fn new(of_type: Type<'a>, expected_size: Option<usize>) -> Self {
        Self {
            of_type,
            expected_size,
        }
    }

    /// Wraps this [`ListMeta`] type into a generic [`MetaType`].
    pub fn into_meta<S>(self) -> MetaType<'a, S> {
        MetaType::List(self)
    }
}

impl<'a> NullableMeta<'a> {
    /// Build a new [`NullableMeta`] type by wrapping the specified [`Type`].
    pub fn new(of_type: Type<'a>) -> Self {
        Self { of_type }
    }

    /// Wraps this [`NullableMeta`] type into a generic [`MetaType`].
    pub fn into_meta<S>(self) -> MetaType<'a, S> {
        MetaType::Nullable(self)
    }
}

impl<'a, S> ObjectMeta<'a, S> {
    /// Build a new [`ObjectMeta`] type with the specified `name` and `fields`.
    pub fn new(name: Cow<'a, str>, fields: &[Field<'a, S>]) -> Self
    where
        S: Clone,
    {
        Self {
            name,
            description: None,
            fields: fields.to_vec(),
            interface_names: vec![],
        }
    }

    /// Sets the `description` of this [`ObjectMeta`] type.
    ///
    /// Overwrites any previously set description.
    #[must_use]
    pub fn description(mut self, description: &str) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the `interfaces` this [`ObjectMeta`] type implements.
    ///
    /// Overwrites any previously set list of interfaces.
    #[must_use]
    pub fn interfaces(mut self, interfaces: &[Type<'a>]) -> Self {
        self.interface_names = interfaces
            .iter()
            .map(|t| t.innermost_name().into())
            .collect();
        self
    }

    /// Wraps this [`ObjectMeta`] type into a generic [`MetaType`].
    pub fn into_meta(self) -> MetaType<'a, S> {
        MetaType::Object(self)
    }
}

impl<'a, S> EnumMeta<'a, S> {
    /// Build a new [`EnumMeta`] type with the specified `name` and possible
    /// `values`.
    pub fn new<T>(name: Cow<'a, str>, values: &[EnumValue]) -> Self
    where
        T: FromInputValue<S>,
        T::Error: IntoFieldError<S>,
    {
        Self {
            name,
            description: None,
            values: values.to_owned(),
            try_parse_fn: try_parse_fn::<S, T>,
        }
    }

    /// Sets the `description` of this [`EnumMeta`] type.
    ///
    /// Overwrites any previously set description.
    #[must_use]
    pub fn description(mut self, description: &str) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Wraps this [`EnumMeta`] type into a generic [`MetaType`].
    pub fn into_meta(self) -> MetaType<'a, S> {
        MetaType::Enum(self)
    }
}

impl<'a, S> InterfaceMeta<'a, S> {
    /// Builds a new [`InterfaceMeta`] type with the specified `name` and
    /// `fields`.
    pub fn new(name: Cow<'a, str>, fields: &[Field<'a, S>]) -> Self
    where
        S: Clone,
    {
        Self {
            name,
            description: None,
            fields: fields.to_vec(),
            interface_names: Vec::new(),
        }
    }

    /// Sets the `description` of this [`InterfaceMeta`] type.
    ///
    /// Overwrites any previously set description.
    #[must_use]
    pub fn description(mut self, description: &str) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Sets the `interfaces` this [`InterfaceMeta`] interface implements.
    ///
    /// Overwrites any previously set list of interfaces.
    #[must_use]
    pub fn interfaces(mut self, interfaces: &[Type<'a>]) -> Self {
        self.interface_names = interfaces
            .iter()
            .map(|t| t.innermost_name().into())
            .collect();
        self
    }

    /// Wraps this [`InterfaceMeta`] type into a generic [`MetaType`].
    pub fn into_meta(self) -> MetaType<'a, S> {
        MetaType::Interface(self)
    }
}

impl<'a> UnionMeta<'a> {
    /// Build a new [`UnionMeta`] type with the specified `name` and possible
    /// [`Type`]s.
    pub fn new(name: Cow<'a, str>, of_types: &[Type]) -> Self {
        Self {
            name,
            description: None,
            of_type_names: of_types.iter().map(|t| t.innermost_name().into()).collect(),
        }
    }

    /// Sets the `description` of this [`UnionMeta`] type.
    ///
    /// Overwrites any previously set description.
    #[must_use]
    pub fn description(mut self, description: &str) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Wraps this [`UnionMeta`] type into a generic [`MetaType`].
    pub fn into_meta<S>(self) -> MetaType<'a, S> {
        MetaType::Union(self)
    }
}

impl<'a, S> InputObjectMeta<'a, S> {
    /// Builds a new [`InputObjectMeta`] type with the specified `name` and
    /// `input_fields`.
    pub fn new<T>(name: Cow<'a, str>, input_fields: &[Argument<'a, S>]) -> Self
    where
        T: FromInputValue<S>,
        T::Error: IntoFieldError<S>,
        S: Clone,
    {
        Self {
            name,
            description: None,
            input_fields: input_fields.to_vec(),
            try_parse_fn: try_parse_fn::<S, T>,
        }
    }

    /// Set the `description` of this [`InputObjectMeta`] type.
    ///
    /// Overwrites any previously set description.
    #[must_use]
    pub fn description(mut self, description: &str) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Wraps this [`InputObjectMeta`] type into a generic [`MetaType`].
    pub fn into_meta(self) -> MetaType<'a, S> {
        MetaType::InputObject(self)
    }
}

impl<'a, S> Field<'a, S> {
    /// Set the `description` of this [`Field`].
    ///
    /// Overwrites any previously set description.
    #[must_use]
    pub fn description(mut self, description: &str) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Adds an `argument` to this [`Field`].
    ///
    /// Arguments are unordered and can't contain duplicates by name.
    #[must_use]
    pub fn argument(mut self, argument: Argument<'a, S>) -> Self {
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

    /// Sets this [`Field`] as deprecated with an optional `reason`.
    ///
    /// Overwrites any previously set deprecation reason.
    #[must_use]
    pub fn deprecated(mut self, reason: Option<&str>) -> Self {
        self.deprecation_status = DeprecationStatus::Deprecated(reason.map(Into::into));
        self
    }
}

impl<'a, S> Argument<'a, S> {
    /// Builds a new [`Argument`] of the given [`Type`] with the given `name`.
    pub fn new(name: &str, arg_type: Type<'a>) -> Self {
        Self {
            name: name.into(),
            description: None,
            arg_type,
            default_value: None,
        }
    }

    /// Sets the `description` of this [`Argument`].
    ///
    /// Overwrites any previously set description.
    #[must_use]
    pub fn description(mut self, description: &str) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the default value of this [`Argument`].
    ///
    /// Overwrites any previously set default value.
    #[must_use]
    pub fn default_value(mut self, val: InputValue<S>) -> Self {
        self.default_value = Some(val);
        self
    }
}

impl EnumValue {
    /// Constructs a new [`EnumValue`] with the provided `name`.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.into(),
            description: None,
            deprecation_status: DeprecationStatus::Current,
        }
    }

    /// Sets the `description` of this [`EnumValue`].
    ///
    /// Overwrites any previously set description.
    #[must_use]
    pub fn description(mut self, description: &str) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Sets this [`EnumValue`] as deprecated with an optional `reason`.
    ///
    /// Overwrites any previously set deprecation reason.
    #[must_use]
    pub fn deprecated(mut self, reason: Option<&str>) -> Self {
        self.deprecation_status = DeprecationStatus::Deprecated(reason.map(Into::into));
        self
    }
}

impl<'a, S: fmt::Debug> fmt::Debug for ScalarMeta<'a, S> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("ScalarMeta")
            .field("name", &self.name)
            .field("description", &self.description)
            .finish()
    }
}

impl<'a, S: fmt::Debug> fmt::Debug for EnumMeta<'a, S> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("EnumMeta")
            .field("name", &self.name)
            .field("description", &self.description)
            .field("values", &self.values)
            .finish()
    }
}

impl<'a, S: fmt::Debug> fmt::Debug for InputObjectMeta<'a, S> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("InputObjectMeta")
            .field("name", &self.name)
            .field("description", &self.description)
            .field("input_fields", &self.input_fields)
            .finish()
    }
}

fn try_parse_fn<S, T>(v: &InputValue<S>) -> Result<(), FieldError<S>>
where
    T: FromInputValue<S>,
    T::Error: IntoFieldError<S>,
{
    T::from_input_value(v)
        .map(drop)
        .map_err(T::Error::into_field_error)
}
