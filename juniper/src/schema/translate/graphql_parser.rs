use std::boxed::Box;
use std::collections::BTreeMap;

use graphql_parser::query::{
    Directive as ExternalDirective, Number as ExternalNumber, Type as ExternalType,
};
use graphql_parser::schema::{Definition, Document, SchemaDefinition};
use graphql_parser::schema::{
    EnumType as ExternalEnum, EnumValue as ExternalEnumValue, Field as ExternalField,
    InputObjectType as ExternalInputObjectType, InputValue as ExternalInputValue,
    InterfaceType as ExternalInterfaceType, ObjectType as ExternalObjectType,
    ScalarType as ExternalScalarType, TypeDefinition as ExternalTypeDefinition,
    UnionType as ExternalUnionType, Value as ExternalValue,
};
use graphql_parser::Pos;

use ast::{InputValue, Type};
use schema::meta::DeprecationStatus;
use schema::meta::{Argument, EnumValue, Field, MetaType};
use schema::model::SchemaType;
use schema::translate::SchemaTranslator;
use value::ScalarValue;

pub struct GraphQLParserTranslator;

impl<'a, S> From<SchemaType<'a, S>> for Document
where
    S: ScalarValue,
{
    fn from(input: SchemaType<'a, S>) -> Document {
        GraphQLParserTranslator::translate_schema(&input)
    }
}

impl SchemaTranslator<graphql_parser::schema::Document> for GraphQLParserTranslator {
    fn translate_schema<'a, S>(input: &SchemaType<'a, S>) -> graphql_parser::schema::Document
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
            .map(|x| Definition::TypeDefinition(x))
            .collect::<Vec<Definition>>();
        doc.definitions.append(&mut types);

        doc.definitions
            .push(Definition::SchemaDefinition(SchemaDefinition {
                position: Pos::default(),
                directives: vec![],
                query: Some(input.query_type_name.clone()),
                mutation: input.mutation_type_name.clone(),
                // TODO: implement once we support subscriptions.
                subscription: None,
            }));

        doc
    }
}

impl GraphQLParserTranslator {
    fn translate_argument<'a, S>(input: &Argument<'a, S>) -> ExternalInputValue
    where
        S: ScalarValue,
    {
        ExternalInputValue {
            position: Pos::default(),
            description: input.description.clone(),
            name: input.name.clone(),
            value_type: GraphQLParserTranslator::translate_type(&input.arg_type),
            default_value: match input.default_value {
                None => None,
                Some(ref v) => Some(GraphQLParserTranslator::translate_value(v)),
            },
            directives: vec![],
        }
    }

    fn translate_value<S>(input: &InputValue<S>) -> ExternalValue
    where
        S: ScalarValue,
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
                } else if let Some(v) = x.as_boolean() {
                    ExternalValue::Boolean(v)
                } else {
                    panic!("unknown argument type")
                }
            }
            InputValue::Enum(x) => ExternalValue::Enum(x.clone()),
            InputValue::Variable(x) => ExternalValue::Variable(x.clone()),
            InputValue::List(x) => ExternalValue::List(
                x.iter()
                    .map(|s| GraphQLParserTranslator::translate_value(&s.item))
                    .collect(),
            ),
            InputValue::Object(x) => {
                let mut fields = BTreeMap::new();
                x.iter().for_each(|(name_span, value_span)| {
                    fields.insert(
                        name_span.item.clone(),
                        GraphQLParserTranslator::translate_value(&value_span.item),
                    );
                });
                ExternalValue::Object(fields)
            }
        }
    }

    fn translate_type(input: &Type) -> ExternalType {
        match input {
            Type::Named(x) => ExternalType::NamedType(x.as_ref().to_string()),
            Type::List(x) => ExternalType::ListType(Box::new(
                GraphQLParserTranslator::translate_type(x.as_ref()),
            )),
            Type::NonNullNamed(x) => {
                ExternalType::NonNullType(Box::new(ExternalType::NamedType(x.as_ref().to_string())))
            }
            Type::NonNullList(x) => ExternalType::NonNullType(Box::new(ExternalType::ListType(
                Box::new(GraphQLParserTranslator::translate_type(x.as_ref())),
            ))),
        }
    }

    fn translate_meta<'empty, S>(input: &MetaType<'empty, S>) -> ExternalTypeDefinition
    where
        S: ScalarValue,
    {
        match input {
            MetaType::Scalar(x) => ExternalTypeDefinition::Scalar(ExternalScalarType {
                position: Pos::default(),
                description: x.description.clone(),
                name: x.name.to_string(),
                directives: vec![],
            }),
            MetaType::Enum(x) => ExternalTypeDefinition::Enum(ExternalEnum {
                position: Pos::default(),
                description: x.description.clone(),
                name: x.name.to_string(),
                directives: vec![],
                values: x
                    .values
                    .iter()
                    .map(GraphQLParserTranslator::translate_enum_value)
                    .collect(),
            }),
            MetaType::Union(x) => ExternalTypeDefinition::Union(ExternalUnionType {
                position: Pos::default(),
                description: x.description.clone(),
                name: x.name.to_string(),
                directives: vec![],
                types: x.of_type_names.clone(),
            }),
            MetaType::Interface(x) => ExternalTypeDefinition::Interface(ExternalInterfaceType {
                position: Pos::default(),
                description: x.description.clone(),
                name: x.name.to_string(),
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
                    description: x.description.clone(),
                    name: x.name.to_string(),
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
                description: x.description.clone(),
                name: x.name.to_string(),
                directives: vec![],
                fields: x
                    .fields
                    .iter()
                    .filter(|x| !x.is_builtin())
                    .map(GraphQLParserTranslator::translate_field)
                    .collect(),
                implements_interfaces: x.interface_names.clone(),
            }),
            _ => panic!("unknown meta type when translating"),
        }
    }

    fn translate_enum_value(input: &EnumValue) -> ExternalEnumValue {
        ExternalEnumValue {
            position: Pos::default(),
            name: input.name.clone(),
            description: input.description.clone(),
            directives: generate_directives(&input.deprecation_status),
        }
    }

    fn translate_field<S>(input: &Field<S>) -> ExternalField
    where
        S: ScalarValue,
    {
        ExternalField {
            position: Pos::default(),
            name: input.name.clone(),
            description: input.description.clone(),
            directives: generate_directives(&input.deprecation_status),
            field_type: GraphQLParserTranslator::translate_type(&input.field_type),
            arguments: input
                .clone()
                .arguments
                .unwrap_or(vec![])
                .iter()
                .filter(|x| !x.is_builtin())
                .map(GraphQLParserTranslator::translate_argument)
                .collect(),
        }
    }
}

fn deprecation_to_directive(status: &DeprecationStatus) -> Option<ExternalDirective> {
    match status {
        DeprecationStatus::Current => None,
        DeprecationStatus::Deprecated(reason) => Some(ExternalDirective {
            position: Pos::default(),
            name: "deprecated".to_string(),
            arguments: if let Some(reason) = reason {
                vec![(
                    "reason".to_string(),
                    ExternalValue::String(reason.to_string()),
                )]
            } else {
                vec![]
            },
        }),
    }
}

// Right now the only directive supported is `@deprecated`. `@skip` and `@include`
// are dealt with elsewhere.
// <https://facebook.github.io/graphql/draft/#sec-Type-System.Directives>
fn generate_directives(status: &DeprecationStatus) -> Vec<ExternalDirective> {
    if let Some(d) = deprecation_to_directive(&status) {
        vec![d]
    } else {
        vec![]
    }
}
