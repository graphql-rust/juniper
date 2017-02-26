use ast::{Document, Operation, Fragment, VariableDefinition, Selection,
          Directive, InputValue, Field, FragmentSpread, InlineFragment};
use parser::Spanning;
use validation::{ValidatorContext, Visitor};

#[doc(hidden)]
pub struct MultiVisitor<'a> {
    visitors: Vec<Box<Visitor<'a> + 'a>>
}

impl<'a> MultiVisitor<'a> {
    #[doc(hidden)]
    pub fn new(visitors: Vec<Box<Visitor<'a> + 'a>>) -> MultiVisitor<'a> {
        MultiVisitor {
            visitors: visitors
        }
    }

    fn visit_all<F: FnMut(&mut Box<Visitor<'a> + 'a>) -> ()>(&mut self, mut f: F) {
        for mut v in &mut self.visitors {
            f(v);
        }
    }
}

impl<'a> Visitor<'a> for MultiVisitor<'a> {
    fn enter_document(&mut self, ctx: &mut ValidatorContext<'a>, doc: &'a Document) {
        self.visit_all(|v| v.enter_document(ctx, doc));
    }

    fn exit_document(&mut self, ctx: &mut ValidatorContext<'a>, doc: &'a Document) {
        self.visit_all(|v| v.exit_document(ctx, doc));
    }

    fn enter_operation_definition(&mut self, ctx: &mut ValidatorContext<'a>, op: &'a Spanning<Operation>) {
        self.visit_all(|v| v.enter_operation_definition(ctx, op));
    }
    fn exit_operation_definition(&mut self, ctx: &mut ValidatorContext<'a>, op: &'a Spanning<Operation>) {
        self.visit_all(|v| v.exit_operation_definition(ctx, op));
    }

    fn enter_fragment_definition(&mut self, ctx: &mut ValidatorContext<'a>, f: &'a Spanning<Fragment>) {
        self.visit_all(|v| v.enter_fragment_definition(ctx, f));
    }
    fn exit_fragment_definition(&mut self, ctx: &mut ValidatorContext<'a>, f: &'a Spanning<Fragment>) {
        self.visit_all(|v| v.exit_fragment_definition(ctx, f));
    }

    fn enter_variable_definition(&mut self, ctx: &mut ValidatorContext<'a>, def: &'a (Spanning<&'a str>, VariableDefinition)) {
        self.visit_all(|v| v.enter_variable_definition(ctx, def));
    }
    fn exit_variable_definition(&mut self, ctx: &mut ValidatorContext<'a>, def: &'a (Spanning<&'a str>, VariableDefinition)) {
        self.visit_all(|v| v.exit_variable_definition(ctx, def));
    }

    fn enter_directive(&mut self, ctx: &mut ValidatorContext<'a>, d: &'a Spanning<Directive>) {
        self.visit_all(|v| v.enter_directive(ctx, d));
    }
    fn exit_directive(&mut self, ctx: &mut ValidatorContext<'a>, d: &'a Spanning<Directive>) {
        self.visit_all(|v| v.exit_directive(ctx, d));
    }

    fn enter_argument(&mut self, ctx: &mut ValidatorContext<'a>, arg: &'a (Spanning<&'a str>, Spanning<InputValue>)) {
        self.visit_all(|v| v.enter_argument(ctx, arg));
    }
    fn exit_argument(&mut self, ctx: &mut ValidatorContext<'a>, arg: &'a (Spanning<&'a str>, Spanning<InputValue>)) {
        self.visit_all(|v| v.exit_argument(ctx, arg));
    }

    fn enter_selection_set(&mut self, ctx: &mut ValidatorContext<'a>, s: &'a Vec<Selection>) {
        self.visit_all(|v| v.enter_selection_set(ctx, s));
    }
    fn exit_selection_set(&mut self, ctx: &mut ValidatorContext<'a>, s: &'a Vec<Selection>) {
        self.visit_all(|v| v.exit_selection_set(ctx, s));
    }

    fn enter_field(&mut self, ctx: &mut ValidatorContext<'a>, f: &'a Spanning<Field>) {
        self.visit_all(|v| v.enter_field(ctx, f));
    }
    fn exit_field(&mut self, ctx: &mut ValidatorContext<'a>, f: &'a Spanning<Field>) {
        self.visit_all(|v| v.exit_field(ctx, f));
    }

    fn enter_fragment_spread(&mut self, ctx: &mut ValidatorContext<'a>, s: &'a Spanning<FragmentSpread>) {
        self.visit_all(|v| v.enter_fragment_spread(ctx, s));
    }
    fn exit_fragment_spread(&mut self, ctx: &mut ValidatorContext<'a>, s: &'a Spanning<FragmentSpread>) {
        self.visit_all(|v| v.exit_fragment_spread(ctx, s));
    }

    fn enter_inline_fragment(&mut self, ctx: &mut ValidatorContext<'a>, f: &'a Spanning<InlineFragment>) {
        self.visit_all(|v| v.enter_inline_fragment(ctx, f));
    }
    fn exit_inline_fragment(&mut self, ctx: &mut ValidatorContext<'a>, f: &'a Spanning<InlineFragment>) {
        self.visit_all(|v| v.exit_inline_fragment(ctx, f));
    }

    fn enter_null_value(&mut self, ctx: &mut ValidatorContext<'a>, n: Spanning<()>) {
        self.visit_all(|v| v.enter_null_value(ctx, n.clone()));
    }
    fn exit_null_value(&mut self, ctx: &mut ValidatorContext<'a>, n: Spanning<()>) {
        self.visit_all(|v| v.exit_null_value(ctx, n.clone()));
    }

    fn enter_int_value(&mut self, ctx: &mut ValidatorContext<'a>, i: Spanning<i64>) {
        self.visit_all(|v| v.enter_int_value(ctx, i.clone()));
    }
    fn exit_int_value(&mut self, ctx: &mut ValidatorContext<'a>, i: Spanning<i64>) {
        self.visit_all(|v| v.exit_int_value(ctx, i.clone()));
    }

    fn enter_float_value(&mut self, ctx: &mut ValidatorContext<'a>, f: Spanning<f64>) {
        self.visit_all(|v| v.enter_float_value(ctx, f.clone()));
    }
    fn exit_float_value(&mut self, ctx: &mut ValidatorContext<'a>, f: Spanning<f64>) {
        self.visit_all(|v| v.exit_float_value(ctx, f.clone()));
    }

    fn enter_string_value(&mut self, ctx: &mut ValidatorContext<'a>, s: Spanning<&'a String>) {
        self.visit_all(|v| v.enter_string_value(ctx, s.clone()));
    }
    fn exit_string_value(&mut self, ctx: &mut ValidatorContext<'a>, s: Spanning<&'a String>) {
        self.visit_all(|v| v.exit_string_value(ctx, s.clone()));
    }

    fn enter_boolean_value(&mut self, ctx: &mut ValidatorContext<'a>, b: Spanning<bool>) {
        self.visit_all(|v| v.enter_boolean_value(ctx, b.clone()));
    }
    fn exit_boolean_value(&mut self, ctx: &mut ValidatorContext<'a>, b: Spanning<bool>) {
        self.visit_all(|v| v.exit_boolean_value(ctx, b.clone()));
    }

    fn enter_enum_value(&mut self, ctx: &mut ValidatorContext<'a>, s: Spanning<&'a String>) {
        self.visit_all(|v| v.enter_enum_value(ctx, s.clone()));
    }
    fn exit_enum_value(&mut self, ctx: &mut ValidatorContext<'a>, s: Spanning<&'a String>) {
        self.visit_all(|v| v.exit_enum_value(ctx, s.clone()));
    }

    fn enter_variable_value(&mut self, ctx: &mut ValidatorContext<'a>, s: Spanning<&'a String>) {
        self.visit_all(|v| v.enter_variable_value(ctx, s.clone()));
    }
    fn exit_variable_value(&mut self, ctx: &mut ValidatorContext<'a>, s: Spanning<&'a String>) {
        self.visit_all(|v| v.exit_variable_value(ctx, s.clone()));
    }

    fn enter_list_value(&mut self, ctx: &mut ValidatorContext<'a>, l: Spanning<&'a Vec<Spanning<InputValue>>>) {
        self.visit_all(|v| v.enter_list_value(ctx, l.clone()));
    }
    fn exit_list_value(&mut self, ctx: &mut ValidatorContext<'a>, l: Spanning<&'a Vec<Spanning<InputValue>>>) {
        self.visit_all(|v| v.exit_list_value(ctx, l.clone()));
    }

    fn enter_object_value(&mut self, ctx: &mut ValidatorContext<'a>, o: Spanning<&'a Vec<(Spanning<String>, Spanning<InputValue>)>>) {
        self.visit_all(|v| v.enter_object_value(ctx, o.clone()));
    }
    fn exit_object_value(&mut self, ctx: &mut ValidatorContext<'a>, o: Spanning<&'a Vec<(Spanning<String>, Spanning<InputValue>)>>) {
        self.visit_all(|v| v.exit_object_value(ctx, o.clone()));
    }

    fn enter_object_field(&mut self, ctx: &mut ValidatorContext<'a>, f: &'a (Spanning<String>, Spanning<InputValue>)) {
        self.visit_all(|v| v.enter_object_field(ctx, f));
    }
    fn exit_object_field(&mut self, ctx: &mut ValidatorContext<'a>, f: &'a (Spanning<String>, Spanning<InputValue>)) {
        self.visit_all(|v| v.exit_object_field(ctx, f));
    }
}
