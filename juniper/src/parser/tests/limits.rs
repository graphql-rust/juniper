//! Tests of the document [`Limits`] that the parser enforces.

use crate::{
    parser::{Limits, ParseError, Spanning, document::parse_document_source},
    schema::model::{RootNode, SchemaType},
    validation::test_harness::{MutationRoot, QueryRoot, SubscriptionRoot},
    value::DefaultScalarValue,
};

/// Parses `source` against the test schema with the given `limits`.
fn parse(source: &str, limits: Limits) -> Result<(), Spanning<ParseError>> {
    let mut schema = SchemaType::new::<QueryRoot, MutationRoot, SubscriptionRoot>(&(), &(), &());
    schema.limits = limits;
    parse_document_source::<DefaultScalarValue>(source, &schema).map(drop)
}

fn depth_limit(max_depth: usize) -> Limits {
    Limits {
        max_depth,
        ..Limits::default()
    }
}

fn token_limit(max_tokens: usize) -> Limits {
    Limits {
        max_tokens,
        ..Limits::default()
    }
}

fn field_limit(max_fields: usize) -> Limits {
    Limits {
        max_fields,
        ..Limits::default()
    }
}

/// Builds a document with `levels` fragments that each spread the next fragment twice.
///
/// Following every spread of the document takes `2^levels` steps.
fn fragment_bomb(levels: usize) -> String {
    let mut source = "{ ...F0 }".to_owned();
    for i in 0..levels {
        source += &format!(
            " fragment F{i} on QueryRoot {{ ...F{j} ...F{j} }}",
            j = i + 1
        );
    }
    source + &format!(" fragment F{levels} on QueryRoot {{ dog {{ name }} }}")
}

/// Builds a document whose operation spreads a chain of `length` fragments.
fn fragment_chain(length: usize) -> String {
    let mut source = "{ ...F1 }".to_owned();
    for i in 1..length {
        source += &format!(" fragment F{i} on QueryRoot {{ ...F{} }}", i + 1);
    }
    source + &format!(" fragment F{length} on QueryRoot {{ dog {{ name }} }}")
}

mod depth {
    use super::{depth_limit, fragment_bomb, fragment_chain, parse};
    use crate::parser::{Limits, ParseError, SourcePosition};

    #[test]
    fn allows_selection_sets_at_limit() {
        assert_eq!(parse("{ a { a { a { a } } } }", depth_limit(4)), Ok(()));
    }

    #[test]
    fn rejects_selection_sets_over_limit() {
        let err = parse("{ a { a { a { a { a } } } } }", depth_limit(4)).unwrap_err();

        assert_eq!(err.item, ParseError::DepthLimitExceeded(4));
    }

    #[test]
    fn reports_position_of_first_part_over_limit() {
        let err = parse("{ a { a { a } } }", depth_limit(2)).unwrap_err();

        assert_eq!(err.span.start, SourcePosition::new(8, 0, 8));
    }

    #[test]
    fn reports_position_of_spread_over_limit() {
        let err = parse(&fragment_chain(3), depth_limit(4)).unwrap_err();

        // The `dog` field of the `F3` fragment nests too deep.
        let dog = fragment_chain(3).rfind("dog").unwrap();
        assert_eq!(err.span.start.index(), dog);
    }

    #[test]
    fn counts_list_values() {
        assert_eq!(parse("{ a(x: [[[1]]]) }", depth_limit(4)), Ok(()));
        assert_eq!(
            parse("{ a(x: [[[[1]]]]) }", depth_limit(4))
                .unwrap_err()
                .item,
            ParseError::DepthLimitExceeded(4),
        );
    }

    #[test]
    fn counts_object_values() {
        assert_eq!(parse("{ a(x: {b: {c: {d: 1}}}) }", depth_limit(4)), Ok(()));
        assert_eq!(
            parse("{ a(x: {b: {c: {d: {e: 1}}}}) }", depth_limit(4))
                .unwrap_err()
                .item,
            ParseError::DepthLimitExceeded(4),
        );
    }

    #[test]
    fn counts_list_types() {
        assert_eq!(
            parse("query($v: [[[[Int]]]]) { a }", depth_limit(4)),
            Ok(())
        );
        assert_eq!(
            parse("query($v: [[[[[Int]]]]]) { a }", depth_limit(4))
                .unwrap_err()
                .item,
            ParseError::DepthLimitExceeded(4),
        );
    }

    #[test]
    fn counts_fragment_spreads() {
        // The operation, 2 fragments, and the `dog` field nest 4 selection sets.
        assert_eq!(parse(&fragment_chain(2), depth_limit(4)), Ok(()));
        assert_eq!(
            parse(&fragment_chain(3), depth_limit(4)).unwrap_err().item,
            ParseError::DepthLimitExceeded(4),
        );
    }

