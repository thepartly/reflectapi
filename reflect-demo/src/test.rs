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
