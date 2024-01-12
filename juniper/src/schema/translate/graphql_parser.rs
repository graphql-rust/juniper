use std::{boxed::Box, collections::BTreeMap};

use graphql_parser::{
    query::{Directive as ExternalDirective, Number as ExternalNumber, Type as ExternalType},
    schema::{
        Definition, Document, EnumType as ExternalEnum, EnumValue as ExternalEnumValue,
        Field as ExternalField, InputObjectType as ExternalInputObjectType,
        InputValue as ExternalInputValue, InterfaceType as ExternalInterfaceType,
        ObjectType as ExternalObjectType, ScalarType as ExternalScalarType, SchemaDefinition, Text,
        TypeDefinition as ExternalTypeDefinition, UnionType as ExternalUnionType,
        Value as ExternalValue,
    },
    Pos,
};

use crate::{
    ast::{InputValue, Type},
    schema::{
        meta::{Argument, DeprecationStatus, EnumValue, Field, MetaType},
        model::SchemaType,
        translate::SchemaTranslator,
    },
    value::ScalarValue,
};

pub struct GraphQLParserTranslator;

impl<'a, S: 'a, T> From<&'a SchemaType<'a, S>> for Document<'a, T>
where
    S: ScalarValue,
    T: Text<'a> + Default,
{
    fn from(input: &'a SchemaType<'a, S>) -> Document<'a, T> {
        GraphQLParserTranslator::translate_schema(input)
    }
}

impl<'a, T> SchemaTranslator<'a, graphql_parser::schema::Document<'a, T>>
    for GraphQLParserTranslator
where
    T: Text<'a> + Default,
{
    fn translate_schema<S: 'a>(input: &'a SchemaType<S>) -> graphql_parser::schema::Document<'a, T>
    where
        S: ScalarValue,
    {
        let mut doc = Document::default();

        // Translate type defs.
        let mut types = input
            .types
            .iter()
            .filter(|(_, meta)| !meta.is_builtin())
            .map(|(_, meta)| GraphQLParserTranslator::translate_meta(meta))
            .map(Definition::TypeDefinition)
            .collect();
        doc.definitions.append(&mut types);

        doc.definitions
            .push(Definition::SchemaDefinition(SchemaDefinition {
                position: Pos::default(),
                directives: vec![],
                query: Some(From::from(input.query_type_name.as_str())),
                mutation: input
                    .mutation_type_name
                    .as_ref()
                    .map(|s| From::from(s.as_str())),
                subscription: input
                    .subscription_type_name
                    .as_ref()
                    .map(|s| From::from(s.as_str())),
            }));

        doc
    }
}

