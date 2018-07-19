use std::borrow::Cow;

use ast::{
    Arguments, Definition, Directive, Document, Field, Fragment, FragmentSpread, InlineFragment,
    InputValue, Operation, OperationType, Selection, Type, VariableDefinitions,
};
use parser::Spanning;
use schema::meta::Argument;
use validation::{ValidatorContext, Visitor};

#[doc(hidden)]
pub fn visit<'a, V: Visitor<'a>>(v: &mut V, ctx: &mut ValidatorContext<'a>, d: &'a Document) {
    v.enter_document(ctx, d);
    visit_definitions(v, ctx, d);
    v.exit_document(ctx, d);
}

fn visit_definitions<'a, V: Visitor<'a>>(
    v: &mut V,
    ctx: &mut ValidatorContext<'a>,
    d: &'a Vec<Definition>,
) {
    for def in d {
        let def_type = match *def {
            Definition::Fragment(Spanning {
                item:
                    Fragment {
                        type_condition: Spanning { item: name, .. },
                        ..
                    },
                ..
            }) => Some(Type::NonNullNamed(Cow::Borrowed(name))),
            Definition::Operation(Spanning {
                item:
                    Operation {
                        operation_type: OperationType::Query,
                        ..
                    },
                ..
            }) => Some(Type::NonNullNamed(Cow::Borrowed(
                ctx.schema.concrete_query_type().name().unwrap(),
            ))),
            Definition::Operation(Spanning {
                item:
                    Operation {
                        operation_type: OperationType::Mutation,
                        ..
                    },
                ..
            }) => ctx.schema
                .concrete_mutation_type()
                .map(|t| Type::NonNullNamed(Cow::Borrowed(t.name().unwrap()))),
        };

        ctx.with_pushed_type(def_type.as_ref(), |ctx| {
            enter_definition(v, ctx, def);
            visit_definition(v, ctx, def);
            exit_definition(v, ctx, def);
        });
    }
}

fn enter_definition<'a, V: Visitor<'a>>(
    v: &mut V,
    ctx: &mut ValidatorContext<'a>,
    def: &'a Definition,
) {
    match *def {
        Definition::Operation(ref op) => v.enter_operation_definition(ctx, op),
        Definition::Fragment(ref f) => v.enter_fragment_definition(ctx, f),
    }
}

fn exit_definition<'a, V: Visitor<'a>>(
    v: &mut V,
    ctx: &mut ValidatorContext<'a>,
    def: &'a Definition,
) {
    match *def {
        Definition::Operation(ref op) => v.exit_operation_definition(ctx, op),
        Definition::Fragment(ref f) => v.exit_fragment_definition(ctx, f),
    }
}

fn visit_definition<'a, V: Visitor<'a>>(
    v: &mut V,
    ctx: &mut ValidatorContext<'a>,
    def: &'a Definition,
) {
    match *def {
        Definition::Operation(ref op) => {
            visit_variable_definitions(v, ctx, &op.item.variable_definitions);
            visit_directives(v, ctx, &op.item.directives);
            visit_selection_set(v, ctx, &op.item.selection_set);
        }
        Definition::Fragment(ref f) => {
            visit_directives(v, ctx, &f.item.directives);
            visit_selection_set(v, ctx, &f.item.selection_set);
        }
    }
}

fn visit_variable_definitions<'a, V: Visitor<'a>>(
    v: &mut V,
    ctx: &mut ValidatorContext<'a>,
    defs: &'a Option<Spanning<VariableDefinitions>>,
) {
    if let Some(ref defs) = *defs {
        for def in defs.item.iter() {
            let var_type = def.1.var_type.item.clone();

            ctx.with_pushed_input_type(Some(&var_type), |ctx| {
                v.enter_variable_definition(ctx, def);

                if let Some(ref default_value) = def.1.default_value {
                    visit_input_value(v, ctx, default_value);
                }

                v.exit_variable_definition(ctx, def);
            })
        }
    }
}

