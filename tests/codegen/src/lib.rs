//! Tests for codegen compilation errors.

// TODO: [Object] Type Validation: §4 (interfaces) for objects
// TODO: [Non-Null] §1 A Non‐Null type must not wrap another Non‐Null type.

#[cfg(test)]
mod for_codegen_tests_only {
    use derive_more as _;
    use futures as _;
    use juniper as _;
    use serde as _;
}

#[rustversion::stable]
#[test]
fn test_failing_compilation() {
    let t = trybuild::TestCases::new();
    t.compile_fail("fail/**/*.rs");
}
