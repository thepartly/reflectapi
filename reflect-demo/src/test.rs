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

#[derive(reflect::Input)]
struct TestStructOneBasicFieldU32 {
    _f: u32,
}
#[test]
fn test_reflect_struct_one_basic_field_u32() {
    insta::assert_json_snapshot!(TestStructOneBasicFieldU32::reflect_input());
}

#[derive(reflect::Input)]
struct TestStructOneBasicFieldString {
    _f: String,
}
#[test]
fn test_reflect_struct_one_basic_field_string() {
    insta::assert_json_snapshot!(TestStructOneBasicFieldString::reflect_input());
}

#[derive(reflect::Input, reflect::Output)]
struct TestStructOneBasicFieldStringReflectBoth {
    _f: String,
}
#[test]
fn test_reflect_struct_one_basic_field_string_reflect_both_input() {
    insta::assert_json_snapshot!(TestStructOneBasicFieldStringReflectBoth::reflect_input());
}
#[test]
fn test_reflect_struct_one_basic_field_string_reflect_both_output() {
    insta::assert_json_snapshot!(TestStructOneBasicFieldStringReflectBoth::reflect_output());
}

#[derive(reflect::Input, reflect::Output)]
struct TestStructOneBasicFieldStringReflectBothDifferently {
    #[reflect(output_type = "u32", input_type = "i32")]
    _f: String,
}
#[test]
fn test_reflect_struct_one_basic_field_string_reflect_both_with_attributes_input() {
    insta::assert_json_snapshot!(
        TestStructOneBasicFieldStringReflectBothDifferently::reflect_input()
    );
}
#[test]
fn test_reflect_struct_one_basic_field_string_reflect_both_with_attributes_output() {
    insta::assert_json_snapshot!(
        TestStructOneBasicFieldStringReflectBothDifferently::reflect_output()
    );
}

#[derive(reflect::Input, reflect::Output)]
struct TestStructOneBasicFieldStringReflectBothEqually {
    #[reflect(output_type = "u32", input_type = "u32")]
    _f: String,
}
#[test]
fn test_reflect_struct_one_basic_field_string_reflect_both_equally_input() {
    insta::allow_duplicates! {
        insta::assert_json_snapshot!(TestStructOneBasicFieldStringReflectBothEqually::reflect_input());
    }
}
#[test]
fn test_reflect_struct_one_basic_field_string_reflect_both_equally_output() {
    insta::allow_duplicates! {
        insta::assert_json_snapshot!(TestStructOneBasicFieldStringReflectBothEqually::reflect_output());
    }
}

#[derive(reflect::Input, reflect::Output)]
struct TestStructOneBasicFieldStringReflectBothEqually2 {
    #[reflect(type = "u32")]
    _f: String,
}
#[test]
fn test_reflect_struct_one_basic_field_string_reflect_both_equally2_input() {
    insta::assert_json_snapshot!(TestStructOneBasicFieldStringReflectBothEqually::reflect_input());
}
#[test]
fn test_reflect_struct_one_basic_field_string_reflect_both_equally2_output() {
    insta::assert_json_snapshot!(
        TestStructOneBasicFieldStringReflectBothEqually2::reflect_output()
    );
}

#[derive(reflect::Input, reflect::Output)]
struct TestStructWithNested {
    _f: TestStructNested,
}
#[derive(reflect::Input, reflect::Output)]
struct TestStructNested {
    _f: String,
}
#[test]
fn test_reflect_struct_with_nested_input() {
    insta::assert_json_snapshot!(TestStructWithNested::reflect_input());
}
#[test]
fn test_reflect_struct_with_nested_output() {
    insta::assert_json_snapshot!(TestStructWithNested::reflect_output());
}

#[derive(reflect::Input, reflect::Output)]
struct TestStructWithNestedExternal {
    _f: crate::test_lib::TestStructNested,
}
#[test]
fn test_reflect_struct_with_nested_external_input() {
    insta::assert_json_snapshot!(TestStructWithNestedExternal::reflect_input());
}
#[test]
fn test_reflect_struct_with_nested_external_output() {
    insta::assert_json_snapshot!(TestStructWithNestedExternal::reflect_output());
}

#[derive(reflect::Input, reflect::Output)]
struct TestStructWithVec {
    _f: Vec<u8>,
}
#[test]
fn test_reflect_struct_with_vec_input() {
    insta::assert_json_snapshot!(TestStructWithVec::reflect_input());
}
#[test]
fn test_reflect_struct_with_vec_output() {
    insta::assert_json_snapshot!(TestStructWithVec::reflect_output());
}