    #[test]
    fn counts_spreads_of_unused_fragments() {
        let source = "{ dog { name } } \
                      fragment F1 on QueryRoot { ...F2 } \
                      fragment F2 on QueryRoot { ...F3 } \
                      fragment F3 on QueryRoot { ...F4 } \
                      fragment F4 on QueryRoot { dog { name } }";

        assert_eq!(
            parse(source, depth_limit(4)).unwrap_err().item,
            ParseError::DepthLimitExceeded(4),
        );
    }

    #[test]
    fn checks_repeated_spreads_once() {
        let limits = Limits {
            max_fields: usize::MAX,
            ..depth_limit(64)
        };

        assert_eq!(parse(&fragment_bomb(40), limits), Ok(()));
    }

    #[test]
    fn leaves_fragment_cycles_to_validation() {
        let source = "{ ...F } fragment F on QueryRoot { ...F }";

        assert_eq!(parse(source, depth_limit(4)), Ok(()));
    }

    #[test]
    fn leaves_undefined_fragments_to_validation() {
        assert_eq!(parse("{ ...Undefined }", depth_limit(4)), Ok(()));
    }
}

mod tokens {
    use super::{parse, token_limit};
    use crate::parser::{ParseError, SourcePosition};

    #[test]
    fn allows_tokens_at_limit() {
        assert_eq!(parse("{ a b }", token_limit(4)), Ok(()));
    }

    #[test]
    fn rejects_first_token_over_limit() {
        let err = parse("{ a b }", token_limit(3)).unwrap_err();

        assert_eq!(err.item, ParseError::TokenLimitExceeded(3));
        assert_eq!(err.span.start, SourcePosition::new(6, 0, 6));
    }
}

mod fields {
    use super::{field_limit, fragment_bomb, parse};
    use crate::parser::{Limits, ParseError, SourcePosition};

    #[test]
    fn allows_fields_at_limit() {
        assert_eq!(parse("{ a b { c } }", field_limit(3)), Ok(()));
    }

    #[test]
    fn rejects_operation_over_limit() {
        let err = parse("query Q { a b { c } }", field_limit(2)).unwrap_err();

        assert_eq!(err.item, ParseError::FieldLimitExceeded(2));
        assert_eq!(err.span.start, SourcePosition::new(0, 0, 0));
    }

    #[test]
    fn counts_fragment_fields_for_each_spread() {
        let source = "{ ...F ...F } fragment F on QueryRoot { a b }";

        assert_eq!(parse(source, field_limit(4)), Ok(()));
        assert_eq!(
            parse(source, field_limit(3)).unwrap_err().item,
            ParseError::FieldLimitExceeded(3),
        );
    }

    #[test]
    fn ignores_unused_fragments() {
        let source = "{ a } fragment F on QueryRoot { b c d }";

        assert_eq!(parse(source, field_limit(1)), Ok(()));
    }

    #[test]
    fn defaults_reject_exponential_expansion() {
        assert_eq!(
            parse(&fragment_bomb(40), Limits::default())
                .unwrap_err()
                .item,
            ParseError::FieldLimitExceeded(Limits::DEFAULT_MAX_FIELDS),
        );
    }
}

/// Default [`Limits`] reject documents that would overflow the 2 MiB stack of a test thread.
#[test]
fn defaults_reject_deep_nesting_without_stack_overflow() {
    let depth = 5_000;
    let source = format!("{{ a(x: {}1{}) }}", "[".repeat(depth), "]".repeat(depth));

    assert_eq!(
        parse(&source, Limits::default()).unwrap_err().item,
        ParseError::DepthLimitExceeded(Limits::DEFAULT_MAX_DEPTH),
    );
}

#[test]
fn defaults_reject_too_many_tokens() {
    let source = format!("{{ {} }}", "a ".repeat(Limits::DEFAULT_MAX_TOKENS));

    assert_eq!(
        parse(&source, Limits::default()).unwrap_err().item,
        ParseError::TokenLimitExceeded(Limits::DEFAULT_MAX_TOKENS),
    );
}

#[test]
fn root_node_sets_limits() {
    let root = RootNode::new(QueryRoot, MutationRoot, SubscriptionRoot)
        .limit_depth(3)
        .limit_tokens(7)
        .limit_fields(9);

    assert_eq!(
        root.schema.limits,
        Limits {
            max_depth: 3,
            max_tokens: 7,
            max_fields: 9,
        },
    );
}
