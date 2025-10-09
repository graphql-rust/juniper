//! Types used to describe a `GraphQL` schema

use std::borrow::ToOwned;

use arcstr::ArcStr;
use derive_more::with_trait::Debug;

use crate::{
    FieldError, IntoFieldError,
    ast::{FromInputValue, InputValue, Type},
    parser::{ParseError, ScalarToken},
    schema::model::SchemaType,
    types::base::TypeKind,
    value::{DefaultScalarValue, ParseScalarValue},
};

/// Whether an item is deprecated, with context.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum DeprecationStatus {
    /// The field/variant is not deprecated.
    Current,

    /// The field/variant is deprecated, with an optional reason
    Deprecated(Option<ArcStr>),
}

impl DeprecationStatus {
    /// If this deprecation status indicates the item is deprecated.
    pub fn is_deprecated(&self) -> bool {
        match self {
            Self::Current => false,
            Self::Deprecated(_) => true,
        }
    }

    /// An optional reason for the deprecation, or none if `Current`.
    pub fn reason(&self) -> Option<&ArcStr> {
        match self {
            Self::Current => None,
            Self::Deprecated(rsn) => rsn.as_ref(),
        }
    }
}

/// Scalar type metadata
#[derive(Debug)]
pub struct ScalarMeta<S> {
    #[doc(hidden)]
    pub name: ArcStr,
    #[doc(hidden)]
    pub description: Option<ArcStr>,
    #[doc(hidden)]
    pub specified_by_url: Option<ArcStr>,
    #[debug(ignore)]
    pub(crate) try_parse_fn: InputValueParseFn<S>,
    #[debug(ignore)]
    pub(crate) parse_fn: ScalarTokenParseFn<S>,
}

impl<S> ScalarMeta<S> {
    /// Builds a new [`ScalarMeta`] type with the specified `name`.
    pub fn new<T>(name: impl Into<ArcStr>) -> Self
    where
        T: FromInputValue<S> + ParseScalarValue<S>,
        T::Error: IntoFieldError<S>,
    {
        Self {
            name: name.into(),
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
    pub fn description(mut self, description: impl Into<ArcStr>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Sets the [specification URL][0] for this [`ScalarMeta`] type.
    ///
    /// Overwrites any previously set [specification URL][0].
    ///
    /// [0]: https://spec.graphql.org/October2021#sec--specifiedBy
    #[must_use]
    pub fn specified_by_url(mut self, url: impl Into<ArcStr>) -> Self {
        self.specified_by_url = Some(url.into());
        self
    }

    /// Wraps this [`ScalarMeta`] type into a generic [`MetaType`].
    pub fn into_meta(self) -> MetaType<S> {
        MetaType::Scalar(self)
    }
}

/// Shortcut for an [`InputValue`] parsing function.
pub type InputValueParseFn<S> = for<'b> fn(&'b InputValue<S>) -> Result<(), FieldError<S>>;

/// Shortcut for a [`ScalarToken`] parsing function.
pub type ScalarTokenParseFn<S> = for<'b> fn(ScalarToken<'b>) -> Result<S, ParseError>;

/// List type metadata
#[derive(Debug)]
pub struct ListMeta {
    #[doc(hidden)]
    pub of_type: Type,

    #[doc(hidden)]
    pub expected_size: Option<usize>,
}

impl ListMeta {
    /// Builds a new [`ListMeta`] type by wrapping the specified [`Type`].
    ///
    /// Specifying `expected_size` will be used to ensure that values of this type will always match
    /// it.
    pub fn new(of_type: Type, expected_size: Option<usize>) -> Self {
        Self {
            of_type,
            expected_size,
        }
    }

    /// Wraps this [`ListMeta`] type into a generic [`MetaType`].
    pub fn into_meta<S>(self) -> MetaType<S> {
        MetaType::List(self)
    }
}

/// Nullable type metadata
#[derive(Debug)]
pub struct NullableMeta {
    #[doc(hidden)]
    pub of_type: Type,
}

impl NullableMeta {
    /// Builds a new [`NullableMeta`] type by wrapping the specified [`Type`].
    pub fn new(of_type: Type) -> Self {
        Self { of_type }
    }