impl GraphQLParserTranslator {
    fn translate_argument<'a, S, T>(input: &'a Argument<S>) -> ExternalInputValue<'a, T>
    where
        S: ScalarValue,
        T: Text<'a>,
    {
        ExternalInputValue {
            position: Pos::default(),
            description: input.description.as_ref().map(From::from),
            name: From::from(input.name.as_str()),
            value_type: GraphQLParserTranslator::translate_type(&input.arg_type),
            default_value: input
                .default_value
                .as_ref()
                .map(|x| GraphQLParserTranslator::translate_value(x)),
            directives: vec![],
        }
    }

    fn translate_value<'a, S: 'a, T>(input: &'a InputValue<S>) -> ExternalValue<'a, T>
    where
        S: ScalarValue,
        T: Text<'a>,
    {
        match input {
            InputValue::Null => ExternalValue::Null,
            InputValue::Scalar(x) => {
                if let Some(v) = x.as_string() {
                    ExternalValue::String(v)
                } else if let Some(v) = x.as_int() {
                    ExternalValue::Int(ExternalNumber::from(v))
                } else if let Some(v) = x.as_float() {
                    ExternalValue::Float(v)
                } else if let Some(v) = x.as_bool() {
                    ExternalValue::Boolean(v)
                } else {
                    panic!("unknown argument type")
                }
            }
            InputValue::Enum(x) => ExternalValue::Enum(From::from(x.as_str())),
            InputValue::Variable(x) => ExternalValue::Variable(From::from(x.as_str())),
            InputValue::List(x) => ExternalValue::List(
                x.iter()
                    .map(|s| GraphQLParserTranslator::translate_value(&s.item))
                    .collect(),
            ),
            InputValue::Object(x) => {
                let mut fields = BTreeMap::new();
                x.iter().for_each(|(name_span, value_span)| {
                    fields.insert(
                        From::from(name_span.item.as_str()),
                        GraphQLParserTranslator::translate_value(&value_span.item),
                    );
                });
                ExternalValue::Object(fields)
            }
        }
    }

    fn translate_type<'a, T>(input: &'a Type<'a>) -> ExternalType<'a, T>
    where
        T: Text<'a>,
    {
        match input {
            Type::Named(x) => ExternalType::NamedType(From::from(x.as_ref())),
            Type::List(x, _) => {
                ExternalType::ListType(GraphQLParserTranslator::translate_type(x).into())
            }
            Type::NonNullNamed(x) => {
                ExternalType::NonNullType(Box::new(ExternalType::NamedType(From::from(x.as_ref()))))
            }
            Type::NonNullList(x, _) => ExternalType::NonNullType(Box::new(ExternalType::ListType(
                Box::new(GraphQLParserTranslator::translate_type(x)),
            ))),
        }
    }

    fn translate_meta<'a, S, T>(input: &'a MetaType<S>) -> ExternalTypeDefinition<'a, T>
    where
        S: ScalarValue,
        T: Text<'a>,
    {
        match input {
            MetaType::Scalar(x) => ExternalTypeDefinition::Scalar(ExternalScalarType {
                position: Pos::default(),
                description: x.description.as_ref().map(From::from),
                name: From::from(x.name.as_ref()),
                directives: vec![],
            }),
            MetaType::Enum(x) => ExternalTypeDefinition::Enum(ExternalEnum {
                position: Pos::default(),
                description: x.description.as_ref().map(|s| From::from(s.as_str())),
                name: From::from(x.name.as_ref()),
                directives: vec![],
                values: x
                    .values
                    .iter()
                    .map(GraphQLParserTranslator::translate_enum_value)
                    .collect(),
            }),
            MetaType::Union(x) => ExternalTypeDefinition::Union(ExternalUnionType {
                position: Pos::default(),
                description: x.description.as_ref().map(|s| From::from(s.as_str())),
                name: From::from(x.name.as_ref()),
                directives: vec![],
                types: x
                    .of_type_names
                    .iter()
                    .map(|s| From::from(s.as_str()))
                    .collect(),
            }),
            MetaType::Interface(x) => ExternalTypeDefinition::Interface(ExternalInterfaceType {
                position: Pos::default(),
                description: x.description.as_ref().map(|s| From::from(s.as_str())),
                name: From::from(x.name.as_ref()),
                implements_interfaces: x
                    .interface_names
                    .iter()
                    .map(|s| From::from(s.as_str()))
                    .collect(),
                directives: vec![],
                fields: x
                    .fields
                    .iter()
                    .filter(|x| !x.is_builtin())
                    .map(GraphQLParserTranslator::translate_field)
                    .collect(),
            }),
            MetaType::InputObject(x) => {
                ExternalTypeDefinition::InputObject(ExternalInputObjectType {
                    position: Pos::default(),
                    description: x.description.as_ref().map(|s| From::from(s.as_str())),
                    name: From::from(x.name.as_ref()),
                    directives: vec![],
                    fields: x
                        .input_fields
                        .iter()
                        .filter(|x| !x.is_builtin())
                        .map(GraphQLParserTranslator::translate_argument)
                        .collect(),
                })
            }
            MetaType::Object(x) => ExternalTypeDefinition::Object(ExternalObjectType {
                position: Pos::default(),
                description: x.description.as_ref().map(|s| From::from(s.as_str())),
                name: From::from(x.name.as_ref()),
                directives: vec![],
                fields: x
                    .fields
                    .iter()
                    .filter(|x| !x.is_builtin())
                    .map(GraphQLParserTranslator::translate_field)
                    .collect(),
                implements_interfaces: x
                    .interface_names
                    .iter()
                    .map(|s| From::from(s.as_str()))
                    .collect(),
            }),
            _ => panic!("unknown meta type when translating"),
        }
    }

    fn translate_enum_value<'a, T>(input: &'a EnumValue) -> ExternalEnumValue<'a, T>
    where
        T: Text<'a>,
    {
        ExternalEnumValue {
            position: Pos::default(),
            name: From::from(input.name.as_ref()),
            description: input.description.as_ref().map(|s| From::from(s.as_str())),
            directives: generate_directives(&input.deprecation_status),
        }
    }

    fn translate_field<'a, S: 'a, T>(input: &'a Field<S>) -> ExternalField<'a, T>
    where
        S: ScalarValue,
        T: Text<'a>,
    {
        let arguments = input
            .arguments
            .as_ref()
            .map(|a| {
                a.iter()
                    .filter(|x| !x.is_builtin())
                    .map(|x| GraphQLParserTranslator::translate_argument(x))
                    .collect()
            })
            .unwrap_or_default();

        ExternalField {
            position: Pos::default(),
            name: From::from(input.name.as_str()),
            description: input.description.as_ref().map(|s| From::from(s.as_str())),
            directives: generate_directives(&input.deprecation_status),
            field_type: GraphQLParserTranslator::translate_type(&input.field_type),
            arguments,
        }
    }
}

