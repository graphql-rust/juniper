use std::collections::{HashMap, HashSet};

use crate::ast::{
    Arguments, Definition, Directive, Field, Fragment, FragmentSpread, InlineFragment, InputValue,
    Operation, OperationType, OwnedDocument, Selection, Type, VariableDefinition,
    VariableDefinitions,
};

use crate::{
    parser::{
        Lexer, Limits, OptionParseResult, ParseError, ParseResult, Parser, Span, Spanning, Token,
        UnlocatedParseResult, value::parse_value_literal,
    },
    schema::{
        meta::{Argument, Field as MetaField},
        model::SchemaType,
    },
    value::ScalarValue,
};

#[doc(hidden)]
pub fn parse_document_source<'a, S>(
    s: &'a str,
    schema: &SchemaType<S>,
) -> UnlocatedParseResult<OwnedDocument<'a, S>>
where
    S: ScalarValue,
{
    let mut lexer = Lexer::new(s);
    let mut parser = Parser::with_limits(&mut lexer, schema.limits)?;
    let document = parse_document(&mut parser, schema)?;
    ExpansionCheck::new(&document, schema.limits).run()?;
    Ok(document)
}

/// Check of a document with its fragment spreads expanded.
///
/// A fragment spread nests the selections of its fragment one level deeper. The parser limits
/// the depth of each definition on its own, but chained fragments can nest much deeper than any
/// single definition. Validation and execution follow fragment spreads recursively, so the depth
/// limit keeps them from overflowing the stack.
///
/// Fragments that spread a fragment more than once also multiply its fields. A document of a few
/// hundred tokens can expand into billions of fields. The field limit bounds the work of execution.
struct ExpansionCheck<'a, 'd, S> {
    document: &'d [Definition<'a, S>],
    fragments: HashMap<&'a str, &'d [Selection<'a, S>]>,
    limits: Limits,

    /// Expansions of the checked fragments, with depths relative to the depth of their spreads.
    ///
    /// Each fragment is checked only once, so fragments that spread the same fragment many times
    /// cannot make the check take exponential time.
    fragment_expansions: HashMap<&'a str, Expansion>,

    /// Fragments that the check is in. A spread of one of these fragments is a cycle, and the
    /// check ignores it. Validation reports fragment cycles.
    open_fragments: HashSet<&'a str>,
}

/// Depth and field count of a part of a document with its fragment spreads expanded.
#[derive(Clone, Copy)]
struct Expansion {
    /// Deepest nesting depth in the part.
    depth: usize,

    /// Number of fields in the part. The count saturates at [`usize::MAX`].
    fields: usize,
}

