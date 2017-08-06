use std::collections::HashSet;
use std::fmt;

use parser::SourcePosition;
use ast::{InputValue, Document, Definition, VariableDefinitions};
use executor::Variables;
use validation::RuleError;
use schema::model::{SchemaType, TypeType};
use schema::meta::{MetaType, ScalarMeta, InputObjectMeta, EnumMeta};

#[derive(Debug)]
enum Path<'a> {
    Root,
    ArrayElement(usize, &'a Path<'a>),
    ObjectField(&'a str, &'a Path<'a>),
}

pub fn validate_input_values(values: &Variables,
                             document: &Document,
                             schema: &SchemaType)
                             -> Vec<RuleError> {
    let mut errs = vec![];

    for def in document {
        if let Definition::Operation(ref op) = *def {
            if let Some(ref vars) = op.item.variable_definitions {
                validate_var_defs(values, &vars.item, schema, &mut errs);
            }
        }
    }

    errs.sort();
    errs
}

fn validate_var_defs(values: &Variables,
                     var_defs: &VariableDefinitions,
                     schema: &SchemaType,
                     errors: &mut Vec<RuleError>) {
    for &(ref name, ref def) in var_defs.iter() {
        let raw_type_name = def.var_type.item.innermost_name();
        match schema.concrete_type_by_name(raw_type_name) {
            Some(t) if t.is_input() => {
                let ct = schema.make_type(&def.var_type.item);

                if def.var_type.item.is_non_null() && is_absent_or_null(values.get(name.item)) {
                    errors.push(RuleError::new(&format!(
                            r#"Variable "${}" of required type "{}" was not provided."#,
                            name.item, def.var_type.item,
                        ),
                                               &[name.start.clone()]));
                } else if let Some(v) = values.get(name.item) {
                    unify_value(name.item, &name.start, v, &ct, schema, errors, Path::Root);
                }
            }
            _ => errors.push(RuleError::new(
                &format!(
                    r#"Variable "${}" expected value of type "{}" which cannot be used as an input type."#,
                    name.item, def.var_type.item,
                ),
                &[ name.start.clone() ],
            )),
        }
    }
}