    /// Wraps this [`NullableMeta`] type into a generic [`MetaType`].
    pub fn into_meta<S>(self) -> MetaType<S> {
        MetaType::Nullable(self)
    }
}

/// Object type metadata
#[derive(Debug)]
pub struct ObjectMeta<S> {
    #[doc(hidden)]
    pub name: ArcStr,
    #[doc(hidden)]
    pub description: Option<ArcStr>,
    #[doc(hidden)]
    pub fields: Vec<Field<S>>,
    #[doc(hidden)]
    pub interface_names: Vec<ArcStr>,
}

impl<S> ObjectMeta<S> {
    /// Builds a new [`ObjectMeta`] type with the specified `name` and `fields`.
    pub fn new(name: impl Into<ArcStr>, fields: &[Field<S>]) -> Self
    where
        S: Clone,
    {
        Self {
            name: name.into(),
            description: None,
            fields: fields.to_vec(),
            interface_names: vec![],
        }
    }

    /// Sets the `description` of this [`ObjectMeta`] type.
    ///
    /// Overwrites any previously set description.
    #[must_use]
    pub fn description(mut self, description: impl Into<ArcStr>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Sets the `interfaces` this [`ObjectMeta`] type implements.
    ///
    /// Overwrites any previously set list of interfaces.
    #[must_use]
    pub fn interfaces(mut self, interfaces: &[Type]) -> Self {
        self.interface_names = interfaces
            .iter()
            .map(|t| t.innermost_name().into())
            .collect();
        self
    }

    /// Wraps this [`ObjectMeta`] type into a generic [`MetaType`].
    pub fn into_meta(self) -> MetaType<S> {
        MetaType::Object(self)
    }
}

/// Enum type metadata
#[derive(Debug)]
pub struct EnumMeta<S> {
    #[doc(hidden)]
    pub name: ArcStr,
    #[doc(hidden)]
    pub description: Option<ArcStr>,
    #[doc(hidden)]
    pub values: Vec<EnumValue>,
    #[debug(ignore)]
    pub(crate) try_parse_fn: InputValueParseFn<S>,
}

impl<S> EnumMeta<S> {
    /// Builds a new [`EnumMeta`] type with the specified `name` and possible `values`.
    pub fn new<T>(name: impl Into<ArcStr>, values: &[EnumValue]) -> Self
    where
        T: FromInputValue<S>,
        T::Error: IntoFieldError<S>,
    {
        Self {
            name: name.into(),
            description: None,
            values: values.to_owned(),
            try_parse_fn: try_parse_fn::<S, T>,
        }
    }

    /// Sets the `description` of this [`EnumMeta`] type.
    ///
    /// Overwrites any previously set description.
    #[must_use]
    pub fn description(mut self, description: impl Into<ArcStr>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Wraps this [`EnumMeta`] type into a generic [`MetaType`].
    pub fn into_meta(self) -> MetaType<S> {
        MetaType::Enum(self)
    }
}

/// Interface type metadata
#[derive(Debug)]
pub struct InterfaceMeta<S> {
    #[doc(hidden)]
    pub name: ArcStr,
    #[doc(hidden)]
    pub description: Option<ArcStr>,
    #[doc(hidden)]
    pub fields: Vec<Field<S>>,
    #[doc(hidden)]
    pub interface_names: Vec<ArcStr>,
}

impl<S> InterfaceMeta<S> {
    /// Builds a new [`InterfaceMeta`] type with the specified `name` and `fields`.
    pub fn new(name: impl Into<ArcStr>, fields: &[Field<S>]) -> Self
    where
        S: Clone,
    {
        Self {
            name: name.into(),
            description: None,
            fields: fields.to_vec(),
            interface_names: Vec::new(),
        }
    }

