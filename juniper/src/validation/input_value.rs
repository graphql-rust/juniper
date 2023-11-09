use std::{collections::HashSet, fmt};

use crate::{
    ast::{InputValue, Operation, VariableDefinitions},
    executor::Variables,
    parser::{SourcePosition, Spanning},
    schema::{
        meta::{EnumMeta, InputObjectMeta, MetaType, ScalarMeta},
        model::{SchemaType, TypeType},
    },
    validation::RuleError,
    value::ScalarValue,
};

#[derive(Debug)]
enum Path<'a> {
    Root,
    ArrayElement(usize, &'a Path<'a>),
    ObjectField(&'a str, &'a Path<'a>),
}

#[doc(hidden)]
pub fn validate_input_values<S>(
    values: &Variables<S>,
    operation: &Spanning<Operation<S>>,
    schema: &SchemaType<S>,
) -> Vec<RuleError>
where
    S: ScalarValue,
{
    let mut errs = vec![];

    if let Some(ref vars) = operation.item.variable_definitions {
        validate_var_defs(values, &vars.item, schema, &mut errs);
    }

    errs.sort();
    errs
}

fn validate_var_defs<S>(
    values: &Variables<S>,
    var_defs: &VariableDefinitions<S>,
    schema: &SchemaType<S>,
    errors: &mut Vec<RuleError>,
) where
    S: ScalarValue,
{
    for (name, def) in var_defs.iter() {
        let raw_type_name = def.var_type.item.innermost_name();
        match schema.concrete_type_by_name(raw_type_name) {
            Some(t) if t.is_input() => {
                let ct = schema.make_type(&def.var_type.item);

                if def.var_type.item.is_non_null() && is_absent_or_null(values.get(name.item)) {
                    errors.push(RuleError::new(
                        &format!(
                            r#"Variable "${}" of required type "{}" was not provided."#,
                            name.item, def.var_type.item,
                        ),
                        &[name.span.start],
                    ));
                } else if let Some(v) = values.get(name.item) {
                    errors.append(&mut unify_value(
                        name.item,
                        &name.span.start,
                        v,
                        &ct,
                        schema,
                        Path::Root,
                    ));
                }
            }
            _ => unreachable!(
                r#"Variable "${}" has invalid input type "{}" after document validation."#,
                name.item, def.var_type.item,
            ),
        }
    }
}