impl<'a, 'd, S> ExpansionCheck<'a, 'd, S> {
    fn new(document: &'d [Definition<'a, S>], limits: Limits) -> Self {
        let fragments = document
            .iter()
            .filter_map(|definition| match definition {
                Definition::Fragment(fragment) => Some((
                    fragment.item.name.item,
                    fragment.item.selection_set.as_slice(),
                )),
                Definition::Operation(_) => None,
            })
            .collect();
        Self {
            document,
            fragments,
            limits,
            fragment_expansions: HashMap::new(),
            open_fragments: HashSet::new(),
        }
    }

    /// Checks every operation and fragment of the document.
    ///
    /// Only operations have a field limit because only operations execute. The depth of unused
    /// fragments is checked too because validation follows their spreads as well.
    fn run(mut self) -> Result<(), Spanning<ParseError>> {
        for definition in self.document {
            match definition {
                Definition::Operation(operation) => {
                    let expansion = self.selection_set(&operation.item.selection_set, 1)?;
                    if expansion.fields > self.limits.max_fields {
                        return Err(Spanning::new(
                            operation.span,
                            ParseError::FieldLimitExceeded(self.limits.max_fields),
                        ));
                    }
                }
                Definition::Fragment(fragment) => {
                    self.fragment(fragment.item.name.item, 0, fragment.span)?;
                }
            }
        }
        Ok(())
    }

    /// Returns the expansion of `selection_set`, which starts at `depth`.
    fn selection_set(
        &mut self,
        selection_set: &'d [Selection<'a, S>],
        depth: usize,
    ) -> Result<Expansion, Spanning<ParseError>> {
        let mut expansion = Expansion { depth, fields: 0 };
        for selection in selection_set {
            let nested = match selection {
                Selection::Field(field) => {
                    expansion.fields = expansion.fields.saturating_add(1);
                    match &field.item.selection_set {
                        Some(nested) => self.nested(nested, depth, field.span)?,
                        None => continue,
                    }
                }
                Selection::InlineFragment(fragment) => {
                    self.nested(&fragment.item.selection_set, depth, fragment.span)?
                }
                Selection::FragmentSpread(spread) => {
                    self.fragment(spread.item.name.item, depth, spread.span)?
                }
            };
            expansion.depth = expansion.depth.max(nested.depth);
            expansion.fields = expansion.fields.saturating_add(nested.fields);
        }
        Ok(expansion)
    }

    /// Returns the expansion of `selection_set`, which nests one level below `depth`.
    ///
    /// Fails at `span` if the nested selection set exceeds the depth limit.
    fn nested(
        &mut self,
        selection_set: &'d [Selection<'a, S>],
        depth: usize,
        span: Span,
    ) -> Result<Expansion, Spanning<ParseError>> {
        if depth >= self.limits.max_depth {
            return Err(Spanning::new(
                span,
                ParseError::DepthLimitExceeded(self.limits.max_depth),
            ));
        }
        self.selection_set(selection_set, depth + 1)
    }

    /// Returns the expansion of the `name`d fragment, which is spread at `depth`.
    ///
    /// Fails at `span` if the fragment exceeds the depth limit. Undefined fragments and fragment
    /// cycles expand to nothing. Validation reports them.
    fn fragment(
        &mut self,
        name: &'a str,
        depth: usize,
        span: Span,
    ) -> Result<Expansion, Spanning<ParseError>> {
        if let Some(&relative) = self.fragment_expansions.get(name) {
            let depth = depth + relative.depth;
            if depth > self.limits.max_depth {
                return Err(Spanning::new(
                    span,
                    ParseError::DepthLimitExceeded(self.limits.max_depth),
                ));
            }
            return Ok(Expansion {
                depth,
                fields: relative.fields,
            });
        }
        let nothing = Expansion { depth, fields: 0 };
        let Some(&selection_set) = self.fragments.get(name) else {
            return Ok(nothing);
        };
        if !self.open_fragments.insert(name) {
            return Ok(nothing);
        }
        let expansion = self.nested(selection_set, depth, span)?;
        self.open_fragments.remove(name);
        self.fragment_expansions.insert(
            name,
            Expansion {
                depth: expansion.depth - depth,
                fields: expansion.fields,
            },
        );
        Ok(expansion)
    }
}

fn parse_document<'a, S>(
    parser: &mut Parser<'a>,
    schema: &SchemaType<S>,
) -> UnlocatedParseResult<OwnedDocument<'a, S>>
where
    S: ScalarValue,
{
    let mut defs = Vec::new();

    loop {
        defs.push(parse_definition(parser, schema)?);

        if parser.peek().item == Token::EndOfFile {
            return Ok(defs);
        }
    }
}

fn parse_definition<'a, S>(
    parser: &mut Parser<'a>,
    schema: &SchemaType<S>,
) -> UnlocatedParseResult<Definition<'a, S>>
where
    S: ScalarValue,
{
    match parser.peek().item {
        Token::CurlyOpen
        | Token::Name("query")
        | Token::Name("mutation")
        | Token::Name("subscription") => Ok(Definition::Operation(parse_operation_definition(
            parser, schema,
        )?)),
        Token::Name("fragment") => Ok(Definition::Fragment(parse_fragment_definition(
            parser, schema,
        )?)),
        _ => Err(parser.next_token()?.map(ParseError::unexpected_token)),
    }
}

fn parse_operation_definition<'a, S>(
    parser: &mut Parser<'a>,
    schema: &SchemaType<S>,
) -> ParseResult<Operation<'a, S>>
where
    S: ScalarValue,
{
    if parser.peek().item == Token::CurlyOpen {
        let fields = schema.concrete_query_type().fields(schema);
        let fields = fields.as_ref().map(|c| c as &[_]);
        let selection_set = parse_selection_set(parser, schema, fields)?;

        Ok(Spanning::new(
            selection_set.span,
            Operation {
                operation_type: OperationType::Query,
                name: None,
                variable_definitions: None,
                directives: None,
                selection_set: selection_set.item,
            },
        ))
    } else {
        let start_pos = parser.peek().span.start;
        let operation_type = parse_operation_type(parser)?;
        let op = match operation_type.item {
            OperationType::Query => Some(schema.concrete_query_type()),
            OperationType::Mutation => schema.concrete_mutation_type(),
            OperationType::Subscription => schema.concrete_subscription_type(),
        };
        let fields = op.and_then(|m| m.fields(schema));
        let fields = fields.as_ref().map(|c| c as &[_]);

        let name = match parser.peek().item {
            Token::Name(_) => Some(parser.expect_name()?),
            _ => None,
        };
        let variable_definitions = parse_variable_definitions(parser, schema)?;
        let directives = parse_directives(parser, schema)?;
        let selection_set = parse_selection_set(parser, schema, fields)?;

        Ok(Spanning::start_end(
            &start_pos,
            &selection_set.span.end,
            Operation {
                operation_type: operation_type.item,
                name,
                variable_definitions,
                directives: directives.map(|s| s.item),
                selection_set: selection_set.item,
            },
        ))
    }
}