    /// Sets the `description` of this [`InterfaceMeta`] type.
    ///
    /// Overwrites any previously set description.
    #[must_use]
    pub fn description(mut self, description: impl Into<ArcStr>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Sets the `interfaces` this [`InterfaceMeta`] interface implements.
    ///
    /// Overwrites any previously set list of interfaces.
    #[must_use]
    pub fn interfaces(mut self, interfaces: &[Type]) -> Self {
        self.interface_names = interfaces
            .iter()
            .map(|t| t.innermost_name().into())
            .collect();
        self
    }

    /// Wraps this [`InterfaceMeta`] type into a generic [`MetaType`].
    pub fn into_meta(self) -> MetaType<S> {
        MetaType::Interface(self)
    }
}

/// Union type metadata
#[derive(Debug)]
pub struct UnionMeta {
    #[doc(hidden)]
    pub name: ArcStr,
    #[doc(hidden)]
    pub description: Option<ArcStr>,
    #[doc(hidden)]
    pub of_type_names: Vec<ArcStr>,
}

impl UnionMeta {
    /// Builds a new [`UnionMeta`] type with the specified `name` and possible [`Type`]s.
    pub fn new(name: impl Into<ArcStr>, of_types: &[Type]) -> Self {
        Self {
            name: name.into(),
            description: None,
            of_type_names: of_types.iter().map(|t| t.innermost_name().into()).collect(),
        }
    }

    /// Sets the `description` of this [`UnionMeta`] type.
    ///
    /// Overwrites any previously set description.
    #[must_use]
    pub fn description(mut self, description: impl Into<ArcStr>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Wraps this [`UnionMeta`] type into a generic [`MetaType`].
    pub fn into_meta<S>(self) -> MetaType<S> {
        MetaType::Union(self)
    }
}

/// Input object metadata
#[derive(Debug)]
pub struct InputObjectMeta<S> {
    #[doc(hidden)]
    pub name: ArcStr,
    #[doc(hidden)]
    pub description: Option<ArcStr>,
    #[doc(hidden)]
    pub input_fields: Vec<Argument<S>>,
    #[doc(hidden)]
    pub is_one_of: bool,
    #[debug(ignore)]
    pub(crate) try_parse_fn: InputValueParseFn<S>,
}

impl<S> InputObjectMeta<S> {
    /// Builds a new [`InputObjectMeta`] type with the specified `name` and `input_fields`.
    pub fn new<T>(name: impl Into<ArcStr>, input_fields: &[Argument<S>]) -> Self
    where
        T: FromInputValue<S>,
        T::Error: IntoFieldError<S>,
        S: Clone,
    {
        Self {
            name: name.into(),
            description: None,
            input_fields: input_fields.to_vec(),
            is_one_of: false,
            try_parse_fn: try_parse_fn::<S, T>,
        }
    }