fn visit_directives<'a, V: Visitor<'a>>(
    v: &mut V,
    ctx: &mut ValidatorContext<'a>,
    directives: &'a Option<Vec<Spanning<Directive>>>,
) {
    if let Some(ref directives) = *directives {
        for directive in directives {
            let directive_arguments = ctx.schema
                .directive_by_name(directive.item.name.item)
                .map(|d| &d.arguments);

            v.enter_directive(ctx, directive);
            visit_arguments(v, ctx, &directive_arguments, &directive.item.arguments);
            v.exit_directive(ctx, directive);
        }
    }
}

fn visit_arguments<'a, V: Visitor<'a>>(
    v: &mut V,
    ctx: &mut ValidatorContext<'a>,
    meta_args: &Option<&Vec<Argument<'a>>>,
    arguments: &'a Option<Spanning<Arguments>>,
) {
    if let Some(ref arguments) = *arguments {
        for argument in arguments.item.iter() {
            let arg_type = meta_args
                .and_then(|args| args.iter().find(|a| a.name == argument.0.item))
                .map(|a| &a.arg_type);

            ctx.with_pushed_input_type(arg_type, |ctx| {
                v.enter_argument(ctx, argument);

                visit_input_value(v, ctx, &argument.1);

                v.exit_argument(ctx, argument);
            })
        }
    }
}

fn visit_selection_set<'a, V: Visitor<'a>>(
    v: &mut V,
    ctx: &mut ValidatorContext<'a>,
    selection_set: &'a Vec<Selection>,
) {
    ctx.with_pushed_parent_type(|ctx| {
        v.enter_selection_set(ctx, selection_set);

        for selection in selection_set.iter() {
            visit_selection(v, ctx, selection);
        }

        v.exit_selection_set(ctx, selection_set);
    });
}

fn visit_selection<'a, V: Visitor<'a>>(
    v: &mut V,
    ctx: &mut ValidatorContext<'a>,
    selection: &'a Selection,
) {
    match *selection {
        Selection::Field(ref field) => visit_field(v, ctx, field),
        Selection::FragmentSpread(ref spread) => visit_fragment_spread(v, ctx, spread),
        Selection::InlineFragment(ref fragment) => visit_inline_fragment(v, ctx, fragment),
    }
}

fn visit_field<'a, V: Visitor<'a>>(
    v: &mut V,
    ctx: &mut ValidatorContext<'a>,
    field: &'a Spanning<Field>,
) {
    let meta_field = ctx.parent_type()
        .and_then(|t| t.field_by_name(field.item.name.item));

    let field_type = meta_field.map(|f| &f.field_type);
    let field_args = meta_field.and_then(|f| f.arguments.as_ref());

    ctx.with_pushed_type(field_type, |ctx| {
        v.enter_field(ctx, field);

        visit_arguments(v, ctx, &field_args, &field.item.arguments);
        visit_directives(v, ctx, &field.item.directives);

        if let Some(ref selection_set) = field.item.selection_set {
            visit_selection_set(v, ctx, selection_set);
        }

        v.exit_field(ctx, field);
    });
}

fn visit_fragment_spread<'a, V: Visitor<'a>>(
    v: &mut V,
    ctx: &mut ValidatorContext<'a>,
    spread: &'a Spanning<FragmentSpread>,
) {
    v.enter_fragment_spread(ctx, spread);

    visit_directives(v, ctx, &spread.item.directives);

    v.exit_fragment_spread(ctx, spread);
}

fn visit_inline_fragment<'a, V: Visitor<'a>>(
    v: &mut V,
    ctx: &mut ValidatorContext<'a>,
    fragment: &'a Spanning<InlineFragment>,
) {
    let mut visit_fn = move |ctx: &mut ValidatorContext<'a>| {
        v.enter_inline_fragment(ctx, fragment);

        visit_directives(v, ctx, &fragment.item.directives);
        visit_selection_set(v, ctx, &fragment.item.selection_set);

        v.exit_inline_fragment(ctx, fragment);
    };

    if let Some(Spanning {
        item: type_name, ..
    }) = fragment.item.type_condition
    {
        ctx.with_pushed_type(
            Some(&Type::NonNullNamed(Cow::Borrowed(type_name))),
            visit_fn,
        );
    } else {
        visit_fn(ctx);
    }
}

