use std::borrow::Cow;

use ast::{Arguments, Definition, Directive, Document, Field, Fragment, FragmentSpread,
          InlineFragment, InputValue, Operation, OperationType, Selection, Type,
          VariableDefinition, VariableDefinitions};

use parser::{Lexer, OptionParseResult, ParseError, ParseResult, Parser, Spanning, Token,
             UnlocatedParseResult};
use parser::value::parse_value_literal;

#[doc(hidden)]
pub fn parse_document_source(s: &str) -> UnlocatedParseResult<Document> {
    let mut lexer = Lexer::new(s);
    let mut parser = Parser::new(&mut lexer).map_err(|s| s.map(ParseError::LexerError))?;
    parse_document(&mut parser)
}

fn parse_document<'a>(parser: &mut Parser<'a>) -> UnlocatedParseResult<'a, Document<'a>> {
    let mut defs = Vec::new();

    loop {
        defs.push(parse_definition(parser)?);

        if parser.peek().item == Token::EndOfFile {
            return Ok(defs);
        }
    }
}

fn parse_definition<'a>(parser: &mut Parser<'a>) -> UnlocatedParseResult<'a, Definition<'a>> {
    match parser.peek().item {
        Token::CurlyOpen | Token::Name("query") | Token::Name("mutation") => {
            Ok(Definition::Operation(parse_operation_definition(parser)?))
        }
        Token::Name("fragment") => Ok(Definition::Fragment(parse_fragment_definition(parser)?)),
        _ => Err(parser.next()?.map(ParseError::UnexpectedToken)),
    }
}

fn parse_operation_definition<'a>(parser: &mut Parser<'a>) -> ParseResult<'a, Operation<'a>> {
    if parser.peek().item == Token::CurlyOpen {
        let selection_set = parse_selection_set(parser)?;

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
        let start_pos = parser.peek().start.clone();
        let operation_type = parse_operation_type(parser)?;
        let name = match parser.peek().item {
            Token::Name(_) => Some(parser.expect_name()?),
            _ => None,
        };
        let variable_definitions = parse_variable_definitions(parser)?;
        let directives = parse_directives(parser)?;
        let selection_set = parse_selection_set(parser)?;

        Ok(Spanning::start_end(
            &start_pos,
            &selection_set.end,
            Operation {
                operation_type: operation_type.item,
                name: name,
                variable_definitions: variable_definitions,
                directives: directives.map(|s| s.item),
                selection_set: selection_set.item,
            },
        ))
    }
}

fn parse_fragment_definition<'a>(parser: &mut Parser<'a>) -> ParseResult<'a, Fragment<'a>> {
    let Spanning {
        start: start_pos, ..
    } = parser.expect(&Token::Name("fragment"))?;
    let name = match parser.expect_name() {
        Ok(n) => if n.item == "on" {
            return Err(n.map(|_| ParseError::UnexpectedToken(Token::Name("on"))));
        } else {
            n
        },
        Err(e) => return Err(e),
    };

    parser.expect(&Token::Name("on"))?;
    let type_cond = parser.expect_name()?;
    let directives = parse_directives(parser)?;
    let selection_set = parse_selection_set(parser)?;

    Ok(Spanning::start_end(
        &start_pos,
        &selection_set.end,
        Fragment {
            name: name,
            type_condition: type_cond,
            directives: directives.map(|s| s.item),
            selection_set: selection_set.item,
        },
    ))
}

fn parse_optional_selection_set<'a>(
    parser: &mut Parser<'a>,
) -> OptionParseResult<'a, Vec<Selection<'a>>> {
    if parser.peek().item == Token::CurlyOpen {
        Ok(Some(parse_selection_set(parser)?))
    } else {
        Ok(None)
    }
}

fn parse_selection_set<'a>(parser: &mut Parser<'a>) -> ParseResult<'a, Vec<Selection<'a>>> {
    parser.unlocated_delimited_nonempty_list(&Token::CurlyOpen, parse_selection, &Token::CurlyClose)
}

fn parse_selection<'a>(parser: &mut Parser<'a>) -> UnlocatedParseResult<'a, Selection<'a>> {
    match parser.peek().item {
        Token::Ellipsis => parse_fragment(parser),
        _ => parse_field(parser).map(Selection::Field),
    }
}