fn deprecation_to_directive<'a, T>(status: &DeprecationStatus) -> Option<ExternalDirective<'a, T>>
where
    T: Text<'a>,
{
    match status {
        DeprecationStatus::Current => None,
        DeprecationStatus::Deprecated(reason) => Some(ExternalDirective {
            position: Pos::default(),
            name: "deprecated".into(),
            arguments: reason
                .as_ref()
                .map(|rsn| vec![(From::from("reason"), ExternalValue::String(rsn.into()))])
                .unwrap_or_default(),
        }),
    }
}

// Right now the only directive supported is `@deprecated`.
// `@skip` and `@include` are dealt with elsewhere.
// https://spec.graphql.org/October2021#sec-Type-System.Directives.Built-in-Directives
fn generate_directives<'a, T>(status: &DeprecationStatus) -> Vec<ExternalDirective<'a, T>>
where
    T: Text<'a>,
{
    if let Some(d) = deprecation_to_directive(status) {
        vec![d]
    } else {
        vec![]
    }
}

/// Sorts the provided [`Document`] in the "type-then-name" manner.
pub(crate) fn sort_schema_document<'a, T: Text<'a>>(document: &mut Document<'a, T>) {
    document.definitions.sort_by(move |a, b| {
        let type_cmp = sort_value::by_type(a).cmp(&sort_value::by_type(b));
        let name_cmp = sort_value::by_is_directive(a)
            .cmp(&sort_value::by_is_directive(b))
            .then(sort_value::by_name(a).cmp(&sort_value::by_name(b)))
            .then(sort_value::by_directive(a).cmp(&sort_value::by_directive(b)));
        type_cmp.then(name_cmp)
    })
}

/// Evaluation of a [`Definition`] weights for sorting.
mod sort_value {
    use graphql_parser::schema::{Definition, Text, TypeDefinition, TypeExtension};

    /// Returns a [`Definition`] sorting weight by its type.
    pub(super) fn by_type<'a, T>(definition: &Definition<'a, T>) -> u8
    where
        T: Text<'a>,
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
        T: Text<'a>,
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
        T: Text<'a>,
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
        T: Text<'a>,
    {
        match definition {
            Definition::SchemaDefinition(_) => 0,
            Definition::DirectiveDefinition(_) => 1,
            _ => 2,
        }
    }
}