    /// Sets the `description` of this [`InputObjectMeta`] type.
    ///
    /// Overwrites any previously set description.
    #[must_use]
    pub fn description(mut self, description: impl Into<ArcStr>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Marks this [`InputObjectMeta`] type as [`@oneOf`].
    ///
    /// [`@oneOf`]: https://spec.graphql.org/September2025#sec--oneOf
    #[must_use]
    pub fn one_of(mut self) -> Self {
        self.is_one_of = true;
        self
    }

    /// Wraps this [`InputObjectMeta`] type into a generic [`MetaType`].
    pub fn into_meta(self) -> MetaType<S> {
        MetaType::InputObject(self)
    }
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

/// Metadata for a field
#[derive(Debug, Clone)]
pub struct Field<S> {
    #[doc(hidden)]
    pub name: ArcStr,
    #[doc(hidden)]
    pub description: Option<ArcStr>,
    #[doc(hidden)]
    pub arguments: Option<Vec<Argument<S>>>,
    #[doc(hidden)]
    pub field_type: Type,
    #[doc(hidden)]
    pub deprecation_status: DeprecationStatus,
}

impl<S> Field<S> {
    /// Sets the `description` of this [`Field`].
    ///
    /// Overwrites any previously set description.
    #[must_use]
    pub fn description(mut self, description: impl Into<ArcStr>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Adds an `argument` to this [`Field`].
    ///
    /// Arguments are unordered and can't contain duplicates by name.
    #[must_use]
    pub fn argument(mut self, argument: Argument<S>) -> Self {
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

    /// Indicates whether this [`Field`] is GraphQL built-in.
    #[must_use]
    pub fn is_builtin(&self) -> bool {
        // "used exclusively by GraphQL’s introspection system"
        self.name.starts_with("__")
    }

    /// Sets this [`Field`] as deprecated with an optional `reason`.
    ///
    /// Overwrites any previously set deprecation reason.
    #[must_use]
    pub fn deprecated(mut self, reason: Option<impl Into<ArcStr>>) -> Self {
        self.deprecation_status = DeprecationStatus::Deprecated(reason.map(Into::into));
        self
    }
}

/// Metadata for an argument to a field
#[derive(Debug, Clone)]
pub struct Argument<S> {
    #[doc(hidden)]
    pub name: ArcStr,
    #[doc(hidden)]
    pub description: Option<ArcStr>,
    #[doc(hidden)]
    pub arg_type: Type,
    #[doc(hidden)]
    pub default_value: Option<InputValue<S>>,
    #[doc(hidden)]
    pub deprecation_status: DeprecationStatus,
}

impl<S> Argument<S> {
    /// Builds a new [`Argument`] of the given [`Type`] with the given `name`.
    pub fn new(name: impl Into<ArcStr>, arg_type: Type) -> Self {
        Self {
            name: name.into(),
            description: None,
            arg_type,
            default_value: None,
            deprecation_status: DeprecationStatus::Current,
        }
    }

    /// Sets the `description` of this [`Argument`].
    ///
    /// Overwrites any previously set description.
    #[must_use]
    pub fn description(mut self, description: impl Into<ArcStr>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Indicates whether this [`Argument`] is GraphQL built-in.
    #[must_use]
    pub fn is_builtin(&self) -> bool {
        // "used exclusively by GraphQL’s introspection system"
        self.name.starts_with("__")
    }

    /// Sets the default value of this [`Argument`].
    ///
    /// Overwrites any previously set default value.
    #[must_use]
    pub fn default_value(mut self, val: InputValue<S>) -> Self {
        self.default_value = Some(val);
        self
    }

    /// Sets this [`Argument`] as deprecated with an optional `reason`.
    ///
    /// Overwrites any previously set deprecation reason.
    #[must_use]
    pub fn deprecated(mut self, reason: Option<impl Into<ArcStr>>) -> Self {
        self.deprecation_status = DeprecationStatus::Deprecated(reason.map(Into::into));
        self
    }
}

/// Metadata for a single value in an enum
#[derive(Debug, Clone)]
pub struct EnumValue {
    /// The name of the enum value
    ///
    /// This is the string literal representation of the enum in responses.
    pub name: ArcStr,

    /// The optional description of the enum value.
    ///
    /// Note: this is not the description of the enum itself; it's the
    /// description of this enum _value_.
    pub description: Option<ArcStr>,

    /// Whether the field is deprecated or not, with an optional reason.
    pub deprecation_status: DeprecationStatus,
}

impl EnumValue {
    /// Constructs a new [`EnumValue`] with the provided `name`.
    pub fn new(name: impl Into<ArcStr>) -> Self {
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
    pub fn description(mut self, description: impl Into<ArcStr>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Sets this [`EnumValue`] as deprecated with an optional `reason`.
    ///
    /// Overwrites any previously set deprecation reason.
    #[must_use]
    pub fn deprecated(mut self, reason: Option<impl Into<ArcStr>>) -> Self {
        self.deprecation_status = DeprecationStatus::Deprecated(reason.map(Into::into));
        self
    }
}

/// Generic type metadata
#[derive(Debug)]
pub enum MetaType<S = DefaultScalarValue> {
    #[doc(hidden)]
    Scalar(ScalarMeta<S>),
    #[doc(hidden)]
    List(ListMeta),
    #[doc(hidden)]
    Nullable(NullableMeta),
    #[doc(hidden)]
    Object(ObjectMeta<S>),
    #[doc(hidden)]
    Enum(EnumMeta<S>),
    #[doc(hidden)]
    Interface(InterfaceMeta<S>),
    #[doc(hidden)]
    Union(UnionMeta),
    #[doc(hidden)]
    InputObject(InputObjectMeta<S>),
    #[doc(hidden)]
    Placeholder(PlaceholderMeta),
}

impl<S> MetaType<S> {
    /// Returns the name of the represented type, if applicable.
    ///
    /// [Lists][`ListMeta`], [`null`ables][`NullableMeta`] and [placeholders][`PlaceholderMeta`]
    /// don't have a name.
    pub fn name(&self) -> Option<&ArcStr> {
        match self {
            Self::Enum(EnumMeta { name, .. })
            | Self::InputObject(InputObjectMeta { name, .. })
            | Self::Interface(InterfaceMeta { name, .. })
            | Self::Object(ObjectMeta { name, .. })
            | Self::Scalar(ScalarMeta { name, .. })
            | Self::Union(UnionMeta { name, .. }) => Some(name),
            Self::List(..) | Self::Nullable(..) | Self::Placeholder(..) => None,
        }
    }

    /// Returns the description of the represented type, if applicable.
    ///
    /// [Lists][`ListMeta`], [`null`ables][`NullableMeta`] and [placeholders][`PlaceholderMeta`]
    /// don't have a description.
    pub fn description(&self) -> Option<&ArcStr> {
        match self {
            Self::Enum(EnumMeta { description, .. })
            | Self::InputObject(InputObjectMeta { description, .. })
            | Self::Interface(InterfaceMeta { description, .. })
            | Self::Object(ObjectMeta { description, .. })
            | Self::Scalar(ScalarMeta { description, .. })
            | Self::Union(UnionMeta { description, .. }) => description.as_ref(),
            Self::List(..) | Self::Nullable(..) | Self::Placeholder(..) => None,
        }
    }

    /// Returns the [specification URL][0] of the represented type, if applicable.
    ///
    /// Only custom GraphQL scalars can have a [specification URL][0].
    ///
    /// [0]: https://spec.graphql.org/October2021#sec--specifiedBy
    pub fn specified_by_url(&self) -> Option<&ArcStr> {
        match self {
            Self::Scalar(ScalarMeta {
                specified_by_url, ..
            }) => specified_by_url.as_ref(),
            Self::Enum(..)
            | Self::InputObject(..)
            | Self::Interface(..)
            | Self::List(..)
            | Self::Nullable(..)
            | Self::Object(..)
            | Self::Placeholder(..)
            | Self::Union(..) => None,
        }
    }

    /// Construct a [`TypeKind`] out of this [`MetaType`].
    ///
    /// # Panics
    ///
    /// If this is [`MetaType::Nullable`] or [``MetaType::Placeholder`].
    pub fn type_kind(&self) -> TypeKind {
        match self {
            Self::Scalar(..) => TypeKind::Scalar,
            Self::List(..) => TypeKind::List,
            Self::Nullable(..) => panic!("сan't take `type_kind` of `MetaType::Nullable`"),
            Self::Object(..) => TypeKind::Object,
            Self::Enum(..) => TypeKind::Enum,
            Self::Interface(..) => TypeKind::Interface,
            Self::Union(..) => TypeKind::Union,
            Self::InputObject(..) => TypeKind::InputObject,
            Self::Placeholder(..) => panic!("сan't take `type_kind` of `MetaType::Placeholder`"),
        }
    }

    /// Returns a [`Field`]'s metadata by its `name`.
    ///
    /// Only [objects][`ObjectMeta`] and [interfaces][`InterfaceMeta`] have fields.
    pub fn field_by_name(&self, name: &str) -> Option<&Field<S>> {
        match self {
            Self::Interface(InterfaceMeta { fields, .. })
            | Self::Object(ObjectMeta { fields, .. }) => fields.iter().find(|f| f.name == name),
            Self::Enum(..)
            | Self::InputObject(..)
            | Self::List(..)
            | Self::Nullable(..)
            | Self::Placeholder(..)
            | Self::Scalar(..)
            | Self::Union(..) => None,
        }
    }

    /// Returns an input field's metadata by its `name`.
    ///
    /// Only [input objects][`InputObjectMeta`] have input fields.
    pub fn input_field_by_name(&self, name: &str) -> Option<&Argument<S>> {
        match self {
            Self::InputObject(InputObjectMeta { input_fields, .. }) => {
                input_fields.iter().find(|f| f.name == name)
            }
            Self::Enum(..)
            | Self::Interface(..)
            | Self::List(..)
            | Self::Nullable(..)
            | Self::Object(..)
            | Self::Placeholder(..)
            | Self::Scalar(..)
            | Self::Union(..) => None,
        }
    }

    /// Construct a [`Type`] literal out of this [`MetaType`].
    pub fn as_type(&self) -> Type {
        match self {
            Self::Enum(EnumMeta { name, .. })
            | Self::InputObject(InputObjectMeta { name, .. })
            | Self::Interface(InterfaceMeta { name, .. })
            | Self::Object(ObjectMeta { name, .. })
            | Self::Scalar(ScalarMeta { name, .. })
            | Self::Union(UnionMeta { name, .. }) => Type::nullable(name.clone()).wrap_non_null(),
            Self::List(ListMeta {
                of_type,
                expected_size,
            }) => of_type.clone().wrap_list(*expected_size).wrap_non_null(),
            Self::Nullable(NullableMeta { of_type }) => of_type.clone().into_nullable(),
            Self::Placeholder(PlaceholderMeta { of_type }) => of_type.clone(),
        }
    }

    /// Returns the [`InputValueParseFn`] of the represented type, if applicable.
    ///
    /// Only [scalars][`ScalarMeta`], [enums][`EnumMeta`] and [input objects][`InputObjectMeta`]
    /// have an [`InputValueParseFn`].
    pub fn input_value_parse_fn(&self) -> Option<InputValueParseFn<S>> {
        match self {
            Self::Enum(EnumMeta { try_parse_fn, .. })
            | Self::InputObject(InputObjectMeta { try_parse_fn, .. })
            | Self::Scalar(ScalarMeta { try_parse_fn, .. }) => Some(*try_parse_fn),
            Self::Interface(..)
            | Self::List(..)
            | Self::Nullable(..)
            | Self::Object(..)
            | Self::Placeholder(..)
            | Self::Union(..) => None,
        }
    }

    /// Indicates whether the represented type is a composite one.
    ///
    /// [Objects][`ObjectMeta`], [interfaces][`InterfaceMeta`] and [unions][`UnionMeta`] are
    /// composite types.
    pub fn is_composite(&self) -> bool {
        matches!(
            self,
            Self::Interface(..) | Self::Object(..) | Self::Union(..)
        )
    }

    /// Indicates whether the represented type can occur in leaf positions of queries.
    ///
    /// Only [enums][`EnumMeta`] and [scalars][`ScalarMeta`] are leaf types.
    pub fn is_leaf(&self) -> bool {
        matches!(self, Self::Enum(..) | Self::Scalar(..))
    }

    /// Indicates whether the represented type is abstract.
    ///
    /// Only [interfaces][`InterfaceMeta`] and [unions][`UnionMeta`] are abstract types.
    pub fn is_abstract(&self) -> bool {
        matches!(self, Self::Interface(..) | Self::Union(..))
    }

    /// Indicates whether the represented type can be used in input positions (e.g. arguments or
    /// variables).
    ///
    /// Only [scalars][`ScalarMeta`], [enums][`EnumMeta`] and [input objects][`InputObjectMeta`] are
    /// input types.
    pub fn is_input(&self) -> bool {
        matches!(
            self,
            Self::Enum(..) | Self::InputObject(..) | Self::Scalar(..)
        )
    }

    /// Indicates whether the represented type is GraphQL built-in.
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

    pub(crate) fn fields<'s>(&self, schema: &'s SchemaType<S>) -> Option<Vec<&'s Field<S>>> {
        schema
            .lookup_type(&self.as_type())
            .and_then(|tpe| match tpe {
                Self::Interface(i) => Some(i.fields.iter().collect()),
                Self::Object(o) => Some(o.fields.iter().collect()),
                Self::Union(u) => Some(
                    u.of_type_names
                        .iter()
                        .filter_map(|n| schema.concrete_type_by_name(n))
                        .filter_map(|t| t.fields(schema))
                        .flatten()
                        .collect(),
                ),
                Self::Enum(..)
                | Self::InputObject(..)
                | Self::List(..)
                | Self::Nullable(..)
                | Self::Placeholder(..)
                | Self::Scalar(..) => None,
            })
    }
}

fn try_parse_fn<S, T>(v: &InputValue<S>) -> Result<(), FieldError<S>>
where
    T: FromInputValue<S>,
    T::Error: IntoFieldError<S>,
{
    T::from_input_value(v)
        .map_err(T::Error::into_field_error)
        .map(drop)
}