#[derive(reflect::Input, reflect::Output)]
struct TestStructWithVecTwo {
    _f: Vec<u8>,
    _f2: Vec<i8>,
}
#[test]
fn test_reflect_struct_with_vec_two_input() {
    insta::assert_json_snapshot!(TestStructWithVecTwo::reflect_input());
}
#[test]
fn test_reflect_struct_with_vec_two_output() {
    insta::assert_json_snapshot!(TestStructWithVecTwo::reflect_output());
}

#[derive(reflect::Input, reflect::Output)]
struct TestStructWithVecExternal {
    _f: Vec<crate::test_lib::TestStructNested>,
}
#[test]
fn test_reflect_struct_with_vec_external_input() {
    insta::assert_json_snapshot!(TestStructWithVecExternal::reflect_input());
}
#[test]
fn test_reflect_struct_with_vec_external_output() {
    insta::assert_json_snapshot!(TestStructWithVecExternal::reflect_output());
}

#[derive(reflect::Input, reflect::Output)]
struct TestStructWithVecNested {
    _f: Vec<Vec<crate::test_lib::TestStructNested>>,
}
#[test]
fn test_reflect_struct_with_vec_nested_input() {
    insta::assert_json_snapshot!(TestStructWithVecNested::reflect_input());
}
#[test]
fn test_reflect_struct_with_vec_nested_output() {
    insta::assert_json_snapshot!(TestStructWithVecNested::reflect_output());
}

#[derive(reflect::Input, reflect::Output)]
struct TestStructWithHashMap {
    _f: HashMap<u8, String>,
}
#[test]
fn test_reflect_struct_with_hashmap_input() {
    insta::assert_json_snapshot!(TestStructWithHashMap::reflect_input());
}
#[test]
fn test_reflect_struct_with_hashmap_output() {
    insta::assert_json_snapshot!(TestStructWithHashMap::reflect_output());
}

#[derive(reflect::Input, reflect::Output)]
struct TestStructEmpty {}
#[test]
fn test_reflect_struct_empty_input() {
    insta::assert_json_snapshot!(TestStructEmpty::reflect_input());
}
#[test]
fn test_reflect_struct_empty_output() {
    insta::assert_json_snapshot!(TestStructEmpty::reflect_output());
}

#[derive(reflect::Input, reflect::Output)]
struct TestStructOption {
    _f: Option<u8>,
}
#[test]
fn test_reflect_struct_option_input() {
    insta::assert_json_snapshot!(TestStructOption::reflect_input());
}
#[test]
fn test_reflect_struct_option_output() {
    insta::assert_json_snapshot!(TestStructOption::reflect_output());
}

#[derive(reflect::Input, reflect::Output)]
struct TestStructNewtype(String);
#[test]
fn test_reflect_struct_newtype_input() {
    insta::assert_json_snapshot!(TestStructNewtype::reflect_input());
}
#[test]
fn test_reflect_struct_newtype_output() {
    insta::assert_json_snapshot!(TestStructNewtype::reflect_output());
}

#[derive(reflect::Input, reflect::Output)]
struct TestStructTuple(u8, String);
#[test]
fn test_reflect_struct_tuple_input() {
    insta::assert_json_snapshot!(TestStructTuple::reflect_input());
}
#[test]
fn test_reflect_struct_tuple_output() {
    insta::assert_json_snapshot!(TestStructTuple::reflect_output());
}

#[derive(reflect::Input, reflect::Output)]
struct TestStructWithTuple {
    _f: (u8, String),
}
#[test]
fn test_reflect_struct_with_tuple_input() {
    insta::assert_json_snapshot!(TestStructWithTuple::reflect_input());
}
#[test]
fn test_reflect_struct_with_tuple_output() {
    insta::assert_json_snapshot!(TestStructWithTuple::reflect_output());
}

#[derive(reflect::Input, reflect::Output)]
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
fn test_reflect_struct_with_tuple12_input() {
    insta::assert_json_snapshot!(TestStructWithTuple12::reflect_input());
}
#[test]
fn test_reflect_struct_with_tuple12_output() {
    insta::assert_json_snapshot!(TestStructWithTuple12::reflect_output());
}

#[derive(reflect::Input, reflect::Output)]
struct TestStructWithFixedSizeArray {
    _f: [u8; 3],
}
#[test]
fn test_reflect_struct_with_fixed_size_array_input() {
    insta::assert_json_snapshot!(TestStructWithFixedSizeArray::reflect_input());
}
#[test]
fn test_reflect_struct_with_fixed_size_array_output() {
    insta::assert_json_snapshot!(TestStructWithFixedSizeArray::reflect_output());
}

