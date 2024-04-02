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
#[reflect(input_type = "u8", output_type = "u8")]
struct TestStructWithAttributes {
    _f: String,
}
#[test]
fn test_reflect_struct_with_attributes() {
    assert_snapshot!(TestStructWithAttributes);
}

#[derive(reflect::Input, reflect::Output, serde::Deserialize, serde::Serialize)]
#[reflect(input_type = "String")]
struct TestStructWithAttributesInputOnly {
    _f: String,
}
#[test]
fn test_reflect_struct_with_attributes_input_only() {
    assert_snapshot!(TestStructWithAttributesInputOnly);
}

#[derive(reflect::Input, reflect::Output, serde::Deserialize, serde::Serialize)]
#[reflect(output_type = "String")]
struct TestStructWithAttributesOutputOnly {
    _f: String,
}
#[test]
fn test_reflect_struct_with_attributes_output_only() {
    assert_snapshot!(TestStructWithAttributesOutputOnly);
}

#[derive(reflect::Input, reflect::Output, serde::Deserialize, serde::Serialize)]
#[reflect(type = "String")]
struct TestStructWithAttributesTypeOnly {
    _f: String,
}
#[test]
fn test_reflect_struct_with_attributes_type_only() {
    assert_snapshot!(TestStructWithAttributesTypeOnly);
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
    #[allow(unused_doc_comments, dead_code)]
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

#[derive(reflect::Input, reflect::Output, serde::Deserialize, serde::Serialize)]
struct TestStructWithAllPrimitiveTypeFields {
    _f_u8: u8,
    _f_u16: u16,
    _f_u32: u32,
    _f_u64: u64,
    _f_u128: u128,
    _f_usize: usize,
    _f_i8: i8,
    _f_i16: i16,
    _f_i32: i32,
    _f_i64: i64,
    _f_i128: i128,
    _f_isize: isize,
    _f_f32: f32,
    _f_f64: f64,
    _f_bool: bool,
    _f_char: char,
    _f_str: String,
    _f_unit: (),
    _f_option: Option<u8>,
    _f_vec: Vec<u8>,
    _f_hashmap: HashMap<u8, String>,
    _f_hashset: std::collections::HashSet<u8>,
    _f_tuple: (u8, String),
    _f_tuple3: (u8, String, u8),
    _f_tuple4: (u8, String, u8, String),
    _f_tuple5: (u8, String, u8, String, u8),
    _f_tuple6: (u8, String, u8, String, u8, String),
    _f_tuple7: (u8, String, u8, String, u8, String, u8),
    _f_tuple8: (u8, String, u8, String, u8, String, u8, String),
    _f_tuple9: (u8, String, u8, String, u8, String, u8, String, u8),
    _f_tuple10: (u8, String, u8, String, u8, String, u8, String, u8, String),
    _f_tuple11: (
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
    ),
    _f_tuple12: (
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
    _f_array: [u8; 3],
    _f_pointer_box: Box<u8>,
    // _f_pointer_rc: std::rc::Rc<u8>, // this does not implement Sync / Send
    _f_pointer_arc: std::sync::Arc<u8>,
    _f_pointer_cell: std::cell::Cell<u8>,
    _f_pointer_refcell: std::cell::RefCell<u8>,
    _f_pointer_mutex: std::sync::Mutex<u8>,
    _f_pointer_rwlock: std::sync::RwLock<u8>,
    _f_pointer_weak: std::sync::Weak<u8>,
    _f_phantomdata: std::marker::PhantomData<u8>,
    _f_infallible: reflect::Infallible,
}
#[test]
fn test_reflect_struct_with_all_primitive_type_fields() {
    assert_snapshot!(TestStructWithAllPrimitiveTypeFields);
}

#[derive(reflect::Input, reflect::Output, serde::Deserialize, serde::Serialize)]
struct TestStructWithArcPointerOnly {
    _f_pointer_arc: std::sync::Arc<u8>,
}
#[test]
fn test_reflect_struct_with_arc_pointer_only() {
    assert_snapshot!(TestStructWithArcPointerOnly);
}

#[derive(reflect::Input, reflect::Output, serde::Deserialize, serde::Serialize)]
struct TestStructWithHashSetField {
    _f_hashset: std::collections::HashSet<u8>,
}
#[test]
fn test_reflect_struct_with_hashset_field() {
    assert_snapshot!(TestStructWithHashSetField);
}

#[derive(reflect::Input, reflect::Output, serde::Deserialize, serde::Serialize)]
struct TestStructWithHashSetFieldGeneric<G>
where
    G: std::hash::Hash + Eq + reflect::Input + reflect::Output,
{
    _f_hashset: std::collections::HashSet<G>,
}
#[test]
fn test_reflect_struct_with_hashset_field_generic() {
    assert_snapshot!(TestStructWithHashSetFieldGeneric::<String>);
}

#[derive(reflect::Input, reflect::Output, serde::Deserialize, serde::Serialize)]
struct TestStructUnitType;
#[test]
fn test_reflect_struct_unit_type() {
    assert_snapshot!(TestStructUnitType);
}
