use crate::{
    ast::InputValue,
    schema::{
        meta::{EnumMeta, InputObjectMeta, MetaType, ScalarMeta},
        model::{SchemaType, TypeType},
    },
    value::ScalarValue,
};
use std::collections::HashSet;

pub fn is_valid_literal_value<S>(
    schema: &SchemaType<S>,
    arg_type: &TypeType<S>,
    arg_value: &InputValue<S>,
) -> bool
where
    S: ScalarValue,
{
    match arg_type {
        TypeType::NonNull(inner) => {
            if arg_value.is_null() {
                // This hack is required as Juniper doesn't allow at the moment
                // for custom defined types to tweak into parsing validation.
                // TODO: Redesign parsing layer to allow such things.
                #[cfg(feature = "json")]
                if let TypeType::Concrete(t) = &**inner {
                    if let MetaType::Scalar(ScalarMeta { name, .. }) = t {
                        if name == "Json" {
                            if let Some(parse_fn) = t.input_value_parse_fn() {
                                return parse_fn(arg_value);
                            }
                        }
                    }
                }

                false
            } else {
                is_valid_literal_value(schema, inner, arg_value)
            }
        }
        TypeType::List(inner, expected_size) => match arg_value {
            InputValue::List(items) => {
                if let Some(expected) = expected_size {
                    if items.len() != *expected {
                        return false;
                    }
                }
                items
                    .iter()
                    .all(|i| is_valid_literal_value(schema, inner, &i.item))
            }
            v => {
                if let Some(expected) = expected_size {
                    if *expected != 1 {
                        return false;
                    }
                }
                is_valid_literal_value(schema, inner, v)
            }
        },
        TypeType::Concrete(t) => {
            // Even though InputValue::String can be parsed into an enum, they
            // are not valid as enum *literals* in a GraphQL query.
            if let (&InputValue::Scalar(_), MetaType::Enum(EnumMeta { .. })) = (arg_value, t) {
                return false;
            }

            // This hack is required as Juniper doesn't allow at the moment
            // for custom defined types to tweak into parsing validation.
            // TODO: Redesign parsing layer to allow such things.
            #[cfg(feature = "json")]
            if let MetaType::Scalar(ScalarMeta { name, .. }) = t {
                if name == "Json" {
                    if let Some(parse_fn) = t.input_value_parse_fn() {
                        return parse_fn(arg_value);
                    }
                }
            }

            match arg_value {
                InputValue::Null | InputValue::Variable(_) => true,
                ref v @ InputValue::Scalar(_) | ref v @ InputValue::Enum(_) => {
                    if let Some(parse_fn) = t.input_value_parse_fn() {
                        parse_fn(v)
                    } else {
                        false
                    }
                }
                InputValue::List(_) => false,
                InputValue::Object(obj) => {
                    if let MetaType::InputObject(InputObjectMeta { input_fields, .. }) = t {
                        let mut remaining_required_fields = input_fields
                            .iter()
                            .filter_map(|f| {
                                if f.arg_type.is_non_null() {
                                    Some(&f.name)
                                } else {
                                    None
                                }
                            })
                            .collect::<HashSet<_>>();

                        let all_types_ok = obj.iter().all(|&(ref key, ref value)| {
                            remaining_required_fields.remove(&key.item);
                            if let Some(arg_type) = input_fields
                                .iter()
                                .filter(|f| f.name == key.item)
                                .map(|f| schema.make_type(&f.arg_type))
                                .next()
                            {
                                is_valid_literal_value(schema, &arg_type, &value.item)
                            } else {
                                false
                            }
                        });

                        all_types_ok && remaining_required_fields.is_empty()
                    } else {
                        false
                    }
                }
            }
        }
    }
}
