#[test]
fn test_object_compile_fail() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/object/fail.rs");
}
