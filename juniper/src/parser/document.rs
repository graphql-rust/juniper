use std::borrow::Cow;

use crate::ast::{
    Arguments, Definition, Directive, Field, Fragment, FragmentSpread, InlineFragment, InputValue,
    Operation, OperationType, OwnedDocument, Selection, Type, VariableDefinition,
    VariableDefinitions,
};

use crate::{
    parser::{
        value::parse_value_literal, Lexer, OptionParseResult, ParseError, ParseResult, Parser,
        Spanning, Token, UnlocatedParseResult,
    },
    schema::{
        meta::{Argument, Field as MetaField},
        model::SchemaType,
    },
    value::ScalarValue,
};

#[doc(hidden)]
pub fn parse_document_source<'a, 'b, S>(
    s: &'a str,
    schema: &'b SchemaType<'b, S>,
) -> UnlocatedParseResult<OwnedDocument<'a, S>>
where
    S: ScalarValue,
{
    let mut lexer = Lexer::new(s);
    let mut parser = Parser::new(&mut lexer).map_err(|s| s.map(ParseError::LexerError))?;
    parse_document(&mut parser, schema)
}

fn parse_document<'a, 'b, S>(
    parser: &mut Parser<'a>,
    schema: &'b SchemaType<'b, S>,
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

fn parse_definition<'a, 'b, S>(
    parser: &mut Parser<'a>,
    schema: &'b SchemaType<'b, S>,
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

fn parse_operation_definition<'a, 'b, S>(
    parser: &mut Parser<'a>,
    schema: &'b SchemaType<'b, S>,
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

fn parse_fragment_definition<'a, 'b, S>(
    parser: &mut Parser<'a>,
    schema: &'b SchemaType<'b, S>,
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

fn parse_optional_selection_set<'a, 'b, S>(
    parser: &mut Parser<'a>,
    schema: &'b SchemaType<'b, S>,
    fields: Option<&[&MetaField<'b, S>]>,
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

fn parse_selection_set<'a, 'b, S>(
    parser: &mut Parser<'a>,
    schema: &'b SchemaType<'b, S>,
    fields: Option<&[&MetaField<'b, S>]>,
) -> ParseResult<Vec<Selection<'a, S>>>
where
    S: ScalarValue,
{
    parser.unlocated_delimited_nonempty_list(
        &Token::CurlyOpen,
        |p| parse_selection(p, schema, fields),
        &Token::CurlyClose,
    )
}

fn parse_selection<'a, 'b, S>(
    parser: &mut Parser<'a>,
    schema: &'b SchemaType<'b, S>,
    fields: Option<&[&MetaField<'b, S>]>,
) -> UnlocatedParseResult<Selection<'a, S>>
where
    S: ScalarValue,
{
    match parser.peek().item {
        Token::Ellipsis => parse_fragment(parser, schema, fields),
        _ => parse_field(parser, schema, fields).map(Selection::Field),
    }
}

fn parse_fragment<'a, 'b, S>(
    parser: &mut Parser<'a>,
    schema: &'b SchemaType<'b, S>,
    fields: Option<&[&MetaField<'b, S>]>,
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

fn parse_field<'a, 'b, S>(
    parser: &mut Parser<'a>,
    schema: &'b SchemaType<'b, S>,
    fields: Option<&[&MetaField<'b, S>]>,
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

fn parse_arguments<'a, 'b, S>(
    parser: &mut Parser<'a>,
    schema: &'b SchemaType<'b, S>,
    arguments: Option<&[Argument<'b, S>]>,
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

fn parse_argument<'a, 'b, S>(
    parser: &mut Parser<'a>,
    schema: &'b SchemaType<'b, S>,
    arguments: Option<&[Argument<'b, S>]>,
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

fn parse_variable_definitions<'a, 'b, S>(
    parser: &mut Parser<'a>,
    schema: &'b SchemaType<'b, S>,
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

fn parse_variable_definition<'a, 'b, S>(
    parser: &mut Parser<'a>,
    schema: &'b SchemaType<'b, S>,
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

fn parse_directives<'a, 'b, S>(
    parser: &mut Parser<'a>,
    schema: &'b SchemaType<'b, S>,
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

fn parse_directive<'a, 'b, S>(
    parser: &mut Parser<'a>,
    schema: &'b SchemaType<'b, S>,
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

pub fn parse_type<'a>(parser: &mut Parser<'a>) -> ParseResult<Type<'a>> {
    let parsed_type = if let Some(Spanning {
        span: ref start_span,
        ..
    }) = parser.skip(&Token::BracketOpen)?
    {
        let inner_type = parse_type(parser)?;
        let end_pos = parser.expect(&Token::BracketClose)?.span.end;
        Spanning::start_end(
            &start_span.start,
            &end_pos,
            Type::List(Box::new(inner_type.item), None),
        )
    } else {
        parser.expect_name()?.map(|s| Type::Named(Cow::Borrowed(s)))
    };

    Ok(match *parser.peek() {
        Spanning {
            item: Token::ExclamationMark,
            ..
        } => wrap_non_null(parser, parsed_type)?,
        _ => parsed_type,
    })
}

fn wrap_non_null<'a>(parser: &mut Parser<'a>, inner: Spanning<Type<'a>>) -> ParseResult<Type<'a>> {
    let end_pos = &parser.expect(&Token::ExclamationMark)?.span.end;

    let wrapped = match inner.item {
        Type::Named(name) => Type::NonNullNamed(name),
        Type::List(l, expected_size) => Type::NonNullList(l, expected_size),
        t => t,
    };

    Ok(Spanning::start_end(&inner.span.start, end_pos, wrapped))
}
