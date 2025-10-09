use std::collections::BTreeMap;

use graphql_parser::{Pos, schema};

use crate::{
    ast,
    schema::{meta, model::SchemaType, translate::SchemaTranslator},
    value::ScalarValue,
};

// TODO: Remove on upgrade to 0.4.1 version of `graphql-parser`.
mod for_minimal_versions_check_only {
    use void as _;
}

pub struct GraphQLParserTranslator;

impl<'a, S: 'a, T> From<&'a SchemaType<S>> for schema::Document<'a, T>
where
    S: ScalarValue,
    T: schema::Text<'a> + Default,
{
    fn from(input: &'a SchemaType<S>) -> schema::Document<'a, T> {
        GraphQLParserTranslator::translate_schema(input)
    }
}

impl<'a, T> SchemaTranslator<'a, schema::Document<'a, T>> for GraphQLParserTranslator
where
    T: schema::Text<'a> + Default,
{
    fn translate_schema<S>(input: &'a SchemaType<S>) -> schema::Document<'a, T>
    where
        S: ScalarValue + 'a,
    {
        let mut doc = schema::Document::default();

        // Translate type defs.
        let mut types = input
            .types
            .iter()
            .filter(|(_, meta)| !meta.is_builtin())
            .map(|(_, meta)| GraphQLParserTranslator::translate_meta(meta))
            .map(schema::Definition::TypeDefinition)
            .collect();
        doc.definitions.append(&mut types);

        doc.definitions.push(schema::Definition::SchemaDefinition(
            schema::SchemaDefinition {
                position: Pos::default(),
                directives: vec![],
                query: Some(input.query_type_name.as_str().into()),
                mutation: input.mutation_type_name.as_ref().map(|s| s.as_str().into()),
                subscription: input
                    .subscription_type_name
                    .as_ref()
                    .map(|s| s.as_str().into()),
            },
        ));

        doc
    }
}

impl GraphQLParserTranslator {
    fn translate_argument<'a, S, T>(input: &'a meta::Argument<S>) -> schema::InputValue<'a, T>
    where
        S: ScalarValue,
        T: schema::Text<'a>,
    {
        let meta::Argument {
            name,
            description,
            arg_type,
            default_value,
            deprecation_status,
        } = input;
        schema::InputValue {
            position: Pos::default(),
            description: description.as_deref().map(Into::into),
            name: name.as_str().into(),
            value_type: GraphQLParserTranslator::translate_type(arg_type),
            default_value: default_value
                .as_ref()
                .map(|x| GraphQLParserTranslator::translate_value(x)),
            directives: deprecation_directive(deprecation_status)
                .map(|d| vec![d])
                .unwrap_or_default(),
        }
    }

    fn translate_value<'a, S, T>(input: &'a ast::InputValue<S>) -> schema::Value<'a, T>
    where
        S: ScalarValue + 'a,
        T: schema::Text<'a>,
    {
        match input {
            ast::InputValue::Null => schema::Value::Null,
            ast::InputValue::Scalar(x) => {
                if let Some(v) = x.try_to_string() {
                    schema::Value::String(v)
                } else if let Some(v) = x.try_to_int() {
                    schema::Value::Int(v.into())
                } else if let Some(v) = x.try_to_float() {
                    schema::Value::Float(v)
                } else if let Some(v) = x.try_to_bool() {
                    schema::Value::Boolean(v)
                } else {
                    panic!("unknown argument type")
                }
            }
            ast::InputValue::Enum(x) => schema::Value::Enum(x.as_str().into()),
            ast::InputValue::Variable(x) => schema::Value::Variable(x.as_str().into()),
            ast::InputValue::List(x) => schema::Value::List(
                x.iter()
                    .map(|s| GraphQLParserTranslator::translate_value(&s.item))
                    .collect(),
            ),
            ast::InputValue::Object(x) => {
                let mut fields = BTreeMap::new();
                x.iter().for_each(|(name_span, value_span)| {
                    fields.insert(
                        name_span.item.as_str().into(),
                        GraphQLParserTranslator::translate_value(&value_span.item),
                    );
                });
                schema::Value::Object(fields)
            }
        }
    }

    fn translate_type<'a, T>(input: &'a ast::Type) -> schema::Type<'a, T>
    where
        T: schema::Text<'a>,
    {
        let mut ty = schema::Type::NamedType(input.innermost_name().into());
        for m in input.modifiers() {
            ty = match m {
                ast::TypeModifier::NonNull => schema::Type::NonNullType(ty.into()),
                ast::TypeModifier::List(..) => schema::Type::ListType(ty.into()),
            };
        }
        ty
    }

    fn translate_meta<'a, S, T>(input: &'a meta::MetaType<S>) -> schema::TypeDefinition<'a, T>
    where
        S: ScalarValue,
        T: schema::Text<'a>,
    {
        match input {
            meta::MetaType::Scalar(meta::ScalarMeta {
                name,
                description,
                specified_by_url,
                try_parse_fn: _,
                parse_fn: _,
            }) => schema::TypeDefinition::Scalar(schema::ScalarType {
                position: Pos::default(),
                description: description.as_deref().map(Into::into),
                name: name.as_str().into(),
                directives: specified_by_url
                    .as_deref()
                    .map(|url| vec![specified_by_url_directive(url)])
                    .unwrap_or_default(),
            }),
            meta::MetaType::Enum(meta::EnumMeta {
                name,
                description,
                values,
                try_parse_fn: _,
            }) => schema::TypeDefinition::Enum(schema::EnumType {
                position: Pos::default(),
                description: description.as_deref().map(Into::into),
                name: name.as_str().into(),
                directives: vec![],
                values: values
                    .iter()
                    .map(GraphQLParserTranslator::translate_enum_value)
                    .collect(),
            }),
            meta::MetaType::Union(meta::UnionMeta {
                name,
                description,
                of_type_names,
            }) => schema::TypeDefinition::Union(schema::UnionType {
                position: Pos::default(),
                description: description.as_deref().map(Into::into),
                name: name.as_str().into(),
                directives: vec![],
                types: of_type_names.iter().map(|s| s.as_str().into()).collect(),
            }),
            meta::MetaType::Interface(meta::InterfaceMeta {
                name,
                description,
                fields,
                interface_names,
            }) => schema::TypeDefinition::Interface(schema::InterfaceType {
                position: Pos::default(),
                description: description.as_deref().map(Into::into),
                name: name.as_str().into(),
                implements_interfaces: interface_names.iter().map(|s| s.as_str().into()).collect(),
                directives: vec![],
                fields: fields
                    .iter()
                    .filter(|x| !x.is_builtin())
                    .map(GraphQLParserTranslator::translate_field)
                    .collect(),
            }),
            meta::MetaType::InputObject(meta::InputObjectMeta {
                name,
                description,
                input_fields,
                is_one_of,
                try_parse_fn: _,
            }) => schema::TypeDefinition::InputObject(schema::InputObjectType {
                position: Pos::default(),
                description: description.as_deref().map(Into::into),
                name: name.as_str().into(),
                directives: is_one_of
                    .then(|| vec![one_of_directive()])
                    .unwrap_or_default(),
                fields: input_fields
                    .iter()
                    .filter(|x| !x.is_builtin())
                    .map(GraphQLParserTranslator::translate_argument)
                    .collect(),
            }),
            meta::MetaType::Object(meta::ObjectMeta {
                name,
                description,
                fields,
                interface_names,
            }) => schema::TypeDefinition::Object(schema::ObjectType {
                position: Pos::default(),
                description: description.as_deref().map(Into::into),
                name: name.as_str().into(),
                directives: vec![],
                fields: fields
                    .iter()
                    .filter(|x| !x.is_builtin())
                    .map(GraphQLParserTranslator::translate_field)
                    .collect(),
                implements_interfaces: interface_names.iter().map(|s| s.as_str().into()).collect(),
            }),
            _ => panic!("unknown `MetaType` when translating"),
        }
    }

    fn translate_enum_value<'a, T>(input: &'a meta::EnumValue) -> schema::EnumValue<'a, T>
    where
        T: schema::Text<'a>,
    {
        let meta::EnumValue {
            name,
            description,
            deprecation_status,
        } = input;
        schema::EnumValue {
            position: Pos::default(),
            name: name.as_str().into(),
            description: description.as_deref().map(Into::into),
            directives: deprecation_directive(deprecation_status)
                .map(|d| vec![d])
                .unwrap_or_default(),
        }
    }

    fn translate_field<'a, S, T>(input: &'a meta::Field<S>) -> schema::Field<'a, T>
    where
        S: ScalarValue + 'a,
        T: schema::Text<'a>,
    {
        let meta::Field {
            name,
            description,
            arguments,
            field_type,
            deprecation_status,
        } = input;
        schema::Field {
            position: Pos::default(),
            name: name.as_str().into(),
            description: description.as_deref().map(Into::into),
            directives: deprecation_directive(deprecation_status)
                .map(|d| vec![d])
                .unwrap_or_default(),
            field_type: GraphQLParserTranslator::translate_type(field_type),
            arguments: arguments
                .as_ref()
                .map(|a| {
                    a.iter()
                        .filter(|x| !x.is_builtin())
                        .map(|x| GraphQLParserTranslator::translate_argument(x))
                        .collect()
                })
                .unwrap_or_default(),
        }
    }
}

