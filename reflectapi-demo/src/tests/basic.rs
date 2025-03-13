use std::collections::{BTreeSet, HashMap, HashSet};

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

#[derive(reflectapi::Input, serde::Deserialize)]
struct TestStructOneBasicFieldU32 {
    _f: u32,
}
#[test]
fn test_reflectapi_struct_one_basic_field_u32() {
    assert_input_snapshot!(TestStructOneBasicFieldU32);
}

#[derive(reflectapi::Input, serde::Deserialize)]
struct TestStructOneBasicFieldString {
    _f: String,
}
#[test]
fn test_reflectapi_struct_one_basic_field_string() {
    assert_input_snapshot!(TestStructOneBasicFieldString);
}

#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
struct TestStructOneBasicFieldStringReflectBoth {
    _f: String,
}
#[test]
fn test_reflectapi_struct_one_basic_field_string_reflectapi_both() {
    assert_snapshot!(TestStructOneBasicFieldStringReflectBoth);
}

#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
struct TestStructOneBasicFieldStringReflectBothDifferently {
    #[reflectapi(output_type = "u32", input_type = "i32")]
    _f: String,
}
#[test]
fn test_reflectapi_struct_one_basic_field_string_reflectapi_both_with_attributes() {
    assert_snapshot!(TestStructOneBasicFieldStringReflectBothDifferently);
}

#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
struct TestStructOneBasicFieldStringReflectBothEqually {
    #[reflectapi(output_type = "u32", input_type = "u32")]
    _f: String,
}
#[test]
fn test_reflectapi_struct_one_basic_field_string_reflectapi_both_equally() {
    assert_snapshot!(TestStructOneBasicFieldStringReflectBothEqually);
}

#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
struct TestStructOneBasicFieldStringReflectBothEqually2 {
    #[reflectapi(type = "u32")]
    _f: String,
}
#[test]
fn test_reflectapi_struct_one_basic_field_string_reflectapi_both_equally2() {
    assert_input_snapshot!(TestStructOneBasicFieldStringReflectBothEqually);
}

#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
struct TestStructWithNested {
    _f: TestStructNested,
}
#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
struct TestStructNested {
    _f: String,
}
#[test]
fn test_reflectapi_struct_with_nested() {
    assert_snapshot!(TestStructWithNested);
}

#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
struct TestStructWithNestedExternal {
    _f: super::test_lib::TestStructNested,
}
#[test]
fn test_reflectapi_struct_with_nested_external() {
    assert_snapshot!(TestStructWithNestedExternal);
}

#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
struct TestStructWithVec {
    _f: Vec<u8>,
}
#[test]
fn test_reflectapi_struct_with_vec() {
    assert_snapshot!(TestStructWithVec);
}

#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
struct TestStructWithVecTwo {
    _f: Vec<u8>,
    _f2: Vec<i8>,
}
#[test]
fn test_reflectapi_struct_with_vec_two() {
    assert_snapshot!(TestStructWithVecTwo);
}

#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
struct TestStructWithVecExternal {
    _f: Vec<super::test_lib::TestStructNested>,
}
#[test]
fn test_reflectapi_struct_with_vec_external() {
    assert_snapshot!(TestStructWithVecExternal);
}

#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
struct TestStructWithVecNested {
    _f: Vec<Vec<super::test_lib::TestStructNested>>,
}
#[test]
fn test_reflectapi_struct_with_vec_nested() {
    assert_snapshot!(TestStructWithVecNested);
}

#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
struct TestStructWithHashMap {
    _f: HashMap<u8, String>,
}
#[test]
fn test_reflectapi_struct_with_hashmap() {
    assert_snapshot!(TestStructWithHashMap);
}

#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
struct TestStructEmpty {}
#[test]
fn test_reflectapi_struct_empty() {
    assert_snapshot!(TestStructEmpty);
}

#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
struct TestStructOption {
    _f: Option<u8>,
}
#[test]
fn test_reflectapi_struct_option() {
    assert_snapshot!(TestStructOption);
}

#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
struct TestStructNewtype(String);
#[test]
fn test_reflectapi_struct_newtype() {
    assert_snapshot!(TestStructNewtype);
}

#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
struct TestStructTuple(u8, String);
#[test]
fn test_reflectapi_struct_tuple() {
    assert_snapshot!(TestStructTuple);
}

#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
struct TestStructWithTuple {
    _f: (u8, String),
}
#[test]
fn test_reflectapi_struct_with_tuple() {
    assert_snapshot!(TestStructWithTuple);
}

#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
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
fn test_reflectapi_struct_with_tuple12() {
    assert_snapshot!(TestStructWithTuple12);
}

