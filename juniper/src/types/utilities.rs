use std::{collections::HashSet, fmt::Display, iter::Iterator};

use crate::{
    ast::InputValue,
    schema::{
        meta::{Argument, EnumMeta, InputObjectMeta, MetaType},
        model::{SchemaType, TypeType},
    },
    value::ScalarValue,
};

pub(crate) fn non_null_error_message(arg_type: impl Display) -> String {
    format!("Type \"{arg_type}\" is not nullable")
}

pub(crate) fn enum_error_message(arg_value: impl Display, arg_type: impl Display) -> String {
    format!("Invalid value \"{arg_value}\" for enum \"{arg_type}\"")
}

pub(crate) fn type_error_message(arg_value: impl Display, arg_type: impl Display) -> String {
    format!("Invalid value \"{arg_value}\" for type \"{arg_type}\"")
}

pub(crate) fn parser_error_message(arg_type: impl Display) -> String {
    format!("Parser error for \"{arg_type}\"")
}

pub(crate) fn input_object_error_message(arg_type: impl Display) -> String {
    format!("\"{arg_type}\" is not an input object")
}

pub(crate) fn field_error_message(
    arg_type: impl Display,
    field_name: impl Display,
    error_message: impl Display,
) -> String {
    format!("Error on \"{arg_type}\" field \"{field_name}\": {error_message}")
}

pub(crate) fn missing_field_error_message(
    arg_type: impl Display,
    missing_fields: impl Display,
) -> String {
    format!("\"{arg_type}\" is missing fields: {missing_fields}")
}

pub(crate) fn unknown_field_error_message(
    arg_type: impl Display,
    field_name: impl Display,
) -> String {
    format!("Field \"{field_name}\" does not exist on type \"{arg_type}\"")
}

/// Returns an error string if the field is invalid
fn validate_object_field<S>(
    schema: &SchemaType<S>,
    object_type: &TypeType<S>,
    object_fields: &Vec<Argument<S>>,
    field_value: &InputValue<S>,
    field_key: &str,
) -> Option<String>
where
    S: ScalarValue,
{
    let field_type = object_fields
        .iter()
        .filter(|f| f.name == field_key)
        .map(|f| schema.make_type(&f.arg_type))
        .next();

    if let Some(field_arg_type) = field_type {
        let error_message = validate_literal_value(schema, &field_arg_type, field_value);

        if let Some(error_message) = error_message {
            Some(field_error_message(object_type, field_key, &error_message))
        } else {
            None
        }
    } else {
        Some(unknown_field_error_message(object_type, field_key))
    }
}

/// Returns an error string if the value is invalid
pub fn validate_literal_value<S>(
    schema: &SchemaType<S>,
    arg_type: &TypeType<S>,
    arg_value: &InputValue<S>,
) -> Option<String>
where
    S: ScalarValue,
{
    match *arg_type {
        TypeType::NonNull(ref inner) => {
            if arg_value.is_null() {
                Some(non_null_error_message(arg_type))
            } else {
                validate_literal_value(schema, inner, arg_value)
            }
        }
        TypeType::List(ref inner, expected_size) => match *arg_value {
            InputValue::Null | InputValue::Variable(_) => None,
            InputValue::List(ref items) => {
                if let Some(expected) = expected_size {
                    if items.len() != expected {
                        return todo!();
                    }
                }
                items
                    .iter()
                    .find_map(|i| validate_literal_value(schema, inner, &i.item))
            }
            ref v => {
                if let Some(expected) = expected_size {
                    if expected != 1 {
                        return todo!();
                    }
                }
                validate_literal_value(schema, inner, v)
            }
        },
        TypeType::Concrete(t) => {
            // Even though InputValue::String can be parsed into an enum, they
            // are not valid as enum *literals* in a GraphQL query.
            if let (&InputValue::Scalar(_), Some(&MetaType::Enum(EnumMeta { .. }))) =
                (arg_value, arg_type.to_concrete())
            {
                return Some(enum_error_message(arg_value, arg_type));
            }

            match *arg_value {
                InputValue::Null | InputValue::Variable(_) => None,
                ref v @ InputValue::Scalar(_) | ref v @ InputValue::Enum(_) => {
                    if let Some(parse_fn) = t.input_value_parse_fn() {
                        if parse_fn(v).is_ok() {
                            // TODO: reuse error?
                            None
                        } else {
                            Some(type_error_message(arg_value, arg_type))
                        }
                    } else {
                        Some(parser_error_message(arg_type))
                    }
                }
                InputValue::List(_) => Some("Input lists are not literals".to_owned()),
                InputValue::Object(ref obj) => {
                    if let MetaType::InputObject(InputObjectMeta {
                        ref input_fields, ..
                    }) = *t
                    {
                        let mut remaining_required_fields = input_fields
                            .iter()
                            .filter_map(|f| {
                                (f.arg_type.is_non_null() && f.default_value.is_none())
                                    .then_some(&f.name)
                            })
                            .collect::<HashSet<_>>();

                        let error_message = obj.iter().find_map(|(key, value)| {
                            remaining_required_fields.remove(&key.item);
                            validate_object_field(
                                schema,
                                arg_type,
                                input_fields,
                                &value.item,
                                &key.item,
                            )
                        });

                        if error_message.is_some() {
                            return error_message;
                        }

                        if remaining_required_fields.is_empty() {
                            None
                        } else {
                            let missing_fields = remaining_required_fields
                                .into_iter()
                                .map(|s| format!("\"{}\"", &**s))
                                .collect::<Vec<_>>()
                                .join(", ");
                            Some(missing_field_error_message(arg_type, missing_fields))
                        }
                    } else {
                        Some(input_object_error_message(arg_type))
                    }
                }
            }
        }
    }
}
