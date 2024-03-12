use std::collections::HashSet;

use crate::{
    ast::InputValue,
    schema::{
        meta::{Argument, EnumMeta, InputObjectMeta, MetaType},
        model::{SchemaType, TypeType},
    },
    value::ScalarValue,
};

/// Common error messages used in validation and execution of GraphQL operations
pub(crate) mod error {
    use std::fmt::Display;

    pub(crate) fn non_null(arg_type: impl Display) -> String {
        format!("\"null\" specified for not nullable type \"{arg_type}\"")
    }

    pub(crate) fn enum_value(arg_value: impl Display, arg_type: impl Display) -> String {
        format!("Invalid value \"{arg_value}\" for enum \"{arg_type}\"")
    }

    pub(crate) fn type_value(arg_value: impl Display, arg_type: impl Display) -> String {
        format!("Invalid value \"{arg_value}\" for type \"{arg_type}\"")
    }

    pub(crate) fn parser(arg_type: impl Display, msg: impl Display) -> String {
        format!("Parser error for \"{arg_type}\": {msg}")
    }

    pub(crate) fn not_input_object(arg_type: impl Display) -> String {
        format!("\"{arg_type}\" is not an input object")
    }

    pub(crate) fn field(
        arg_type: impl Display,
        field_name: impl Display,
        error_message: impl Display,
    ) -> String {
        format!("Error on \"{arg_type}\" field \"{field_name}\": {error_message}")
    }

    pub(crate) fn missing_fields(arg_type: impl Display, missing_fields: impl Display) -> String {
        format!("\"{arg_type}\" is missing fields: {missing_fields}")
    }

    pub(crate) fn unknown_field(arg_type: impl Display, field_name: impl Display) -> String {
        format!("Field \"{field_name}\" does not exist on type \"{arg_type}\"")
    }

    pub(crate) fn invalid_list_length(
        arg_value: impl Display,
        actual: usize,
        expected: usize,
    ) -> String {
        format!("Expected list of length {expected}, but \"{arg_value}\" has length {actual}")
    }
}

/// Validates the specified field of a GraphQL object and returns an error message if the field is
/// invalid.
fn validate_object_field<S>(
    schema: &SchemaType<S>,
    object_type: &TypeType<S>,
    object_fields: &[Argument<S>],
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

        error_message.map(|m| error::field(object_type, field_key, m))
    } else {
        Some(error::unknown_field(object_type, field_key))
    }
}

/// Validates the specified GraphQL literal and returns an error message if the it's invalid.
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
                Some(error::non_null(arg_type))
            } else {
                validate_literal_value(schema, inner, arg_value)
            }
        }
        TypeType::List(ref inner, expected_size) => match *arg_value {
            InputValue::Null | InputValue::Variable(_) => None,
            InputValue::List(ref items) => {
                if let Some(expected) = expected_size {
                    if items.len() != expected {
                        return Some(error::invalid_list_length(arg_value, items.len(), expected));
                    }
                }
                items
                    .iter()
                    .find_map(|i| validate_literal_value(schema, inner, &i.item))
            }
            ref v => {
                if let Some(expected) = expected_size {
                    if expected != 1 {
                        return Some(error::invalid_list_length(arg_value, 1, expected));
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
                return Some(error::enum_value(arg_value, arg_type));
            }

            match *arg_value {
                InputValue::Null | InputValue::Variable(_) => None,
                ref v @ InputValue::Scalar(_) | ref v @ InputValue::Enum(_) => {
                    if let Some(parse_fn) = t.input_value_parse_fn() {
                        if parse_fn(v).is_ok() {
                            None
                        } else {
                            Some(error::type_value(arg_value, arg_type))
                        }
                    } else {
                        Some(error::parser(arg_type, "no parser present"))
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
                            Some(error::missing_fields(arg_type, missing_fields))
                        }
                    } else {
                        Some(error::not_input_object(arg_type))
                    }
                }
            }
        }
    }
}
