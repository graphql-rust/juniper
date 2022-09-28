// TODO: [Object] Type Validation: §4 (interfaces) for objects
// TODO: [Non-Null] §1 A Non‐Null type must not wrap another Non‐Null type.

#[rustversion::nightly]
#[test]
fn test_failing_compilation() {
    let t = trybuild::TestCases::new();
    t.compile_fail("fail/**/*.rs");
}