fn parse_fragment<'a>(parser: &mut Parser<'a>) -> UnlocatedParseResult<'a, Selection<'a>> {
    let Spanning {
        start: ref start_pos,
        ..
    } = parser.expect(&Token::Ellipsis)?;

    match parser.peek().item {
        Token::Name("on") => {
            parser.next()?;
            let name = parser.expect_name()?;
            let directives = parse_directives(parser)?;
            let selection_set = parse_selection_set(parser)?;

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
            let selection_set = parse_selection_set(parser)?;

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
            let directives = parse_directives(parser)?;

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
            let directives = parse_directives(parser)?;
            let selection_set = parse_selection_set(parser)?;

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
        _ => Err(parser.next()?.map(ParseError::UnexpectedToken)),
    }
}

fn parse_field<'a>(parser: &mut Parser<'a>) -> ParseResult<'a, Field<'a>> {
    let mut alias = Some(parser.expect_name()?);

    let name = if parser.skip(&Token::Colon)?.is_some() {
        parser.expect_name()?
    } else {
        alias.take().unwrap()
    };

    let arguments = parse_arguments(parser)?;
    let directives = parse_directives(parser)?;
    let selection_set = parse_optional_selection_set(parser)?;

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
            alias: alias,
            name: name,
            arguments: arguments,
            directives: directives.map(|s| s.item),
            selection_set: selection_set.map(|s| s.item),
        },
    ))
}

fn parse_arguments<'a>(parser: &mut Parser<'a>) -> OptionParseResult<'a, Arguments<'a>> {
    if parser.peek().item != Token::ParenOpen {
        Ok(None)
    } else {
        Ok(Some(
            parser
                .delimited_nonempty_list(&Token::ParenOpen, parse_argument, &Token::ParenClose)?
                .map(|args| Arguments {
                    items: args.into_iter().map(|s| s.item).collect(),
                }),
        ))
    }
}

fn parse_argument<'a>(
    parser: &mut Parser<'a>,
) -> ParseResult<'a, (Spanning<&'a str>, Spanning<InputValue>)> {
    let name = parser.expect_name()?;
    parser.expect(&Token::Colon)?;
    let value = parse_value_literal(parser, false)?;

    Ok(Spanning::start_end(
        &name.start.clone(),
        &value.end.clone(),
        (name, value),
    ))
}

fn parse_operation_type<'a>(parser: &mut Parser<'a>) -> ParseResult<'a, OperationType> {
    match parser.peek().item {
        Token::Name("query") => Ok(parser.next()?.map(|_| OperationType::Query)),
        Token::Name("mutation") => Ok(parser.next()?.map(|_| OperationType::Mutation)),
        _ => Err(parser.next()?.map(ParseError::UnexpectedToken)),
    }
}

fn parse_variable_definitions<'a>(
    parser: &mut Parser<'a>,
) -> OptionParseResult<'a, VariableDefinitions<'a>> {
    if parser.peek().item != Token::ParenOpen {
        Ok(None)
    } else {
        Ok(Some(
            parser
                .delimited_nonempty_list(
                    &Token::ParenOpen,
                    parse_variable_definition,
                    &Token::ParenClose,
                )?
                .map(|defs| VariableDefinitions {
                    items: defs.into_iter().map(|s| s.item).collect(),
                }),
        ))
    }
}

fn parse_variable_definition<'a>(
    parser: &mut Parser<'a>,
) -> ParseResult<'a, (Spanning<&'a str>, VariableDefinition<'a>)> {
    let Spanning {
        start: start_pos, ..
    } = parser.expect(&Token::Dollar)?;
    let var_name = parser.expect_name()?;
    parser.expect(&Token::Colon)?;
    let var_type = parse_type(parser)?;

    let default_value = if parser.skip(&Token::Equals)?.is_some() {
        Some(parse_value_literal(parser, true)?)
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
                var_type: var_type,
                default_value: default_value,
            },
        ),
    ))
}

fn parse_directives<'a>(
    parser: &mut Parser<'a>,
) -> OptionParseResult<'a, Vec<Spanning<Directive<'a>>>> {
    if parser.peek().item != Token::At {
        Ok(None)
    } else {
        let mut items = Vec::new();
        while parser.peek().item == Token::At {
            items.push(parse_directive(parser)?);
        }

        Ok(Spanning::spanning(items))
    }
}

fn parse_directive<'a>(parser: &mut Parser<'a>) -> ParseResult<'a, Directive<'a>> {
    let Spanning {
        start: start_pos, ..
    } = parser.expect(&Token::At)?;
    let name = parser.expect_name()?;
    let arguments = parse_arguments(parser)?;

    Ok(Spanning::start_end(
        &start_pos,
        &arguments.as_ref().map_or(&name.end, |s| &s.end).clone(),
        Directive {
            name: name,
            arguments: arguments,
        },
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