fn parse_fragment_definition<'a, S>(
    parser: &mut Parser<'a>,
    schema: &SchemaType<S>,
) -> ParseResult<Fragment<'a, S>>
where
    S: ScalarValue,
{
    let start_pos = parser.expect(&Token::Name("fragment"))?.span.start;
    let name = match parser.expect_name() {
        Ok(n) => {
            if n.item == "on" {
                return Err(n.map(|_| ParseError::UnexpectedToken("on".into())));
            } else {
                n
            }
        }
        Err(e) => return Err(e),
    };

    parser.expect(&Token::Name("on"))?;
    let type_cond = parser.expect_name()?;

    let fields = schema
        .concrete_type_by_name(type_cond.item)
        .and_then(|m| m.fields(schema));
    let fields = fields.as_ref().map(|c| c as &[_]);

    let directives = parse_directives(parser, schema)?;
    let selection_set = parse_selection_set(parser, schema, fields)?;

    Ok(Spanning::start_end(
        &start_pos,
        &selection_set.span.end,
        Fragment {
            name,
            type_condition: type_cond,
            directives: directives.map(|s| s.item),
            selection_set: selection_set.item,
        },
    ))
}

fn parse_optional_selection_set<'a, S>(
    parser: &mut Parser<'a>,
    schema: &SchemaType<S>,
    fields: Option<&[&MetaField<S>]>,
) -> OptionParseResult<Vec<Selection<'a, S>>>
where
    S: ScalarValue,
{
    if parser.peek().item == Token::CurlyOpen {
        Ok(Some(parse_selection_set(parser, schema, fields)?))
    } else {
        Ok(None)
    }
}

fn parse_selection_set<'a, S>(
    parser: &mut Parser<'a>,
    schema: &SchemaType<S>,
    fields: Option<&[&MetaField<S>]>,
) -> ParseResult<Vec<Selection<'a, S>>>
where
    S: ScalarValue,
{
    parser.nested(|parser| {
        parser.unlocated_delimited_nonempty_list(
            &Token::CurlyOpen,
            |p| parse_selection(p, schema, fields),
            &Token::CurlyClose,
        )
    })
}

fn parse_selection<'a, S>(
    parser: &mut Parser<'a>,
    schema: &SchemaType<S>,
    fields: Option<&[&MetaField<S>]>,
) -> UnlocatedParseResult<Selection<'a, S>>
where
    S: ScalarValue,
{
    match parser.peek().item {
        Token::Ellipsis => parse_fragment(parser, schema, fields),
        _ => parse_field(parser, schema, fields).map(Selection::Field),
    }
}