#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
struct TestStructWithFixedSizeArray {
    _f: [u8; 3],
}
#[test]
fn test_reflectapi_struct_with_fixed_size_array() {
    assert_snapshot!(TestStructWithFixedSizeArray);
}

#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
struct TestStructWithArc {
    _f: std::sync::Arc<u8>,
}
#[test]
fn test_reflectapi_struct_with_arc() {
    assert_snapshot!(TestStructWithArc);
}

#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
struct TestStructWithSelfViaArc {
    _f: std::sync::Arc<Self>,
}
#[test]
fn test_reflectapi_struct_with_self_via_arc() {
    assert_snapshot!(TestStructWithSelfViaArc);
}

#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
#[reflectapi(input_type = "u8", output_type = "u8")]
struct TestStructWithAttributes {
    _f: String,
}
#[test]
fn test_reflectapi_struct_with_attributes() {
    assert_snapshot!(TestStructWithAttributes);
}

#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
#[reflectapi(input_type = "String")]
struct TestStructWithAttributesInputOnly {
    _f: String,
}
#[test]
fn test_reflectapi_struct_with_attributes_input_only() {
    assert_snapshot!(TestStructWithAttributesInputOnly);
}

#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
#[reflectapi(output_type = "String")]
struct TestStructWithAttributesOutputOnly {
    _f: String,
}
#[test]
fn test_reflectapi_struct_with_attributes_output_only() {
    assert_snapshot!(TestStructWithAttributesOutputOnly);
}

#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
#[reflectapi(type = "String")]
struct TestStructWithAttributesTypeOnly {
    _f: String,
}
#[test]
fn test_reflectapi_struct_with_attributes_type_only() {
    assert_snapshot!(TestStructWithAttributesTypeOnly);
}

#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
struct TestStructWithTransformFallback {
    #[reflectapi(
        input_transform = "reflectapi::TypeReference::fallback_recursively",
        output_transform = "reflectapi::TypeReference::fallback_recursively"
    )]
    _f: std::sync::Arc<u8>,
}
#[test]
fn test_reflectapi_struct_with_transform_fallback() {
    assert_snapshot!(TestStructWithTransformFallback);
}

#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
struct TestStructWithTransformBoth {
    #[reflectapi(transform = "reflectapi::TypeReference::fallback_recursively")]
    _f: std::sync::Arc<u8>,
}
#[test]
fn test_reflectapi_struct_with_transform_both() {
    assert_snapshot!(TestStructWithTransformBoth);
}

#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
struct TestStructWithTransformInput {
    #[reflectapi(input_transform = "reflectapi::TypeReference::fallback_recursively")]
    _f: std::sync::Arc<u8>,
}
#[test]
fn test_reflectapi_struct_with_transform_input() {
    assert_snapshot!(TestStructWithTransformInput);
}

#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
struct TestStructWithTransformOutput {
    #[reflectapi(output_transform = "reflectapi::TypeReference::fallback_recursively")]
    _f: std::sync::Arc<u8>,
}
#[test]
fn test_reflectapi_struct_with_transform_output() {
    assert_snapshot!(TestStructWithTransformOutput);
}

#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
struct TestStructWithTransformFallbackNested {
    #[reflectapi(
        input_transform = "reflectapi::TypeReference::fallback_recursively",
        output_transform = "reflectapi::TypeReference::fallback_recursively"
    )]
    #[allow(clippy::redundant_allocation)]
    _f: std::sync::Arc<std::sync::Arc<u8>>,
}
#[test]
fn test_reflectapi_struct_with_transform_fallback_nested() {
    assert_snapshot!(TestStructWithTransformFallbackNested);
}

#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
struct TestStructWithTransformArray {
    #[reflectapi(transform = "reflectapi::TypeReference::fallback_recursively")]
    _f: [u8; 8],
}
#[test]
fn test_reflectapi_struct_with_transform_array() {
    assert_snapshot!(TestStructWithTransformArray);
}

/// Some Struct docs
/// more
/// more
#[allow(unused_doc_comments, dead_code)]
#[derive(reflectapi::Input, serde::Deserialize)]
struct TestStructDocumented {
    /// field docs
    /// multiline
    f: u8,
}
#[test]
fn test_reflectapi_struct_documented() {
    assert_input_snapshot!(TestStructDocumented);
}

/// Some Enum docs
/// more
#[allow(unused_doc_comments, dead_code)]
#[derive(reflectapi::Input, serde::Deserialize)]
enum TestEnumDocumented<
    #[allow(unused_doc_comments, dead_code)]
    /// some generic param docs
    /// multiline
    T,
> where
    T: reflectapi::Input,
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
fn test_reflectapi_enum_documented() {
    assert_input_snapshot!(TestEnumDocumented::<u8>);
}

