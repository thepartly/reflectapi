#[derive(reflect::Input, reflect::Output, serde::Deserialize, serde::Serialize)]
#[serde(rename = "MyStruct")]
struct TestStructRename {}
#[test]
fn test_struct_rename() {
    insta::assert_json_snapshot!((
        TestStructRename::reflect_input(),
        TestStructRename::reflect_output()
    ));
}

#[derive(reflect::Input, reflect::Output, serde::Deserialize, serde::Serialize)]
#[serde(rename(serialize = "MyStructOutput", deserialize = "MyStructInput"))]
struct TestStructRenameDifferently {}
#[test]
fn test_struct_rename_differently() {
    insta::assert_json_snapshot!((
        TestStructRenameDifferently::reflect_input(),
        TestStructRenameDifferently::reflect_output()
    ));
}

#[derive(reflect::Input, reflect::Output, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct TestStructRenameAll {
    field_name: u8,
}
#[test]
fn test_struct_rename_all() {
    insta::assert_json_snapshot!((
        TestStructRenameAll::reflect_input(),
        TestStructRenameAll::reflect_output()
    ));
}

#[derive(reflect::Input, reflect::Output, serde::Deserialize, serde::Serialize)]
#[serde(rename_all(serialize = "camelCase", deserialize = "snake_case"))]
struct TestStructRenameAllDifferently {
    field_name: u8,
}
#[test]
fn test_struct_rename_all_differently() {
    insta::assert_json_snapshot!((
        TestStructRenameAllDifferently::reflect_input(),
        TestStructRenameAllDifferently::reflect_output()
    ));
}

#[derive(reflect::Input, reflect::Output, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "PascalCase")]
struct TestStructRenameAllPascalCase {
    field_name: u8,
}
#[test]
fn test_struct_rename_all_pascal_case() {
    insta::assert_json_snapshot!((
        TestStructRenameAllPascalCase::reflect_input(),
        TestStructRenameAllPascalCase::reflect_output()
    ));
}

#[derive(reflect::Input, reflect::Output, serde::Deserialize, serde::Serialize)]
struct TestStructRenameField {
    #[serde(rename(serialize = "field_name", deserialize = "fieldName"))]
    f: u8,
}
#[test]
fn test_struct_rename_field() {
    insta::assert_json_snapshot!((
        TestStructRenameField::reflect_input(),
        TestStructRenameField::reflect_output()
    ));
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
    insta::assert_json_snapshot!((
        TestEnumRename::reflect_input(),
        TestEnumRename::reflect_output()
    ));
}

#[derive(reflect::Input, reflect::Output, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
enum TestEnumRenameAll {
    FieldName,
}
#[test]
fn test_enum_rename_all() {
    insta::assert_json_snapshot!((
        TestEnumRenameAll::reflect_input(),
        TestEnumRenameAll::reflect_output()
    ));
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
    insta::assert_json_snapshot!((
        TestEnumRenameVariantField::reflect_input(),
        TestEnumRenameVariantField::reflect_output()
    ));
}