/// Forms a [`@deprecated(reason:)`] [`schema::Directive`] out of the provided
/// [`meta::DeprecationStatus`].
///
/// [`@deprecated(reason:)`]: https://spec.graphql.org/September2025#sec--deprecated
fn deprecation_directive<'a, T>(
    status: &meta::DeprecationStatus,
) -> Option<schema::Directive<'a, T>>
where
    T: schema::Text<'a>,
{
    match status {
        meta::DeprecationStatus::Current => None,
        meta::DeprecationStatus::Deprecated(reason) => Some(schema::Directive {
            position: Pos::default(),
            name: "deprecated".into(),
            arguments: reason
                .as_ref()
                .map(|rsn| vec![("reason".into(), schema::Value::String(rsn.as_str().into()))])
                .unwrap_or_default(),
        }),
    }
}

/// Forms a [`@oneOf`] [`schema::Directive`].
///
/// [`@oneOf`]: https://spec.graphql.org/September2025#sec--oneOf
fn one_of_directive<'a, T>() -> schema::Directive<'a, T>
where
    T: schema::Text<'a>,
{
    schema::Directive {
        position: Pos::default(),
        name: "oneOf".into(),
        arguments: vec![],
    }
}

/// Forms a `@specifiedBy(url:)` [`schema::Directive`] out of the provided `url`.
///
/// [`@specifiedBy(url:)`]: https://spec.graphql.org/September2025#sec--specifiedBy
fn specified_by_url_directive<'a, T>(url: &str) -> schema::Directive<'a, T>
where
    T: schema::Text<'a>,
{
    schema::Directive {
        position: Pos::default(),
        name: "specifiedBy".into(),
        arguments: vec![("url".into(), schema::Value::String(url.into()))],
    }
}