fn unify_value<'a, S>(
    var_name: &str,
    var_pos: &SourcePosition,
    value: &InputValue<S>,
    meta_type: &TypeType<'a, S>,
    schema: &SchemaType<S>,
    path: Path<'a>,
) -> Vec<RuleError>
where
    S: ScalarValue,
{
    let mut errors: Vec<RuleError> = vec![];

    match *meta_type {
        TypeType::NonNull(ref inner) => {
            if value.is_null() {
                errors.push(unification_error(
                    var_name,
                    var_pos,
                    &path,
                    format!(r#"Expected "{meta_type}", found null"#),
                ));
            } else {
                errors.append(&mut unify_value(
                    var_name, var_pos, value, inner, schema, path,
                ));
            }
        }

        TypeType::List(ref inner, expected_size) => {
            if value.is_null() {
                return errors;
            }

            match value.to_list_value() {
                Some(l) => {
                    if let Some(expected) = expected_size {
                        if l.len() != expected {
                            errors.push(unification_error(
                                var_name,
                                var_pos,
                                &path,
                                format!(
                                    "Expected list of {expected} elements, \
                                     found {} elements",
                                    l.len(),
                                ),
                            ));
                        }
                    }
                    for (i, v) in l.iter().enumerate() {
                        errors.append(&mut unify_value(
                            var_name,
                            var_pos,
                            v,
                            inner,
                            schema,
                            Path::ArrayElement(i, &path),
                        ));
                    }
                }
                _ => errors.append(&mut unify_value(
                    var_name, var_pos, value, inner, schema, path,
                )),
            }
        }

        TypeType::Concrete(mt) => {
            if value.is_null() {
                return errors;
            }

            match *mt {
                MetaType::Scalar(ref sm) => {
                    errors.append(&mut unify_scalar(var_name, var_pos, value, sm, &path))
                }
                MetaType::Enum(ref em) => {
                    errors.append(&mut unify_enum(var_name, var_pos, value, em, &path))
                }
                MetaType::InputObject(ref iom) => {
                    let mut e = unify_input_object(var_name, var_pos, value, iom, schema, &path);
                    if e.is_empty() {
                        // All the fields didn't have errors, see if there is an
                        // overall error when parsing the input value.
                        if let Err(e) = (iom.try_parse_fn)(value) {
                            errors.push(unification_error(
                                var_name,
                                var_pos,
                                &path,
                                format!(
                                    "Expected input of type `{}`. \
                                     Got: `{value}`. \
                                     Details: {}",
                                    iom.name,
                                    e.message(),
                                ),
                            ));
                        }
                    } else {
                        errors.append(&mut e);
                    }
                }
                _ => panic!("Can't unify non-input concrete type"),
            }
        }
    }
    errors
}

fn unify_scalar<S>(
    var_name: &str,
    var_pos: &SourcePosition,
    value: &InputValue<S>,
    meta: &ScalarMeta<S>,
    path: &Path<'_>,
) -> Vec<RuleError>
where
    S: ScalarValue,
{
    let mut errors: Vec<RuleError> = vec![];

    if let Err(e) = (meta.try_parse_fn)(value) {
        return vec![unification_error(
            var_name,
            var_pos,
            path,
            format!(
                "Expected input scalar `{}`. Got: `{value}`. Details: {}",
                meta.name,
                e.message(),
            ),
        )];
    }

    match *value {
        InputValue::List(_) => errors.push(unification_error(
            var_name,
            var_pos,
            path,
            format!(r#"Expected "{}", found list"#, meta.name),
        )),
        InputValue::Object(_) => errors.push(unification_error(
            var_name,
            var_pos,
            path,
            format!(r#"Expected "{}", found object"#, meta.name),
        )),
        _ => (),
    }
    errors
}

fn unify_enum<S>(
    var_name: &str,
    var_pos: &SourcePosition,
    value: &InputValue<S>,
    meta: &EnumMeta<S>,
    path: &Path<'_>,
) -> Vec<RuleError>
where
    S: ScalarValue,
{
    let mut errors: Vec<RuleError> = vec![];

    match value {
        // TODO: avoid this bad duplicate as_str() call. (value system refactor)
        InputValue::Scalar(scalar) if scalar.as_str().is_some() => {
            if let Some(name) = scalar.as_str() {
                if !meta.values.iter().any(|ev| ev.name == *name) {
                    errors.push(unification_error(
                        var_name,
                        var_pos,
                        path,
                        format!(r#"Invalid value for enum "{}""#, meta.name),
                    ))
                }
            }
        }
        InputValue::Enum(name) => {
            if !meta.values.iter().any(|ev| &ev.name == name) {
                errors.push(unification_error(
                    var_name,
                    var_pos,
                    path,
                    format!(r#"Invalid value for enum "{}""#, meta.name),
                ))
            }
        }
        _ => errors.push(unification_error(
            var_name,
            var_pos,
            path,
            format!(r#"Expected "{}", found not a string or enum"#, meta.name),
        )),
    }
    errors
}

fn unify_input_object<S>(
    var_name: &str,
    var_pos: &SourcePosition,
    value: &InputValue<S>,
    meta: &InputObjectMeta<S>,
    schema: &SchemaType<S>,
    path: &Path<'_>,
) -> Vec<RuleError>
where
    S: ScalarValue,
{
    let mut errors: Vec<RuleError> = vec![];

    if let Some(ref obj) = value.to_object_value() {
        let mut keys = obj.keys().collect::<HashSet<&&str>>();

        for input_field in &meta.input_fields {
            let mut has_value = false;
            keys.remove(&input_field.name.as_str());

            if let Some(value) = obj.get(input_field.name.as_str()) {
                if !value.is_null() {
                    has_value = true;

                    errors.append(&mut unify_value(
                        var_name,
                        var_pos,
                        value,
                        &schema.make_type(&input_field.arg_type),
                        schema,
                        Path::ObjectField(&input_field.name, path),
                    ));
                }
            }

            if !has_value && input_field.arg_type.is_non_null() {
                errors.push(unification_error(
                    var_name,
                    var_pos,
                    &Path::ObjectField(&input_field.name, path),
                    format!(r#"Expected "{}", found null"#, input_field.arg_type),
                ));
            }
        }

        for key in keys {
            errors.push(unification_error(
                var_name,
                var_pos,
                &Path::ObjectField(key, path),
                "Unknown field",
            ));
        }
    } else {
        errors.push(unification_error(
            var_name,
            var_pos,
            path,
            format!(r#"Expected "{}", found not an object"#, meta.name),
        ));
    }
    errors
}

fn is_absent_or_null<S>(v: Option<&InputValue<S>>) -> bool
where
    S: ScalarValue,
{
    v.map_or(true, InputValue::is_null)
}

fn unification_error(
    var_name: impl fmt::Display,
    var_pos: &SourcePosition,
    path: &Path<'_>,
    message: impl fmt::Display,
) -> RuleError {
    RuleError::new(
        &format!(r#"Variable "${var_name}" got invalid value. {path}{message}."#),
        &[*var_pos],
    )
}

impl<'a> fmt::Display for Path<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Path::Root => write!(f, ""),
            Path::ArrayElement(idx, prev) => write!(f, "{prev}In element #{idx}: "),
            Path::ObjectField(name, prev) => write!(f, r#"{prev}In field "{name}": "#),
        }
    }
}
