use crate::{
    ast::{
        Directive, Document, Field, Fragment, FragmentSpread, InlineFragment, InputValue,
        Operation, Selection, VariableDefinition,
    },
    parser::Spanning,
    validation::{ValidatorContext, Visitor},
    DefaultScalarValue,
};

#[doc(hidden)]
pub struct MultiVisitorNil;

impl MultiVisitorNil {
    pub fn with<V>(self, visitor: V) -> MultiVisitorCons<V, Self> {
        MultiVisitorCons(visitor, self)
    }
}

#[doc(hidden)]
pub struct MultiVisitorCons<A, B>(A, B);

impl<A, B> MultiVisitorCons<A, B> {
    pub fn with<V>(self, visitor: V) -> MultiVisitorCons<V, Self> {
        MultiVisitorCons(visitor, self)
    }
}

impl<'a> Visitor<'a> for MultiVisitorNil {}

impl<'a, A, B> Visitor<'a> for MultiVisitorCons<A, B>
where
    A: Visitor<'a> + 'a,
    B: Visitor<'a> + 'a,
{
    fn enter_document(&mut self, ctx: &mut ValidatorContext<'a>, doc: &'a Document) {
        self.0.enter_document(ctx, doc);
        self.1.enter_document(ctx, doc);
    }
    fn exit_document(&mut self, ctx: &mut ValidatorContext<'a>, doc: &'a Document) {
        self.0.exit_document(ctx, doc);
        self.1.exit_document(ctx, doc);
    }

    fn enter_operation_definition(
        &mut self,
        ctx: &mut ValidatorContext<'a>,
        op: &'a Spanning<Operation>,
    ) {
        self.0.enter_operation_definition(ctx, op);
        self.1.enter_operation_definition(ctx, op);
    }
    fn exit_operation_definition(
        &mut self,
        ctx: &mut ValidatorContext<'a>,
        op: &'a Spanning<Operation>,
    ) {
        self.0.exit_operation_definition(ctx, op);
        self.1.exit_operation_definition(ctx, op);
    }

    fn enter_fragment_definition(
        &mut self,
        ctx: &mut ValidatorContext<'a>,
        f: &'a Spanning<Fragment>,
    ) {
        self.0.enter_fragment_definition(ctx, f);
        self.1.enter_fragment_definition(ctx, f);
    }
    fn exit_fragment_definition(
        &mut self,
        ctx: &mut ValidatorContext<'a>,
        f: &'a Spanning<Fragment>,
    ) {
        self.0.exit_fragment_definition(ctx, f);
        self.1.exit_fragment_definition(ctx, f);
    }

    fn enter_variable_definition(
        &mut self,
        ctx: &mut ValidatorContext<'a>,
        def: &'a (Spanning<&'a str>, VariableDefinition),
    ) {
        self.0.enter_variable_definition(ctx, def);
        self.1.enter_variable_definition(ctx, def);
    }
    fn exit_variable_definition(
        &mut self,
        ctx: &mut ValidatorContext<'a>,
        def: &'a (Spanning<&'a str>, VariableDefinition),
    ) {
        self.0.exit_variable_definition(ctx, def);
        self.1.exit_variable_definition(ctx, def);
    }

    fn enter_directive(&mut self, ctx: &mut ValidatorContext<'a>, d: &'a Spanning<Directive>) {
        self.0.enter_directive(ctx, d);
        self.1.enter_directive(ctx, d);
    }
    fn exit_directive(&mut self, ctx: &mut ValidatorContext<'a>, d: &'a Spanning<Directive>) {
        self.0.exit_directive(ctx, d);
        self.1.exit_directive(ctx, d);
    }

    fn enter_argument(
        &mut self,
        ctx: &mut ValidatorContext<'a>,
        arg: &'a (Spanning<&'a str>, Spanning<InputValue>),
    ) {
        self.0.enter_argument(ctx, arg);
        self.1.enter_argument(ctx, arg);
    }
    fn exit_argument(
        &mut self,
        ctx: &mut ValidatorContext<'a>,
        arg: &'a (Spanning<&'a str>, Spanning<InputValue>),
    ) {
        self.0.exit_argument(ctx, arg);
        self.1.exit_argument(ctx, arg);
    }

    fn enter_selection_set(&mut self, ctx: &mut ValidatorContext<'a>, s: &'a [Selection]) {
        self.0.enter_selection_set(ctx, s);
        self.1.enter_selection_set(ctx, s);
    }
    fn exit_selection_set(&mut self, ctx: &mut ValidatorContext<'a>, s: &'a [Selection]) {
        self.0.exit_selection_set(ctx, s);
        self.1.exit_selection_set(ctx, s);
    }

    fn enter_field(&mut self, ctx: &mut ValidatorContext<'a>, f: &'a Spanning<Field>) {
        self.0.enter_field(ctx, f);
        self.1.enter_field(ctx, f);
    }
    fn exit_field(&mut self, ctx: &mut ValidatorContext<'a>, f: &'a Spanning<Field>) {
        self.0.exit_field(ctx, f);
        self.1.exit_field(ctx, f);
    }

    fn enter_fragment_spread(
        &mut self,
        ctx: &mut ValidatorContext<'a>,
        s: &'a Spanning<FragmentSpread>,
    ) {
        self.0.enter_fragment_spread(ctx, s);
        self.1.enter_fragment_spread(ctx, s);
    }
    fn exit_fragment_spread(
        &mut self,
        ctx: &mut ValidatorContext<'a>,
        s: &'a Spanning<FragmentSpread>,
    ) {
        self.0.exit_fragment_spread(ctx, s);
        self.1.exit_fragment_spread(ctx, s);
    }

    fn enter_inline_fragment(
        &mut self,
        ctx: &mut ValidatorContext<'a>,
        f: &'a Spanning<InlineFragment>,
    ) {
        self.0.enter_inline_fragment(ctx, f);
        self.1.enter_inline_fragment(ctx, f);
    }
    fn exit_inline_fragment(
        &mut self,
        ctx: &mut ValidatorContext<'a>,
        f: &'a Spanning<InlineFragment>,
    ) {
        self.0.exit_inline_fragment(ctx, f);
        self.1.exit_inline_fragment(ctx, f);
    }

    fn enter_null_value(&mut self, ctx: &mut ValidatorContext<'a>, n: Spanning<()>) {
        self.0.enter_null_value(ctx, n);
        self.1.enter_null_value(ctx, n);
    }
    fn exit_null_value(&mut self, ctx: &mut ValidatorContext<'a>, n: Spanning<()>) {
        self.0.exit_null_value(ctx, n);
        self.1.exit_null_value(ctx, n);
    }

    fn enter_scalar_value(
        &mut self,
        ctx: &mut ValidatorContext<'a>,
        n: Spanning<&'a DefaultScalarValue>,
    ) {
        self.0.enter_scalar_value(ctx, n);
        self.1.enter_scalar_value(ctx, n);
    }
    fn exit_scalar_value(
        &mut self,
        ctx: &mut ValidatorContext<'a>,
        n: Spanning<&'a DefaultScalarValue>,
    ) {
        self.0.exit_scalar_value(ctx, n);
        self.1.exit_scalar_value(ctx, n);
    }

    fn enter_enum_value(&mut self, ctx: &mut ValidatorContext<'a>, s: Spanning<&'a String>) {
        self.0.enter_enum_value(ctx, s);
        self.1.enter_enum_value(ctx, s);
    }
    fn exit_enum_value(&mut self, ctx: &mut ValidatorContext<'a>, s: Spanning<&'a String>) {
        self.0.exit_enum_value(ctx, s);
        self.1.exit_enum_value(ctx, s);
    }

    fn enter_variable_value(&mut self, ctx: &mut ValidatorContext<'a>, s: Spanning<&'a String>) {
        self.0.enter_variable_value(ctx, s);
        self.1.enter_variable_value(ctx, s);
    }
    fn exit_variable_value(&mut self, ctx: &mut ValidatorContext<'a>, s: Spanning<&'a String>) {
        self.0.exit_variable_value(ctx, s);
        self.1.exit_variable_value(ctx, s);
    }

    fn enter_list_value(
        &mut self,
        ctx: &mut ValidatorContext<'a>,
        l: Spanning<&'a Vec<Spanning<InputValue>>>,
    ) {
        self.0.enter_list_value(ctx, l);
        self.1.enter_list_value(ctx, l);
    }
    fn exit_list_value(
        &mut self,
        ctx: &mut ValidatorContext<'a>,
        l: Spanning<&'a Vec<Spanning<InputValue>>>,
    ) {
        self.0.exit_list_value(ctx, l);
        self.1.exit_list_value(ctx, l);
    }

    fn enter_object_value(
        &mut self,
        ctx: &mut ValidatorContext<'a>,
        o: Spanning<&'a Vec<(Spanning<String>, Spanning<InputValue>)>>,
    ) {
        self.0.enter_object_value(ctx, o);
        self.1.enter_object_value(ctx, o);
    }
    fn exit_object_value(
        &mut self,
        ctx: &mut ValidatorContext<'a>,
        o: Spanning<&'a Vec<(Spanning<String>, Spanning<InputValue>)>>,
    ) {
        self.0.exit_object_value(ctx, o);
        self.1.exit_object_value(ctx, o);
    }

    fn enter_object_field(
        &mut self,
        ctx: &mut ValidatorContext<'a>,
        f: &'a (Spanning<String>, Spanning<InputValue>),
    ) {
        self.0.enter_object_field(ctx, f);
        self.1.enter_object_field(ctx, f);
    }
    fn exit_object_field(
        &mut self,
        ctx: &mut ValidatorContext<'a>,
        f: &'a (Spanning<String>, Spanning<InputValue>),
    ) {
        self.0.exit_object_field(ctx, f);
        self.1.exit_object_field(ctx, f);
    }
}