#[derive(reflect::Input, reflect::Output)]
struct TestStructWithArc {
    _f: std::sync::Arc<u8>,
}
#[test]
fn test_reflect_struct_with_arc_input() {
    insta::assert_json_snapshot!(TestStructWithArc::reflect_input());
}
#[test]
fn test_reflect_struct_with_arc_output() {
    insta::assert_json_snapshot!(TestStructWithArc::reflect_output());
}

#[derive(reflect::Input, reflect::Output)]
struct TestStructWithSelfViaArc {
    _f: std::sync::Arc<Self>,
}
#[test]
fn test_reflect_struct_with_self_via_arc_input() {
    insta::assert_json_snapshot!(TestStructWithSelfViaArc::reflect_input());
}
#[test]
fn test_reflect_struct_with_self_via_arc_output() {
    insta::assert_json_snapshot!(TestStructWithSelfViaArc::reflect_output());
}

#[derive(reflect::Input, reflect::Output)]
struct TestStructWithTransformFallback {
    #[reflect(
        input_transform = "reflect::TypeReference::fallback_recursively",
        output_transform = "reflect::TypeReference::fallback_recursively"
    )]
    _f: std::sync::Arc<u8>,
}
#[test]
fn test_reflect_struct_with_transform_fallback_input() {
    insta::assert_json_snapshot!(TestStructWithTransformFallback::reflect_input());
}
#[test]
fn test_reflect_struct_with_transform_fallback_output() {
    insta::assert_json_snapshot!(TestStructWithTransformFallback::reflect_output());
}

#[derive(reflect::Input, reflect::Output)]
struct TestStructWithTransformBoth {
    #[reflect(transform = "reflect::TypeReference::fallback_recursively")]
    _f: std::sync::Arc<u8>,
}
#[test]
fn test_reflect_struct_with_transform_both_input() {
    insta::assert_json_snapshot!(TestStructWithTransformBoth::reflect_input());
}
#[test]
fn test_reflect_struct_with_transform_both_output() {
    insta::assert_json_snapshot!(TestStructWithTransformBoth::reflect_output());
}

#[derive(reflect::Input, reflect::Output)]
struct TestStructWithTransformInput {
    #[reflect(input_transform = "reflect::TypeReference::fallback_recursively")]
    _f: std::sync::Arc<u8>,
}
#[test]
fn test_reflect_struct_with_transform_input_input() {
    insta::assert_json_snapshot!(TestStructWithTransformInput::reflect_input());
}
#[test]
fn test_reflect_struct_with_transform_input_output() {
    insta::assert_json_snapshot!(TestStructWithTransformInput::reflect_output());
}

#[derive(reflect::Input, reflect::Output)]
struct TestStructWithTransformOutput {
    #[reflect(output_transform = "reflect::TypeReference::fallback_recursively")]
    _f: std::sync::Arc<u8>,
}
#[test]
fn test_reflect_struct_with_transform_output_input() {
    insta::assert_json_snapshot!(TestStructWithTransformOutput::reflect_input());
}
#[test]
fn test_reflect_struct_with_transform_output_output() {
    insta::assert_json_snapshot!(TestStructWithTransformOutput::reflect_output());
}

#[derive(reflect::Input, reflect::Output)]
struct TestStructWithTransformFallbackNested {
    #[reflect(
        input_transform = "reflect::TypeReference::fallback_recursively",
        output_transform = "reflect::TypeReference::fallback_recursively"
    )]
    _f: std::sync::Arc<std::sync::Arc<u8>>,
}
#[test]
fn test_reflect_struct_with_transform_fallback_nested_input() {
    insta::assert_json_snapshot!(TestStructWithTransformFallbackNested::reflect_input());
}
#[test]
fn test_reflect_struct_with_transform_fallback_nested_output() {
    insta::assert_json_snapshot!(TestStructWithTransformFallbackNested::reflect_output());
}

#[derive(reflect::Input, reflect::Output)]
struct TestStructWithTransformArray {
    #[reflect(transform = "reflect::TypeReference::fallback_recursively")]
    _f: [u8; 8],
}
#[test]
fn test_reflect_struct_with_transform_array_input() {
    insta::assert_json_snapshot!(TestStructWithTransformArray::reflect_input());
}
#[test]
fn test_reflect_struct_with_transform_array_output() {
    insta::assert_json_snapshot!(TestStructWithTransformArray::reflect_output());
}
