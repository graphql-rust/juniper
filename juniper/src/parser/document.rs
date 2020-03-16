use std::borrow::Cow;

use crate::ast::{
    Arguments, Definition, Directive, Document, Field, Fragment, FragmentSpread, InlineFragment,
    InputValue, Operation, OperationType, Selection, Type, VariableDefinition, VariableDefinitions,
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
) -> UnlocatedParseResult<'a, Document<'a, S>>
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
) -> UnlocatedParseResult<'a, Document<'a, S>>
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
) -> UnlocatedParseResult<'a, Definition<'a, S>>
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
        _ => Err(parser.next_token()?.map(ParseError::UnexpectedToken)),
    }
}

fn parse_operation_definition<'a, 'b, S>(
    parser: &mut Parser<'a>,
    schema: &'b SchemaType<'b, S>,
) -> ParseResult<'a, Operation<'a, S>>
where
    S: ScalarValue,
{
    if parser.peek().item == Token::CurlyOpen {
        let fields = schema.concrete_query_type().fields(schema);
        let fields = fields.as_ref().map(|c| c as &[_]);
        let selection_set = parse_selection_set(parser, schema, fields)?;

        Ok(Spanning::start_end(
            &selection_set.start,
            &selection_set.end,
            Operation {
                operation_type: OperationType::Query,
                name: None,
                variable_definitions: None,
                directives: None,
                selection_set: selection_set.item,
            },
        ))
    } else {
        let start_pos = parser.peek().start;
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
            &selection_set.end,
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
) -> ParseResult<'a, Fragment<'a, S>>
where
    S: ScalarValue,
{
    let Spanning {
        start: start_pos, ..
    } = parser.expect(&Token::Name("fragment"))?;
    let name = match parser.expect_name() {
        Ok(n) => {
            if n.item == "on" {
                return Err(n.map(|_| ParseError::UnexpectedToken(Token::Name("on"))));
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
        &selection_set.end,
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
) -> OptionParseResult<'a, Vec<Selection<'a, S>>>
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
) -> ParseResult<'a, Vec<Selection<'a, S>>>
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
) -> UnlocatedParseResult<'a, Selection<'a, S>>
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
) -> UnlocatedParseResult<'a, Selection<'a, S>>
where
    S: ScalarValue,
{
    let Spanning {
        start: ref start_pos,
        ..
    } = parser.expect(&Token::Ellipsis)?;

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
                &start_pos.clone(),
                &selection_set.end,
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
                &start_pos.clone(),
                &selection_set.end,
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
                    .map_or(&frag_name.end, |s| &s.end)
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
                &start_pos.clone(),
                &selection_set.end,
                InlineFragment {
                    type_condition: None,
                    directives: directives.map(|s| s.item),
                    selection_set: selection_set.item,
                },
            )))
        }
        _ => Err(parser.next_token()?.map(ParseError::UnexpectedToken)),
    }
}

fn parse_field<'a, 'b, S>(
    parser: &mut Parser<'a>,
    schema: &'b SchemaType<'b, S>,
    fields: Option<&[&MetaField<'b, S>]>,
) -> ParseResult<'a, Field<'a, S>>
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
        &alias.as_ref().unwrap_or(&name).start.clone(),
        &selection_set
            .as_ref()
            .map(|s| &s.end)
            .or_else(|| directives.as_ref().map(|s| &s.end))
            .or_else(|| arguments.as_ref().map(|s| &s.end))
            .unwrap_or(&name.end)
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
) -> OptionParseResult<'a, Arguments<'a, S>>
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
) -> ParseResult<'a, (Spanning<&'a str>, Spanning<InputValue<S>>)>
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
        &name.start.clone(),
        &value.end.clone(),
        (name, value),
    ))
}

fn parse_operation_type<'a>(parser: &mut Parser<'a>) -> ParseResult<'a, OperationType> {
    match parser.peek().item {
        Token::Name("query") => Ok(parser.next_token()?.map(|_| OperationType::Query)),
        Token::Name("mutation") => Ok(parser.next_token()?.map(|_| OperationType::Mutation)),
        Token::Name("subscription") => {
            Ok(parser.next_token()?.map(|_| OperationType::Subscription))
        }
        _ => Err(parser.next_token()?.map(ParseError::UnexpectedToken)),
    }
}

fn parse_variable_definitions<'a, 'b, S>(
    parser: &mut Parser<'a>,
    schema: &'b SchemaType<'b, S>,
) -> OptionParseResult<'a, VariableDefinitions<'a, S>>
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
) -> ParseResult<'a, (Spanning<&'a str>, VariableDefinition<'a, S>)>
where
    S: ScalarValue,
{
    let Spanning {
        start: start_pos, ..
    } = parser.expect(&Token::Dollar)?;
    let var_name = parser.expect_name()?;
    parser.expect(&Token::Colon)?;
    let var_type = parse_type(parser)?;
    let tpe = schema.lookup_type(&var_type.item);

    let default_value = if parser.skip(&Token::Equals)?.is_some() {
        Some(parse_value_literal(parser, true, schema, tpe)?)
    } else {
        None
    };

    Ok(Spanning::start_end(
        &start_pos,
        &default_value
            .as_ref()
            .map_or(&var_type.end, |s| &s.end)
            .clone(),
        (
            Spanning::start_end(&start_pos, &var_name.end, var_name.item),
            VariableDefinition {
                var_type,
                default_value,
            },
        ),
    ))
}

fn parse_directives<'a, 'b, S>(
    parser: &mut Parser<'a>,
    schema: &'b SchemaType<'b, S>,
) -> OptionParseResult<'a, Vec<Spanning<Directive<'a, S>>>>
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
) -> ParseResult<'a, Directive<'a, S>>
where
    S: ScalarValue,
{
    let Spanning {
        start: start_pos, ..
    } = parser.expect(&Token::At)?;
    let name = parser.expect_name()?;

    let directive = schema.directive_by_name(name.item);

    let arguments = parse_arguments(
        parser,
        schema,
        directive.as_ref().map(|d| &d.arguments as &[_]),
    )?;

    Ok(Spanning::start_end(
        &start_pos,
        &arguments.as_ref().map_or(&name.end, |s| &s.end).clone(),
        Directive { name, arguments },
    ))
}

pub fn parse_type<'a>(parser: &mut Parser<'a>) -> ParseResult<'a, Type<'a>> {
    let parsed_type = if let Some(Spanning {
        start: start_pos, ..
    }) = parser.skip(&Token::BracketOpen)?
    {
        let inner_type = parse_type(parser)?;
        let Spanning { end: end_pos, .. } = parser.expect(&Token::BracketClose)?;
        Spanning::start_end(&start_pos, &end_pos, Type::List(Box::new(inner_type.item)))
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

fn wrap_non_null<'a>(
    parser: &mut Parser<'a>,
    inner: Spanning<Type<'a>>,
) -> ParseResult<'a, Type<'a>> {
    let Spanning { end: end_pos, .. } = parser.expect(&Token::ExclamationMark)?;

    let wrapped = match inner.item {
        Type::Named(name) => Type::NonNullNamed(name),
        Type::List(l) => Type::NonNullList(l),
        t => t,
    };

    Ok(Spanning::start_end(&inner.start, &end_pos, wrapped))
}
