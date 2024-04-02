#[derive(reflect::Input, reflect::Output, serde::Deserialize, serde::Serialize)]
#[serde(rename = "MyStruct")]
struct TestStructRename {}
#[test]
fn test_struct_rename() {
    assert_snapshot!(TestStructRename);
}

#[derive(reflect::Input, reflect::Output, serde::Deserialize, serde::Serialize)]
#[serde(rename(serialize = "MyStructOutput", deserialize = "MyStructInput"))]
struct TestStructRenameDifferently {}
#[test]
fn test_struct_rename_differently() {
    assert_snapshot!(TestStructRenameDifferently);
}

#[derive(reflect::Input, reflect::Output, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct TestStructRenameAll {
    field_name: u8,
}
#[test]
fn test_struct_rename_all() {
    assert_snapshot!(TestStructRenameAll);
}

#[derive(reflect::Input, reflect::Output, serde::Deserialize, serde::Serialize)]
#[serde(rename_all(serialize = "camelCase", deserialize = "snake_case"))]
struct TestStructRenameAllDifferently {
    field_name: u8,
}
#[test]
fn test_struct_rename_all_differently() {
    assert_snapshot!(TestStructRenameAllDifferently);
}

#[derive(reflect::Input, reflect::Output, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "PascalCase")]
struct TestStructRenameAllPascalCase {
    field_name: u8,
}
#[test]
fn test_struct_rename_all_pascal_case() {
    assert_snapshot!(TestStructRenameAllPascalCase);
}

#[derive(reflect::Input, reflect::Output, serde::Deserialize, serde::Serialize)]
struct TestStructRenameField {
    #[serde(rename(serialize = "field_name", deserialize = "fieldName"))]
    f: u8,
}
#[test]
fn test_struct_rename_field() {
    assert_snapshot!(TestStructRenameField);
}

#[derive(reflect::Input, reflect::Output, serde::Deserialize, serde::Serialize)]
#[serde(rename = "MyEnum")]
enum TestEnumRename {
    #[serde(rename = "V1")]
    Variant1,
    #[serde(rename = "V2")]
    Variant2,
}
#[test]
fn test_enum_rename() {
    assert_snapshot!(TestEnumRename);
}

#[derive(reflect::Input, reflect::Output, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
enum TestEnumRenameAll {
    FieldName,
}
#[test]
fn test_enum_rename_all() {
    assert_snapshot!(TestEnumRenameAll);
}

