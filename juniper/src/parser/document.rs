use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
};

use crate::{
    ast::{
        Arguments, Definition, Directive, Field, Fragment, FragmentSpread, InlineFragment,
        InputValue, Operation, OperationType, OwnedDocument, Selection, Type, VariableDefinition,
        VariablesDefinition,
    },
    parser::{
        Lexer, OptionParseResult, ParseError, ParseResult, Parser, ScalarToken, Span, Spanning,
        Token, UnlocatedParseResult, value::parse_value_literal,
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
    SpreadDepthCheck::new(&document, schema.limits.max_depth).run()?;
    Ok(document)
}

/// Check of the nesting depth that fragment spreads add to a document.
///
/// The parser limits the depth of each definition on its own. A fragment spread nests the
/// selections of its fragment one level deeper. Chained fragments can thus nest much deeper than
/// any single definition. Validation and execution follow fragment spreads recursively, so this
/// check keeps them from overflowing the stack.
struct SpreadDepthCheck<'a, 'd, S> {
    document: &'d [Definition<'a, S>],
    fragments: HashMap<&'a str, &'d [Selection<'a, S>]>,
    max_depth: usize,

    /// Depths of the checked fragments, relative to the depth of their spreads.
    ///
    /// Each fragment is checked only once, so fragments that spread the same fragment many times
    /// cannot make the check take exponential time.
    relative_depths: HashMap<&'a str, usize>,

    /// Fragments that the check is in. A spread of one of these fragments is a cycle, and the
    /// check ignores it. Validation reports fragment cycles.
    open_fragments: HashSet<&'a str>,
}

impl<'a, 'd, S> SpreadDepthCheck<'a, 'd, S> {
    fn new(document: &'d [Definition<'a, S>], max_depth: usize) -> Self {
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
            max_depth,
            relative_depths: HashMap::new(),
            open_fragments: HashSet::new(),
        }
    }

    /// Checks every operation and fragment of the document.
    ///
    /// Unused fragments are checked too because validation follows their spreads as well.
    fn run(mut self) -> Result<(), Spanning<ParseError>> {
        for definition in self.document {
            match definition {
                Definition::Operation(operation) => {
                    self.selection_set_depth(&operation.item.selection_set, 1)?;
                }
                Definition::Fragment(fragment) => {
                    self.fragment_depth(fragment.item.name.item, 0, fragment.span)?;
                }
            }
        }
        Ok(())
    }

    /// Returns the deepest depth inside `selection_set`, which starts at `depth`.
    fn selection_set_depth(
        &mut self,
        selection_set: &'d [Selection<'a, S>],
        depth: usize,
    ) -> Result<usize, Spanning<ParseError>> {
        let mut deepest = depth;
        for selection in selection_set {
            let selection_depth = match selection {
                Selection::Field(field) => match &field.item.selection_set {
                    Some(nested) => self.nested_depth(nested, depth, field.span)?,
                    None => depth,
                },
                Selection::InlineFragment(fragment) => {
                    self.nested_depth(&fragment.item.selection_set, depth, fragment.span)?
                }
                Selection::FragmentSpread(spread) => {
                    self.fragment_depth(spread.item.name.item, depth, spread.span)?
                }
            };
            deepest = deepest.max(selection_depth);
        }
        Ok(deepest)
    }

    /// Returns the deepest depth inside `selection_set`, which nests one level below `depth`.
    ///
    /// Fails at `span` if the nested selection set exceeds the depth limit.
    fn nested_depth(
        &mut self,
        selection_set: &'d [Selection<'a, S>],
        depth: usize,
        span: Span,
    ) -> Result<usize, Spanning<ParseError>> {
        if depth >= self.max_depth {
            return Err(Spanning::new(
                span,
                ParseError::DepthLimitExceeded(self.max_depth),
            ));
        }
        self.selection_set_depth(selection_set, depth + 1)
    }

    /// Returns the deepest depth inside the `name`d fragment, which is spread at `depth`.
    ///
    /// Fails at `span` if the fragment exceeds the depth limit. Undefined fragments and fragment
    /// cycles add no depth. Validation reports them.
    fn fragment_depth(
        &mut self,
        name: &'a str,
        depth: usize,
        span: Span,
    ) -> Result<usize, Spanning<ParseError>> {
        if let Some(&relative_depth) = self.relative_depths.get(name) {
            return if depth + relative_depth > self.max_depth {
                Err(Spanning::new(
                    span,
                    ParseError::DepthLimitExceeded(self.max_depth),
                ))
            } else {
                Ok(depth + relative_depth)
            };
        }
        let Some(&selection_set) = self.fragments.get(name) else {
            return Ok(depth);
        };
        if !self.open_fragments.insert(name) {
            return Ok(depth);
        }
        let deepest = self.nested_depth(selection_set, depth, span)?;
        self.open_fragments.remove(name);
        self.relative_depths.insert(name, deepest - depth);
        Ok(deepest)
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
    let description = parse_description(parser)?;

    let mut def = match parser.peek().item {
        // Descriptions are not permitted on query shorthand.
        // See: https://spec.graphql.org/September2025#sel-GAFTRJABAByBz7P
        Token::CurlyOpen if description.is_some() => {
            return Err(parser.next_token()?.map(ParseError::unexpected_token));
        }
        Token::CurlyOpen
        | Token::Name("query")
        | Token::Name("mutation")
        | Token::Name("subscription") => {
            Definition::Operation(parse_operation_definition(parser, schema)?)
        }
        Token::Name("fragment") => Definition::Fragment(parse_fragment_definition(parser, schema)?),
        _ => return Err(parser.next_token()?.map(ParseError::unexpected_token)),
    };
    def.set_description(description);
    Ok(def)
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
                description: None,
                variables_definition: None,
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
        let variables_definition = parse_variables_definition(parser, schema)?;
        let directives = parse_directives(parser, schema)?;
        let selection_set = parse_selection_set(parser, schema, fields)?;

        Ok(Spanning::start_end(
            &start_pos,
            &selection_set.span.end,
            Operation {
                operation_type: operation_type.item,
                name,
                description: None,
                variables_definition,
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
    let name = parser.expect_name()?;
    if name.item == "on" {
        return Err(name.map(|_| ParseError::UnexpectedToken("on".into())));
    }

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
            description: None,
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

fn parse_variables_definition<'a, S>(
    parser: &mut Parser<'a>,
    schema: &SchemaType<S>,
) -> OptionParseResult<VariablesDefinition<'a, S>>
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
                .map(|defs| VariablesDefinition {
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
    let description = parse_description(parser)?;

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
                description,
                var_type,
                default_value,
                directives: directives.map(|s| s.item),
            },
        ),
    ))
}

fn parse_description<'a>(parser: &mut Parser<'a>) -> OptionParseResult<Cow<'a, str>> {
    if !matches!(parser.peek().item, Token::Scalar(ScalarToken::String(_))) {
        Ok(None)
    } else {
        let token = parser.next_token()?;
        let Token::Scalar(ScalarToken::String(lit)) = token.item else {
            unreachable!("already checked to be `ScalarToken::String`")
        };
        Ok(Some(Spanning::new(
            token.span,
            lit.parse().map_err(|e| Spanning::new(token.span, e))?,
        )))
    }
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