fn unify_value<'a>(var_name: &str,
                   var_pos: &SourcePosition,
                   value: &InputValue,
                   meta_type: &TypeType<'a>,
                   schema: &SchemaType,
                   errors: &mut Vec<RuleError>,
                   path: Path<'a>) {
    match *meta_type {
        TypeType::NonNull(ref inner) => {
            if value.is_null() {
                push_unification_error(errors,
                                       var_name,
                                       var_pos,
                                       &path,
                                       &format!(r#"Expected "{}", found null"#, meta_type));
            } else {
                unify_value(var_name, var_pos, value, inner, schema, errors, path);
            }
        }

        TypeType::List(ref inner) => {
            if value.is_null() {
                return;
            }

            match value.to_list_value() {
                Some(l) => {
                    for (i, v) in l.iter().enumerate() {
                        unify_value(var_name,
                                    var_pos,
                                    v,
                                    inner,
                                    schema,
                                    errors,
                                    Path::ArrayElement(i, &path));
                    }
                }
                _ => unify_value(var_name, var_pos, value, inner, schema, errors, path),
            }
        }

        TypeType::Concrete(mt) => {
            if value.is_null() {
                return;
            }

            match *mt {
                MetaType::Scalar(ref sm) => {
                    unify_scalar(var_name, var_pos, value, sm, errors, &path)
                }
                MetaType::Enum(ref em) => unify_enum(var_name, var_pos, value, em, errors, &path),
                MetaType::InputObject(ref iom) => {
                    unify_input_object(var_name, var_pos, value, iom, schema, errors, &path)
                }
                _ => panic!("Can't unify non-input concrete type"),
            }
        }
    }
}

fn unify_scalar<'a>(var_name: &str,
                    var_pos: &SourcePosition,
                    value: &InputValue,
                    meta: &ScalarMeta,
                    errors: &mut Vec<RuleError>,
                    path: &Path<'a>) {
    if !(meta.try_parse_fn)(value) {
        push_unification_error(errors,
                               var_name,
                               var_pos,
                               path,
                               &format!(r#"Expected "{}""#, meta.name));
        return;
    }

    match *value {
        InputValue::List(_) => {
            push_unification_error(errors,
                                   var_name,
                                   var_pos,
                                   path,
                                   &format!(r#"Expected "{}", found list"#, meta.name))
        }
        InputValue::Object(_) => {
            push_unification_error(errors,
                                   var_name,
                                   var_pos,
                                   path,
                                   &format!(r#"Expected "{}", found object"#, meta.name))
        }
        _ => (),
    }
}

fn unify_enum<'a>(var_name: &str,
                  var_pos: &SourcePosition,
                  value: &InputValue,
                  meta: &EnumMeta,
                  errors: &mut Vec<RuleError>,
                  path: &Path<'a>) {
    match *value {
        InputValue::String(ref name) |
        InputValue::Enum(ref name) => {
            if !meta.values.iter().any(|ev| &ev.name == name) {
                push_unification_error(errors,
                                       var_name,
                                       var_pos,
                                       path,
                                       &format!(r#"Invalid value for enum "{}""#, meta.name))
            }
        }
        _ => push_unification_error(
            errors,
            var_name,
            var_pos,
            path,
            &format!(r#"Expected "{}", found not a string or enum"#, meta.name),
        ),
    }
}

fn unify_input_object<'a>(var_name: &str,
                          var_pos: &SourcePosition,
                          value: &InputValue,
                          meta: &InputObjectMeta,
                          schema: &SchemaType,
                          errors: &mut Vec<RuleError>,
                          path: &Path<'a>) {
    if let Some(ref obj) = value.to_object_value() {
        let mut keys = obj.keys().collect::<HashSet<&&str>>();

        for input_field in &meta.input_fields {
            let mut has_value = false;
            keys.remove(&input_field.name.as_str());

            if let Some(value) = obj.get(input_field.name.as_str()) {
                if !value.is_null() {
                    has_value = true;

                    unify_value(var_name,
                                var_pos,
                                value,
                                &schema.make_type(&input_field.arg_type),
                                schema,
                                errors,
                                Path::ObjectField(&input_field.name, path));
                }
            }

            if !has_value && input_field.arg_type.is_non_null() {
                push_unification_error(
                    errors,
                    var_name,
                    var_pos,
                    &Path::ObjectField(&input_field.name, path),
                    &format!(r#"Expected "{}", found null"#, input_field.arg_type),
                );
            }
        }

        for key in keys {
            push_unification_error(errors,
                                   var_name,
                                   var_pos,
                                   &Path::ObjectField(key, path),
                                   "Unknown field");
        }
    } else {
        push_unification_error(errors,
                               var_name,
                               var_pos,
                               path,
                               &format!(r#"Expected "{}", found not an object"#, meta.name));
    }
}

fn is_absent_or_null(v: Option<&InputValue>) -> bool {
    v.map_or(true, InputValue::is_null)
}

fn push_unification_error<'a>(errors: &mut Vec<RuleError>,
                              var_name: &str,
                              var_pos: &SourcePosition,
                              path: &Path<'a>,
                              message: &str) {
    errors.push(RuleError::new(&format!(
            r#"Variable "${}" got invalid value. {}{}."#,
            var_name, path, message,
        ),
                               &[var_pos.clone()]));
}

impl<'a> fmt::Display for Path<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Path::Root => write!(f, ""),
            Path::ArrayElement(idx, prev) => write!(f, "{}In element #{}: ", prev, idx),
            Path::ObjectField(name, prev) => write!(f, r#"{}In field "{}": "#, prev, name),
        }
    }
}