/// Sorts the provided [`schema::Document`] in the "type-then-name" manner.
pub(crate) fn sort_schema_document<'a, T>(document: &mut schema::Document<'a, T>)
where
    T: schema::Text<'a>,
{
    document.definitions.sort_by(move |a, b| {
        let type_cmp = sort_value::by_type(a).cmp(&sort_value::by_type(b));
        let name_cmp = sort_value::by_is_directive(a)
            .cmp(&sort_value::by_is_directive(b))
            .then(sort_value::by_name(a).cmp(&sort_value::by_name(b)))
            .then(sort_value::by_directive(a).cmp(&sort_value::by_directive(b)));
        type_cmp.then(name_cmp)
    })
}

/// Evaluation of a [`schema::Definition`] weights for sorting.
mod sort_value {
    use graphql_parser::schema::{self, Definition, TypeDefinition, TypeExtension};

    /// Returns a [`Definition`] sorting weight by its type.
    pub(super) fn by_type<'a, T>(definition: &Definition<'a, T>) -> u8
    where
        T: schema::Text<'a>,
    {
        match definition {
            Definition::SchemaDefinition(_) => 0,
            Definition::DirectiveDefinition(_) => 1,
            Definition::TypeDefinition(t) => match t {
                TypeDefinition::Enum(_) => 2,
                TypeDefinition::InputObject(_) => 4,
                TypeDefinition::Interface(_) => 6,
                TypeDefinition::Scalar(_) => 8,
                TypeDefinition::Object(_) => 10,
                TypeDefinition::Union(_) => 12,
            },
            Definition::TypeExtension(e) => match e {
                TypeExtension::Enum(_) => 3,
                TypeExtension::InputObject(_) => 5,
                TypeExtension::Interface(_) => 7,
                TypeExtension::Scalar(_) => 9,
                TypeExtension::Object(_) => 11,
                TypeExtension::Union(_) => 13,
            },
        }
    }

    /// Returns a [`Definition`] sorting weight by its name.
    pub(super) fn by_name<'b, 'a, T>(definition: &'b Definition<'a, T>) -> Option<&'b T::Value>
    where
        T: schema::Text<'a>,
    {
        match definition {
            Definition::SchemaDefinition(_) => None,
            Definition::DirectiveDefinition(d) => Some(&d.name),
            Definition::TypeDefinition(t) => match t {
                TypeDefinition::Enum(d) => Some(&d.name),
                TypeDefinition::InputObject(d) => Some(&d.name),
                TypeDefinition::Interface(d) => Some(&d.name),
                TypeDefinition::Scalar(d) => Some(&d.name),
                TypeDefinition::Object(d) => Some(&d.name),
                TypeDefinition::Union(d) => Some(&d.name),
            },
            Definition::TypeExtension(e) => match e {
                TypeExtension::Enum(d) => Some(&d.name),
                TypeExtension::InputObject(d) => Some(&d.name),
                TypeExtension::Interface(d) => Some(&d.name),
                TypeExtension::Scalar(d) => Some(&d.name),
                TypeExtension::Object(d) => Some(&d.name),
                TypeExtension::Union(d) => Some(&d.name),
            },
        }
    }

    /// Returns a [`Definition`] sorting weight by its directive.
    pub(super) fn by_directive<'b, 'a, T>(definition: &'b Definition<'a, T>) -> Option<&'b T::Value>
    where
        T: schema::Text<'a>,
    {
        match definition {
            Definition::SchemaDefinition(_) => None,
            Definition::DirectiveDefinition(_) => None,
            Definition::TypeDefinition(t) => match t {
                TypeDefinition::Enum(d) => d.directives.first().map(|d| &d.name),
                TypeDefinition::InputObject(d) => d.directives.first().map(|d| &d.name),
                TypeDefinition::Interface(d) => d.directives.first().map(|d| &d.name),
                TypeDefinition::Scalar(d) => d.directives.first().map(|d| &d.name),
                TypeDefinition::Object(d) => d.directives.first().map(|d| &d.name),
                TypeDefinition::Union(d) => d.directives.first().map(|d| &d.name),
            },
            Definition::TypeExtension(e) => match e {
                TypeExtension::Enum(d) => d.directives.first().map(|d| &d.name),
                TypeExtension::InputObject(d) => d.directives.first().map(|d| &d.name),
                TypeExtension::Interface(d) => d.directives.first().map(|d| &d.name),
                TypeExtension::Scalar(d) => d.directives.first().map(|d| &d.name),
                TypeExtension::Object(d) => d.directives.first().map(|d| &d.name),
                TypeExtension::Union(d) => d.directives.first().map(|d| &d.name),
            },
        }
    }

    /// Returns a [`Definition`] sorting weight by whether it represents a directive.
    pub(super) fn by_is_directive<'a, T>(definition: &Definition<'a, T>) -> u8
    where
        T: schema::Text<'a>,
    {
        match definition {
            Definition::SchemaDefinition(_) => 0,
            Definition::DirectiveDefinition(_) => 1,
            _ => 2,
        }
    }
}