fn visit_input_value<'a, V: Visitor<'a>>(
    v: &mut V,
    ctx: &mut ValidatorContext<'a>,
    input_value: &'a Spanning<InputValue>,
) {
    enter_input_value(v, ctx, input_value);

    match input_value.item {
        InputValue::Object(ref fields) => for field in fields {
            let inner_type = ctx.current_input_type_literal()
                .and_then(|t| match *t {
                    Type::NonNullNamed(ref name) | Type::Named(ref name) => {
                        ctx.schema.concrete_type_by_name(name)
                    }
                    _ => None,
                })
                .and_then(|ct| ct.input_field_by_name(&field.0.item))
                .map(|f| &f.arg_type);

            ctx.with_pushed_input_type(inner_type, |ctx| {
                v.enter_object_field(ctx, field);
                visit_input_value(v, ctx, &field.1);
                v.exit_object_field(ctx, field);
            })
        },
        InputValue::List(ref ls) => {
            let inner_type = ctx.current_input_type_literal().and_then(|t| match *t {
                Type::List(ref inner) | Type::NonNullList(ref inner) => {
                    Some(inner.as_ref().clone())
                }
                _ => None,
            });

            ctx.with_pushed_input_type(inner_type.as_ref(), |ctx| {
                for value in ls {
                    visit_input_value(v, ctx, value);
                }
            })
        }
        _ => (),
    }

    exit_input_value(v, ctx, input_value);
}

fn enter_input_value<'a, V: Visitor<'a>>(
    v: &mut V,
    ctx: &mut ValidatorContext<'a>,
    input_value: &'a Spanning<InputValue>,
) {
    use InputValue::*;

    let start = &input_value.start;
    let end = &input_value.end;

    match input_value.item {
        Null => v.enter_null_value(ctx, Spanning::start_end(start, end, ())),
        Int(ref i) => v.enter_int_value(ctx, Spanning::start_end(start, end, *i)),
        Float(ref f) => v.enter_float_value(ctx, Spanning::start_end(start, end, *f)),
        String(ref s) => v.enter_string_value(ctx, Spanning::start_end(start, end, s)),
        Boolean(ref b) => v.enter_boolean_value(ctx, Spanning::start_end(start, end, *b)),
        Enum(ref s) => v.enter_enum_value(ctx, Spanning::start_end(start, end, s)),
        Variable(ref s) => v.enter_variable_value(ctx, Spanning::start_end(start, end, s)),
        List(ref l) => v.enter_list_value(ctx, Spanning::start_end(start, end, l)),
        Object(ref o) => v.enter_object_value(ctx, Spanning::start_end(start, end, o)),
    }
}

fn exit_input_value<'a, V: Visitor<'a>>(
    v: &mut V,
    ctx: &mut ValidatorContext<'a>,
    input_value: &'a Spanning<InputValue>,
) {
    use InputValue::*;

    let start = &input_value.start;
    let end = &input_value.end;

    match input_value.item {
        Null => v.exit_null_value(ctx, Spanning::start_end(start, end, ())),
        Int(ref i) => v.exit_int_value(ctx, Spanning::start_end(start, end, *i)),
        Float(ref f) => v.exit_float_value(ctx, Spanning::start_end(start, end, *f)),
        String(ref s) => v.exit_string_value(ctx, Spanning::start_end(start, end, s)),
        Boolean(ref b) => v.exit_boolean_value(ctx, Spanning::start_end(start, end, *b)),
        Enum(ref s) => v.exit_enum_value(ctx, Spanning::start_end(start, end, s)),
        Variable(ref s) => v.exit_variable_value(ctx, Spanning::start_end(start, end, s)),
        List(ref l) => v.exit_list_value(ctx, Spanning::start_end(start, end, l)),
        Object(ref o) => v.exit_object_value(ctx, Spanning::start_end(start, end, o)),
    }
}