// test enume rename variant named and unnamed field
#[derive(reflect::Input, reflect::Output, serde::Deserialize, serde::Serialize)]
enum TestEnumRenameVariantField {
    Variant1(#[serde(rename = "variant1_field_name")] u8),
    Variant2 {
        #[serde(rename = "variant2_field_name")]
        field_name: u8,
    },
}
#[test]
fn test_enum_rename_variant_field() {
    assert_snapshot!(TestEnumRenameVariantField);
}

#[derive(reflect::Input, reflect::Output, serde::Deserialize, serde::Serialize)]
#[serde(untagged)]
enum TestEnumUntagged {
    Variant1(u8),
    Variant2 { field_name: u8 },
}
#[test]
fn test_enum_untagged() {
    assert_snapshot!(TestEnumUntagged);
}

#[derive(reflect::Input, reflect::Output, serde::Deserialize, serde::Serialize)]
#[serde(tag = "type")]
enum TestEnumTag {
    Variant1 { field_name: u8 },
    Variant2(u8),
}
#[test]
fn test_enum_tag() {
    assert_snapshot!(TestEnumTag);
}

#[derive(reflect::Input, reflect::Output, serde::Deserialize, serde::Serialize)]
#[serde(tag = "type", content = "content")]
enum TestEnumTagContent {
    Variant1 { field_name: u8 },
    Variant2(u8),
}
#[test]
fn test_enum_tag_content() {
    assert_snapshot!(TestEnumTagContent);
}

#[derive(reflect::Input, reflect::Output, serde::Deserialize, serde::Serialize)]
#[serde(tag = "type", content = "content", rename_all = "camelCase")]
enum TestEnumTagContentRenameAll {
    Variant1 { field_name: u8 },
    Variant2(u8),
}
#[test]
fn test_enum_tag_content_rename_all() {
    assert_snapshot!(TestEnumTagContentRenameAll);
}

#[derive(reflect::Input, reflect::Output, serde::Deserialize, serde::Serialize)]
enum TestEnumRenameAllOnVariant {
    #[serde(rename_all = "camelCase")]
    Variant1 {
        field_name: u8,
    },
    Variant2(u8),
}
#[test]
fn test_enum_rename_all_on_variant() {
    assert_snapshot!(TestEnumRenameAllOnVariant);
}

#[derive(reflect::Input, reflect::Output, serde::Deserialize, serde::Serialize)]
struct TestStructWithSerdeSkipSerializeIf {
    #[serde(skip_serializing_if = "Option::is_none")]
    f: Option<u8>,
}
#[test]
fn test_struct_with_serde_skip_serialize_if() {
    assert_snapshot!(TestStructWithSerdeSkipSerializeIf);
}

#[derive(reflect::Input, reflect::Output, serde::Deserialize, serde::Serialize)]
struct TestStructWithSerdeDefault {
    #[serde(default)]
    f: u8,
}
#[test]
fn test_struct_with_serde_default() {
    assert_snapshot!(TestStructWithSerdeDefault);
}

#[derive(reflect::Input, reflect::Output, serde::Deserialize, serde::Serialize)]
struct TestStructWithSerdeSkip {
    #[serde(skip)]
    _f: u8,
}
#[test]
fn test_struct_with_serde_skip() {
    assert_snapshot!(TestStructWithSerdeSkip);
}

#[derive(reflect::Input, reflect::Output, serde::Deserialize, serde::Serialize)]
struct TestStructWithSerdeSkipSerialize {
    #[serde(skip_serializing)]
    _f: u8,
}
#[test]
fn test_struct_with_serde_skip_serialize() {
    assert_snapshot!(TestStructWithSerdeSkipSerialize);
}

#[derive(reflect::Input, reflect::Output, serde::Deserialize, serde::Serialize)]
struct TestStructWithSerdeSkipDeserialize {
    #[serde(skip_deserializing)]
    f: u8,
}
#[test]
fn test_struct_with_serde_skip_deserialize() {
    assert_snapshot!(TestStructWithSerdeSkipDeserialize);
}

#[derive(reflect::Input, reflect::Output, serde::Deserialize, serde::Serialize)]
enum TestEnumWithFieldSkip {
    Variant1 {
        #[serde(skip)]
        _f: u8,
    },
}
#[test]
fn test_enum_with_field_skip() {
    assert_snapshot!(TestEnumWithFieldSkip);
}

#[derive(reflect::Input, reflect::Output, serde::Deserialize, serde::Serialize)]
enum TestEnumWithVariantSkip {
    #[serde(skip)]
    _Variant1,
}
#[test]
fn test_enum_with_variant_skip() {
    assert_snapshot!(TestEnumWithVariantSkip);
}

#[derive(reflect::Input, reflect::Output, serde::Deserialize, serde::Serialize)]
enum TestEnumWithVariantSkipSerialize {
    #[serde(skip_serializing)]
    Variant1,
}
#[test]
fn test_enum_with_variant_skip_serialize() {
    assert_snapshot!(TestEnumWithVariantSkipSerialize);
}

#[derive(reflect::Input, reflect::Output, serde::Deserialize, serde::Serialize)]
enum TestEnumWithVariantSkipDeserialize {
    #[serde(skip_deserializing)]
    _Variant1,
}
#[test]
fn test_enum_with_variant_skip_deserialize() {
    assert_snapshot!(TestEnumWithVariantSkipDeserialize);
}

#[derive(reflect::Input, reflect::Output, serde::Deserialize, serde::Serialize)]
#[serde(transparent)]
struct TestStructWithSerdeTransparent {
    _f: u8,
}
#[test]
fn test_struct_with_serde_transparent() {
    assert_snapshot!(TestStructWithSerdeTransparent);
}

#[derive(reflect::Input, reflect::Output, serde::Deserialize, serde::Serialize)]
enum TestEnumWithVariantUntagged {
    #[serde(untagged)]
    Variant1(u8),
}
#[test]
fn test_enum_with_variant_untagged() {
    assert_snapshot!(TestEnumWithVariantUntagged);
}

#[derive(reflect::Input, reflect::Output, serde::Deserialize, serde::Serialize)]
#[serde(tag = "type")]
enum TestEnumWithVariantOther {
    V0,
    #[serde(other)]
    Variant1,
}
#[test]
fn test_enum_with_variant_other() {
    assert_snapshot!(TestEnumWithVariantOther);
}

#[derive(reflect::Input, reflect::Output, serde::Deserialize, serde::Serialize)]
struct TestStructWithFlattenNested {
    f: u8,
}
#[derive(reflect::Input, reflect::Output, serde::Deserialize, serde::Serialize)]
struct TestStructWithFlatten {
    #[serde(flatten)]
    g: TestStructWithFlattenNested,
}
#[test]
fn test_struct_with_flatten() {
    assert_snapshot!(TestStructWithFlatten);
}

#[derive(reflect::Input, reflect::Output, serde::Deserialize, serde::Serialize)]
struct TestStructWithFlattenOptional {
    #[serde(flatten, skip_serializing_if = "Option::is_none", default)]
    g: Option<TestStructWithFlattenNested>,
}
#[test]
fn test_struct_with_flatten_optional() {
    assert_snapshot!(TestStructWithFlattenOptional);
}

#[derive(reflect::Input, reflect::Output, serde::Deserialize, serde::Serialize)]
struct TestStructWithFlattenOptionalAndRequired {
    #[serde(flatten, skip_serializing_if = "Option::is_none", default)]
    g: Option<TestStructWithFlattenNested>,
    #[serde(flatten)]
    k: TestStructRenameAll
}
#[test]
fn test_struct_with_flatten_optional_and_required() {
    assert_snapshot!(TestStructWithFlattenOptionalAndRequired);
}
