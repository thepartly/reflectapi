use std::collections::HashMap;

#[test]
fn compiler_error_cases() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/errors/*.rs");
}

#[test]
fn compiler_success_cases() {
    let t = trybuild::TestCases::new();
    t.pass("tests/success/*.rs");
}

#[derive(reflect::Input, serde::Deserialize)]
struct TestStructOneBasicFieldU32 {
    _f: u32,
}
#[test]
fn test_reflect_struct_one_basic_field_u32() {
    assert_input_snapshot!(TestStructOneBasicFieldU32);
}

#[derive(reflect::Input, serde::Deserialize)]
struct TestStructOneBasicFieldString {
    _f: String,
}
#[test]
fn test_reflect_struct_one_basic_field_string() {
    assert_input_snapshot!(TestStructOneBasicFieldString);
}

#[derive(reflect::Input, reflect::Output, serde::Deserialize, serde::Serialize)]
struct TestStructOneBasicFieldStringReflectBoth {
    _f: String,
}
#[test]
fn test_reflect_struct_one_basic_field_string_reflect_both() {
    assert_snapshot!(TestStructOneBasicFieldStringReflectBoth);
}

#[derive(reflect::Input, reflect::Output, serde::Deserialize, serde::Serialize)]
struct TestStructOneBasicFieldStringReflectBothDifferently {
    #[reflect(output_type = "u32", input_type = "i32")]
    _f: String,
}
#[test]
fn test_reflect_struct_one_basic_field_string_reflect_both_with_attributes() {
    assert_snapshot!(TestStructOneBasicFieldStringReflectBothDifferently);
}

#[derive(reflect::Input, reflect::Output, serde::Deserialize, serde::Serialize)]
struct TestStructOneBasicFieldStringReflectBothEqually {
    #[reflect(output_type = "u32", input_type = "u32")]
    _f: String,
}
#[test]
fn test_reflect_struct_one_basic_field_string_reflect_both_equally() {
    assert_snapshot!(TestStructOneBasicFieldStringReflectBothEqually);
}

#[derive(reflect::Input, reflect::Output, serde::Deserialize, serde::Serialize)]
struct TestStructOneBasicFieldStringReflectBothEqually2 {
    #[reflect(type = "u32")]
    _f: String,
}
#[test]
fn test_reflect_struct_one_basic_field_string_reflect_both_equally2() {
    assert_input_snapshot!(TestStructOneBasicFieldStringReflectBothEqually);
}

#[derive(reflect::Input, reflect::Output, serde::Deserialize, serde::Serialize)]
struct TestStructWithNested {
    _f: TestStructNested,
}
#[derive(reflect::Input, reflect::Output, serde::Deserialize, serde::Serialize)]
struct TestStructNested {
    _f: String,
}
#[test]
fn test_reflect_struct_with_nested() {
    assert_snapshot!(TestStructWithNested);
}

#[derive(reflect::Input, reflect::Output, serde::Deserialize, serde::Serialize)]
struct TestStructWithNestedExternal {
    _f: super::test_lib::TestStructNested,
}
#[test]
fn test_reflect_struct_with_nested_external() {
    assert_snapshot!(TestStructWithNestedExternal);
}

#[derive(reflect::Input, reflect::Output, serde::Deserialize, serde::Serialize)]
struct TestStructWithVec {
    _f: Vec<u8>,
}
#[test]
fn test_reflect_struct_with_vec() {
    assert_snapshot!(TestStructWithVec);
}

#[derive(reflect::Input, reflect::Output, serde::Deserialize, serde::Serialize)]
struct TestStructWithVecTwo {
    _f: Vec<u8>,
    _f2: Vec<i8>,
}
#[test]
fn test_reflect_struct_with_vec_two() {
    assert_snapshot!(TestStructWithVecTwo);
}

#[derive(reflect::Input, reflect::Output, serde::Deserialize, serde::Serialize)]
struct TestStructWithVecExternal {
    _f: Vec<super::test_lib::TestStructNested>,
}
#[test]
fn test_reflect_struct_with_vec_external() {
    assert_snapshot!(TestStructWithVecExternal);
}

#[derive(reflect::Input, reflect::Output, serde::Deserialize, serde::Serialize)]
struct TestStructWithVecNested {
    _f: Vec<Vec<super::test_lib::TestStructNested>>,
}
#[test]
fn test_reflect_struct_with_vec_nested() {
    assert_snapshot!(TestStructWithVecNested);
}

#[derive(reflect::Input, reflect::Output, serde::Deserialize, serde::Serialize)]
struct TestStructWithHashMap {
    _f: HashMap<u8, String>,
}
#[test]
fn test_reflect_struct_with_hashmap() {
    assert_snapshot!(TestStructWithHashMap);
}

#[derive(reflect::Input, reflect::Output, serde::Deserialize, serde::Serialize)]
struct TestStructEmpty {}
#[test]
fn test_reflect_struct_empty() {
    assert_snapshot!(TestStructEmpty);
}

#[derive(reflect::Input, reflect::Output, serde::Deserialize, serde::Serialize)]
struct TestStructOption {
    _f: Option<u8>,
}
#[test]
fn test_reflect_struct_option() {
    assert_snapshot!(TestStructOption);
}

#[derive(reflect::Input, reflect::Output, serde::Deserialize, serde::Serialize)]
struct TestStructNewtype(String);
#[test]
fn test_reflect_struct_newtype() {
    assert_snapshot!(TestStructNewtype);
}

#[derive(reflect::Input, reflect::Output, serde::Deserialize, serde::Serialize)]
struct TestStructTuple(u8, String);
#[test]
fn test_reflect_struct_tuple() {
    assert_snapshot!(TestStructTuple);
}

#[derive(reflect::Input, reflect::Output, serde::Deserialize, serde::Serialize)]
struct TestStructWithTuple {
    _f: (u8, String),
}
#[test]
fn test_reflect_struct_with_tuple() {
    assert_snapshot!(TestStructWithTuple);
}

#[derive(reflect::Input, reflect::Output, serde::Deserialize, serde::Serialize)]
struct TestStructWithTuple12 {
    _f: (
        u8,
        String,
        u8,
        String,
        u8,
        String,
        u8,
        String,
        u8,
        String,
        u8,
        String,
    ),
}
#[test]
fn test_reflect_struct_with_tuple12() {
    assert_snapshot!(TestStructWithTuple12);
}

#[derive(reflect::Input, reflect::Output, serde::Deserialize, serde::Serialize)]
struct TestStructWithFixedSizeArray {
    _f: [u8; 3],
}
#[test]
fn test_reflect_struct_with_fixed_size_array() {
    assert_snapshot!(TestStructWithFixedSizeArray);
}

#[derive(reflect::Input, reflect::Output, serde::Deserialize, serde::Serialize)]
struct TestStructWithArc {
    _f: std::sync::Arc<u8>,
}
#[test]
fn test_reflect_struct_with_arc() {
    assert_snapshot!(TestStructWithArc);
}

#[derive(reflect::Input, reflect::Output, serde::Deserialize, serde::Serialize)]
struct TestStructWithSelfViaArc {
    _f: std::sync::Arc<Self>,
}
#[test]
fn test_reflect_struct_with_self_via_arc() {
    assert_snapshot!(TestStructWithSelfViaArc);
}

#[derive(reflect::Input, reflect::Output, serde::Deserialize, serde::Serialize)]
struct TestStructWithTransformFallback {
    #[reflect(
        input_transform = "reflect::TypeReference::fallback_recursively",
        output_transform = "reflect::TypeReference::fallback_recursively"
    )]
    _f: std::sync::Arc<u8>,
}
#[test]
fn test_reflect_struct_with_transform_fallback() {
    assert_snapshot!(TestStructWithTransformFallback);
}

#[derive(reflect::Input, reflect::Output, serde::Deserialize, serde::Serialize)]
struct TestStructWithTransformBoth {
    #[reflect(transform = "reflect::TypeReference::fallback_recursively")]
    _f: std::sync::Arc<u8>,
}
#[test]
fn test_reflect_struct_with_transform_both() {
    assert_snapshot!(TestStructWithTransformBoth);
}

#[derive(reflect::Input, reflect::Output, serde::Deserialize, serde::Serialize)]
struct TestStructWithTransformInput {
    #[reflect(input_transform = "reflect::TypeReference::fallback_recursively")]
    _f: std::sync::Arc<u8>,
}
#[test]
fn test_reflect_struct_with_transform_input() {
    assert_snapshot!(TestStructWithTransformInput);
}

#[derive(reflect::Input, reflect::Output, serde::Deserialize, serde::Serialize)]
struct TestStructWithTransformOutput {
    #[reflect(output_transform = "reflect::TypeReference::fallback_recursively")]
    _f: std::sync::Arc<u8>,
}
#[test]
fn test_reflect_struct_with_transform_output() {
    assert_snapshot!(TestStructWithTransformOutput);
}

#[derive(reflect::Input, reflect::Output, serde::Deserialize, serde::Serialize)]
struct TestStructWithTransformFallbackNested {
    #[reflect(
        input_transform = "reflect::TypeReference::fallback_recursively",
        output_transform = "reflect::TypeReference::fallback_recursively"
    )]
    _f: std::sync::Arc<std::sync::Arc<u8>>,
}
#[test]
fn test_reflect_struct_with_transform_fallback_nested() {
    assert_snapshot!(TestStructWithTransformFallbackNested);
}

#[derive(reflect::Input, reflect::Output, serde::Deserialize, serde::Serialize)]
struct TestStructWithTransformArray {
    #[reflect(transform = "reflect::TypeReference::fallback_recursively")]
    _f: [u8; 8],
}
#[test]
fn test_reflect_struct_with_transform_array() {
    assert_snapshot!(TestStructWithTransformArray);
}

/// Some Struct docs
/// more
/// more
#[allow(unused_doc_comments, dead_code)]
#[derive(reflect::Input, serde::Deserialize)]
struct TestStructDocumented {
    /// field docs
    /// multiline
    f: u8,
}
#[test]
fn test_reflect_struct_documented() {
    assert_input_snapshot!(TestStructDocumented);
}

/// Some Enum docs
/// more
#[allow(unused_doc_comments, dead_code)]
#[derive(reflect::Input, serde::Deserialize)]
enum TestEnumDocumented<
    /// some generic param docs
    /// multiline
    T,
> where
    T: reflect::Input,
{
    /// Variant1 docs
    Variant1(
        /// variant1 field docs
        T,
    ),
    /// Variant2 docs
    /// multiline
    /// more
    /// more
    Variant2 {
        /// named field variant2 field docs
        named_field: T,
    },
}
#[test]
fn test_reflect_enum_documented() {
    assert_input_snapshot!(TestEnumDocumented::<u8>);
}