fn parse_fragment<'a, S>(
    parser: &mut Parser<'a>,
    schema: &SchemaType<S>,
    fields: Option<&[&MetaField<S>]>,
) -> UnlocatedParseResult<Selection<'a, S>>
where
    S: ScalarValue,
{
    let start_pos = parser.expect(&Token::Ellipsis)?.span.start;

    match parser.peek().item {
        Token::Name("on") => {
            parser.next_token()?;
            let name = parser.expect_name()?;

            let fields = schema
                .concrete_type_by_name(name.item)
                .and_then(|m| m.fields(schema));
            let fields = fields.as_ref().map(|c| c as &[_]);
            let directives = parse_directives(parser, schema)?;
            let selection_set = parse_selection_set(parser, schema, fields)?;

            Ok(Selection::InlineFragment(Spanning::start_end(
                &start_pos,
                &selection_set.span.end,
                InlineFragment {
                    type_condition: Some(name),
                    directives: directives.map(|s| s.item),
                    selection_set: selection_set.item,
                },
            )))
        }
        Token::CurlyOpen => {
            let selection_set = parse_selection_set(parser, schema, fields)?;

            Ok(Selection::InlineFragment(Spanning::start_end(
                &start_pos,
                &selection_set.span.end,
                InlineFragment {
                    type_condition: None,
                    directives: None,
                    selection_set: selection_set.item,
                },
            )))
        }
        Token::Name(_) => {
            let frag_name = parser.expect_name()?;
            let directives = parse_directives(parser, schema)?;

            Ok(Selection::FragmentSpread(Spanning::start_end(
                &start_pos.clone(),
                &directives
                    .as_ref()
                    .map_or(&frag_name.span.end, |s| &s.span.end)
                    .clone(),
                FragmentSpread {
                    name: frag_name,
                    directives: directives.map(|s| s.item),
                },
            )))
        }
        Token::At => {
            let directives = parse_directives(parser, schema)?;
            let selection_set = parse_selection_set(parser, schema, fields)?;

            Ok(Selection::InlineFragment(Spanning::start_end(
                &start_pos,
                &selection_set.span.end,
                InlineFragment {
                    type_condition: None,
                    directives: directives.map(|s| s.item),
                    selection_set: selection_set.item,
                },
            )))
        }
        _ => Err(parser.next_token()?.map(ParseError::unexpected_token)),
    }
}

fn parse_field<'a, S>(
    parser: &mut Parser<'a>,
    schema: &SchemaType<S>,
    fields: Option<&[&MetaField<S>]>,
) -> ParseResult<Field<'a, S>>
where
    S: ScalarValue,
{
    let mut alias = Some(parser.expect_name()?);

    let name = if parser.skip(&Token::Colon)?.is_some() {
        parser.expect_name()?
    } else {
        alias.take().unwrap()
    };

    let field = fields.and_then(|f| f.iter().find(|f| f.name == name.item));
    let args = field
        .as_ref()
        .and_then(|f| f.arguments.as_ref().map(|a| a as &[_]));

    let fields = field
        .as_ref()
        .and_then(|f| schema.lookup_type(&f.field_type))
        .and_then(|m| m.fields(schema));
    let fields = fields.as_ref().map(|c| c as &[_]);

    let arguments = parse_arguments(parser, schema, args)?;

    let directives = parse_directives(parser, schema)?;
    let selection_set = parse_optional_selection_set(parser, schema, fields)?;

    Ok(Spanning::start_end(
        &alias.as_ref().unwrap_or(&name).span.start,
        &selection_set
            .as_ref()
            .map(|s| &s.span.end)
            .or_else(|| directives.as_ref().map(|s| &s.span.end))
            .or_else(|| arguments.as_ref().map(|s| &s.span.end))
            .unwrap_or(&name.span.end)
            .clone(),
        Field {
            alias,
            name,
            arguments,
            directives: directives.map(|s| s.item),
            selection_set: selection_set.map(|s| s.item),
        },
    ))
}

fn parse_arguments<'a, S>(
    parser: &mut Parser<'a>,
    schema: &SchemaType<S>,
    arguments: Option<&[Argument<S>]>,
) -> OptionParseResult<Arguments<'a, S>>
where
    S: ScalarValue,
{
    if parser.peek().item != Token::ParenOpen {
        Ok(None)
    } else {
        Ok(Some(
            parser
                .delimited_nonempty_list(
                    &Token::ParenOpen,
                    |p| parse_argument(p, schema, arguments),
                    &Token::ParenClose,
                )?
                .map(|args| Arguments {
                    items: args.into_iter().map(|s| s.item).collect(),
                }),
        ))
    }
}

fn parse_argument<'a, S>(
    parser: &mut Parser<'a>,
    schema: &SchemaType<S>,
    arguments: Option<&[Argument<S>]>,
) -> ParseResult<(Spanning<&'a str>, Spanning<InputValue<S>>)>
where
    S: ScalarValue,
{
    let name = parser.expect_name()?;
    let tpe = arguments
        .and_then(|args| args.iter().find(|a| a.name == name.item))
        .and_then(|arg| schema.lookup_type(&arg.arg_type));

    parser.expect(&Token::Colon)?;
    let value = parse_value_literal(parser, false, schema, tpe)?;

    Ok(Spanning::start_end(
        &name.span.start,
        &value.span.end.clone(),
        (name, value),
    ))
}

fn parse_operation_type(parser: &mut Parser<'_>) -> ParseResult<OperationType> {
    match parser.peek().item {
        Token::Name("query") => Ok(parser.next_token()?.map(|_| OperationType::Query)),
        Token::Name("mutation") => Ok(parser.next_token()?.map(|_| OperationType::Mutation)),
        Token::Name("subscription") => {
            Ok(parser.next_token()?.map(|_| OperationType::Subscription))
        }
        _ => Err(parser.next_token()?.map(ParseError::unexpected_token)),
    }
}

fn parse_variable_definitions<'a, S>(
    parser: &mut Parser<'a>,
    schema: &SchemaType<S>,
) -> OptionParseResult<VariableDefinitions<'a, S>>
where
    S: ScalarValue,
{
    if parser.peek().item != Token::ParenOpen {
        Ok(None)
    } else {
        Ok(Some(
            parser
                .delimited_nonempty_list(
                    &Token::ParenOpen,
                    |p| parse_variable_definition(p, schema),
                    &Token::ParenClose,
                )?
                .map(|defs| VariableDefinitions {
                    items: defs.into_iter().map(|s| s.item).collect(),
                }),
        ))
    }
}