#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
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
    _f_infallible: reflectapi::Infallible,
}
#[test]
fn test_reflectapi_struct_with_all_primitive_type_fields() {
    assert_snapshot!(TestStructWithAllPrimitiveTypeFields);
}

#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
struct TestStructWithArcPointerOnly {
    _f_pointer_arc: std::sync::Arc<u8>,
}
#[test]
fn test_reflectapi_struct_with_arc_pointer_only() {
    assert_snapshot!(TestStructWithArcPointerOnly);
}

#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
struct TestStructWithHashSetField {
    _f_hashset: std::collections::HashSet<u8>,
}
#[test]
fn test_reflectapi_struct_with_hashset_field() {
    assert_snapshot!(TestStructWithHashSetField);
}

#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
struct TestStructWithHashSetFieldGeneric<G>
where
    G: std::hash::Hash + Eq + reflectapi::Input + reflectapi::Output,
{
    _f_hashset: std::collections::HashSet<G>,
}
#[test]
fn test_reflectapi_struct_with_hashset_field_generic() {
    assert_snapshot!(TestStructWithHashSetFieldGeneric::<String>);
}

#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
struct TestStructUnitType;
#[test]
fn test_reflectapi_struct_unit_type() {
    assert_snapshot!(TestStructUnitType);
}

#[test]
fn test_reflectapi_enum_with_skip_variant() {
    #[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
    enum TestEnumWithSkipVariant {
        A,
        #[reflectapi(skip)]
        B,
        #[reflectapi(output_skip)]
        I,
        #[reflectapi(input_skip)]
        O,
    }
    assert_snapshot!(TestEnumWithSkipVariant);
}

#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
struct TestStructWithSkipField {
    #[reflectapi(skip)]
    _f: u8,
}

#[test]
fn test_reflectapi_struct_with_skip_field() {
    assert_snapshot!(TestStructWithSkipField);
}

#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
struct TestStructWithSkipFieldInput {
    #[reflectapi(input_skip)]
    _f: u8,
}
#[test]
fn test_reflectapi_struct_with_skip_field_input() {
    assert_snapshot!(TestStructWithSkipFieldInput);
}

#[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
struct TestStructWithSkipFieldOutput {
    #[reflectapi(output_skip)]
    _f: u8,
}
#[test]
fn test_reflectapi_struct_with_skip_field_output() {
    assert_snapshot!(TestStructWithSkipFieldOutput);
}

#[test]
fn test_reflectapi_struct_with_additional_derives() {
    #[derive(
        reflectapi::Input,
        reflectapi::Output,
        serde::Deserialize,
        serde::Serialize,
        Clone,
        PartialOrd,
        Ord,
        PartialEq,
        Eq,
        Hash,
    )]
    #[reflectapi(derive(Clone, PartialOrd, Ord, Hash, PartialEq, Eq))]
    struct X {}

    #[derive(
        reflectapi::Input,
        reflectapi::Output,
        serde::Deserialize,
        serde::Serialize,
        Clone,
        PartialOrd,
        Ord,
        PartialEq,
        Eq,
        Hash,
    )]
    #[reflectapi(derive(Clone, PartialOrd, Ord, Hash, PartialEq, Eq))]
    enum Y {
        Y,
    }

    #[derive(
        reflectapi::Input,
        reflectapi::Output,
        serde::Deserialize,
        serde::Serialize,
        Clone,
        PartialEq,
        Eq,
        PartialOrd,
        Ord,
        Hash,
        Default,
    )]
    #[reflectapi(derive(Clone, PartialOrd, Ord, PartialEq, Eq, Hash, Default))]
    struct U;

    #[derive(reflectapi::Input, reflectapi::Output, serde::Deserialize, serde::Serialize)]
    #[reflectapi(derive(Clone))]
    struct Test {
        us: BTreeSet<U>,
        xs: BTreeSet<X>,
        ys: HashSet<Y>,
    }

    assert_snapshot!(Test);
}

#[derive(reflectapi::Output, serde::Serialize)]
struct TestStructOneBasicFieldStaticStr {
    _f: &'static str,
}
#[test]
fn test_reflectapi_struct_one_basic_field_static_str() {
    assert_output_snapshot!(TestStructOneBasicFieldStaticStr);
}

#[test]
fn test_reflectapi_deprecated() {
    #[derive(serde::Serialize, reflectapi::Input, serde::Deserialize, reflectapi::Output)]
    struct StructWithDeprecatedField {
        #[deprecated]
        _f: u8,
        #[deprecated = "g's deprecation note"]
        _g: u8,
        #[deprecated(note = "h's deprecation note")]
        _h: u8,
    }
    assert_snapshot!(StructWithDeprecatedField);
}