fn parse_variable_definition<'a, S>(
    parser: &mut Parser<'a>,
    schema: &SchemaType<S>,
) -> ParseResult<(Spanning<&'a str>, VariableDefinition<'a, S>)>
where
    S: ScalarValue,
{
    let start_pos = parser.expect(&Token::Dollar)?.span.start;
    let var_name = parser.expect_name()?;
    parser.expect(&Token::Colon)?;
    let var_type = parse_type(parser)?;
    let tpe = schema.lookup_type(&var_type.item);

    let default_value = if parser.skip(&Token::Equals)?.is_some() {
        Some(parse_value_literal(parser, true, schema, tpe)?)
    } else {
        None
    };

    let directives = parse_directives(parser, schema)?;

    Ok(Spanning::start_end(
        &start_pos,
        &default_value
            .as_ref()
            .map_or(&var_type.span.end, |s| &s.span.end)
            .clone(),
        (
            Spanning::start_end(&start_pos, &var_name.span.end, var_name.item),
            VariableDefinition {
                var_type,
                default_value,
                directives: directives.map(|s| s.item),
            },
        ),
    ))
}

fn parse_directives<'a, S>(
    parser: &mut Parser<'a>,
    schema: &SchemaType<S>,
) -> OptionParseResult<Vec<Spanning<Directive<'a, S>>>>
where
    S: ScalarValue,
{
    if parser.peek().item != Token::At {
        Ok(None)
    } else {
        let mut items = Vec::new();
        while parser.peek().item == Token::At {
            items.push(parse_directive(parser, schema)?);
        }

        Ok(Spanning::spanning(items))
    }
}

fn parse_directive<'a, S>(
    parser: &mut Parser<'a>,
    schema: &SchemaType<S>,
) -> ParseResult<Directive<'a, S>>
where
    S: ScalarValue,
{
    let start_pos = parser.expect(&Token::At)?.span.start;
    let name = parser.expect_name()?;

    let directive = schema.directive_by_name(name.item);

    let arguments = parse_arguments(
        parser,
        schema,
        directive.as_ref().map(|d| &d.arguments as &[_]),
    )?;

    Ok(Spanning::start_end(
        &start_pos,
        &arguments
            .as_ref()
            .map_or(&name.span.end, |s| &s.span.end)
            .clone(),
        Directive { name, arguments },
    ))
}

pub fn parse_type<'a>(parser: &mut Parser<'a>) -> ParseResult<Type<&'a str>> {
    let parsed_type = if let Some(Spanning {
        span: start_span, ..
    }) = parser.skip(&Token::BracketOpen)?
    {
        let inner_type = parser.nested(parse_type)?;
        let end_pos = parser.expect(&Token::BracketClose)?.span.end;
        Spanning::start_end(&start_span.start, &end_pos, inner_type.item.wrap_list(None))
    } else {
        parser.expect_name()?.map(Type::nullable)
    };

    Ok(match *parser.peek() {
        Spanning {
            item: Token::ExclamationMark,
            ..
        } => wrap_non_null(parser, parsed_type)?,
        _ => parsed_type,
    })
}

fn wrap_non_null<'a>(
    parser: &mut Parser<'a>,
    mut inner: Spanning<Type<&'a str>>,
) -> ParseResult<Type<&'a str>> {
    let end_pos = &parser.expect(&Token::ExclamationMark)?.span.end;

    if !inner.item.is_non_null() {
        inner.item = inner.item.wrap_non_null();
    }

    Ok(Spanning::start_end(&inner.span.start, end_pos, inner.item))
}
